//! `explain_lines` — the v1.0 Task 11 parity coverage beyond the
//! case line itself: the Epi Info command of every dataset case and
//! one explain trace per `validation::REGISTRY` statistic.
//! `corpus-runner` prints both as extra canonical-JSON lines; the
//! wasm parity run emits the same bytes through
//! `runCorpusCase`/`runCorpusExplainLine`.
//!
//! The command line of fixture `<id>` is
//! `{"command": "FREQ AGE", "id": "<id>"}`. The explain line of
//! registry row `<statistic>` (its `[]` expanded to the first
//! concrete instance the covering case emits) is
//! `{"explanation": <bits-encoded Explanation>, "id": "<statistic>",
//! "statistic": "<concrete>"}` — the explanation serialized exactly
//! as `parity_line` encodes `actual`.

use super::parity_norm::encode_bits;
use super::run_case;
use super::synth;
use crate::dataset::Dataset;
use crate::explain::{explain_calculator, explain_procedure};
use crate::export::epi_info_command;
use crate::receipt::canonical_json;
use crate::validation::{statistic_ids, REGISTRY};
use crate::EngineError;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

/// The `(registry statistic, covering fixture key)` pairs the parity
/// run emits one explain trace for — every REGISTRY row with at
/// least one covering fixture, in registry order, each row's first
/// fixture (`dir/id`; the `..`-rooted golden key resolves through
/// `root.join` the same way `report::run_fixture` reads it).
pub fn explain_plan() -> Vec<(String, String)> {
    REGISTRY
        .iter()
        .filter_map(|c| {
            c.fixtures
                .first()
                .map(|f| (c.statistic.to_string(), (*f).to_string()))
        })
        .collect()
}

/// The registry id's family prefix (`freq`, `calc.proportion`) —
/// the same rule `tests/explain_support` uses.
fn registry_prefix(statistic: &str) -> &str {
    if statistic.starts_with("calc.") {
        let mut it = statistic.splitn(3, '.');
        match (it.next(), it.next()) {
            (Some(a), Some(b)) => &statistic[..a.len() + b.len() + 1],
            _ => statistic,
        }
    } else {
        match statistic.split_once('.') {
            Some((f, _)) => &statistic[..f.len()],
            None => statistic,
        }
    }
}

/// The registry statistic's relative path (`freq.categories[].percent`
/// → `categories[].percent`, `calc.proportion.estimate` → `estimate`).
fn registry_rel(statistic: &str) -> Result<&str, EngineError> {
    if statistic.starts_with("calc.") {
        statistic
            .splitn(3, '.')
            .nth(2)
            .ok_or_else(|| corpus(format!("{statistic}: not calc.<id>.<path>")))
    } else {
        statistic
            .split_once('.')
            .map(|(_, r)| r)
            .ok_or_else(|| corpus(format!("{statistic}: not family.<path>")))
    }
}

/// The concrete-index paths a `[]`-pattern expands to inside `tree`;
/// a container path expands to every leaf beneath it — the same
/// expansion `tests/explain_support/tree.rs::concretize` performs,
/// `CorpusFormat` instead of panicking on a shape mismatch.
pub fn concretize(tree: &Value, pattern: &str) -> Result<Vec<String>, EngineError> {
    let mut out = Vec::new();
    let mut frontier: Vec<(&Value, String)> = vec![(tree, String::new())];
    for seg in pattern.split('.') {
        let (key, is_arr) = match seg.strip_suffix("[]") {
            Some(k) => (k, true),
            None => (seg, false),
        };
        let mut next = Vec::new();
        for (node, path) in frontier {
            let child = &node[key];
            if child.is_null() {
                continue;
            }
            let prefix = if path.is_empty() {
                key.to_string()
            } else {
                format!("{path}.{key}")
            };
            if is_arr {
                let items = child
                    .as_array()
                    .ok_or_else(|| corpus(format!("{prefix}: expected an array")))?;
                for (i, item) in items.iter().enumerate() {
                    next.push((item, format!("{prefix}[{i}]")));
                }
            } else {
                next.push((child, prefix));
            }
        }
        frontier = next;
    }
    for (node, path) in frontier {
        match node {
            Value::Object(m) if m.contains_key("undefined") => out.push(path),
            Value::Object(_) | Value::Array(_) => {
                for rel in statistic_ids(node) {
                    out.push(format!("{path}.{}", rel.replace("[]", "[0]")));
                }
            }
            _ => out.push(path),
        }
    }
    Ok(out)
}

/// The Epi Info command of a dataset-backed case, or `None` —
/// `"freq"`, `"means"` and dataset `"two_by_two"`/
/// `"stratified_two_by_two"` fixtures only (spec §7.3).
pub fn command_for(fixture: &Value) -> Result<Option<String>, EngineError> {
    let procedure = fixture["procedure"]
        .as_str()
        .ok_or_else(|| corpus("procedure is not a string"))?;
    let has_dataset = fixture["inputs"].get("dataset").is_some();
    match (procedure, has_dataset) {
        ("freq", true) => Ok(Some(epi_info_command("freq", &synth::freq_opts(fixture)?)?)),
        ("means", true) => Ok(Some(epi_info_command(
            "means",
            &synth::means_opts(fixture)?,
        )?)),
        ("two_by_two" | "stratified_two_by_two", true) => Ok(Some(epi_info_command(
            "tables",
            &synth::tables_options(fixture)?,
        )?)),
        _ => Ok(None),
    }
}

/// The command parity line for one dataset case, or `None`.
pub fn command_line(fixture: &Value) -> Result<Option<String>, EngineError> {
    let Some(command) = command_for(fixture)? else {
        return Ok(None);
    };
    let id = fixture["id"]
        .as_str()
        .ok_or_else(|| corpus("fixture has no string id"))?;
    Ok(Some(canonical_json(&json!({
        "command": command,
        "id": id,
    }))))
}

/// The explain parity line of registry row `statistic` covered by
/// `fixture` — the row's first covering fixture per
/// [`explain_plan`]. The trace goes through the public
/// `explain_procedure`/`explain_calculator` API so the parity run
/// exercises exactly what `explainProcedure`/`explainCalculator`
/// export.
pub fn explain_line(
    fixture: &Value,
    datasets: &BTreeMap<String, Dataset>,
    statistic: &str,
) -> Result<String, EngineError> {
    let procedure = fixture["procedure"]
        .as_str()
        .ok_or_else(|| corpus("procedure is not a string"))?;
    let rel = registry_rel(statistic)?;
    let actual = run_case(fixture, datasets)?.actual;
    let concrete = concretize(&actual, rel)?
        .into_iter()
        .next()
        .ok_or_else(|| {
            corpus(format!(
                "{statistic}: no concrete instance in {}",
                fixture["id"]
            ))
        })?;
    let stat = format!("{}.{concrete}", registry_prefix(statistic));
    let explanation = match procedure {
        "freq" | "means" => {
            let ds = synth::fixture_dataset(fixture, datasets)?;
            let opts = if procedure == "freq" {
                synth::freq_opts(fixture)?
            } else {
                synth::means_opts(fixture)?
            };
            explain_procedure(ds, procedure, &opts, &stat)?
        }
        "two_by_two" | "stratified_two_by_two" => {
            let (ds, opts) = synth::tables_case(fixture, datasets)?;
            explain_procedure(&ds, "tables", &opts, &stat)?
        }
        "rxc" => {
            let (ds, opts) = synth::rxc_case(fixture)?;
            explain_procedure(&ds, "tables", &opts, &stat)?
        }
        name if name.starts_with("calc.") => {
            explain_calculator(&name[5..], &fixture["inputs"], &stat)?
        }
        other => return Err(corpus(format!("unexplainable procedure {other}"))),
    };
    let encoded = serde_json::to_value(&explanation)
        .map_err(|e| corpus(format!("explanation does not serialize: {e}")))?;
    Ok(canonical_json(&json!({
        "explanation": encode_bits(&encoded),
        "id": statistic,
        "statistic": stat,
    })))
}
