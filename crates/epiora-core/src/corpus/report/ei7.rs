//! The Epi Info 7 leg of the validation report: which registry
//! statistics `ei7/oswego.json` maps labels onto (for the Reference
//! column) and the `## Epi Info 7 check` section, which reruns the
//! three `oswego.pgm` commands (FREQ SEX, TABLES VANILLA ILL, TABLES
//! VANILLA ILL STRATAVAR=SEX — the same construction as
//! `tests/ei7.rs`) and reports `compare_ei7`'s outcome.

use crate::corpus::ei7::compare_ei7;
use crate::corpus::load_dataset;
use crate::procedures::{
    freq, tables, FreqOptions, ProportionCi, RunContext, TablesOptions, TablesResult, VarSpec,
};
use crate::EngineError;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

/// `freq.categories[0].percent` → `freq.categories[].percent`, so a
/// mapped ei7 key can be matched against a registry statistic id.
fn normalize_key(key: &str) -> String {
    let mut out = String::new();
    let mut rest = key;
    while let Some(open) = rest.find('[') {
        match rest[open..].find(']') {
            Some(close) => {
                out.push_str(&rest[..open]);
                out.push_str("[]");
                rest = &rest[open + close + 1..];
            }
            None => break,
        }
    }
    out.push_str(rest);
    out
}

/// The registry statistic ids `ei7/oswego.json` maps labels onto —
/// empty when the file is absent.
pub(super) fn covered_statistics(fixtures_dir: &Path) -> BTreeSet<String> {
    let mut covered = BTreeSet::new();
    let Ok(text) = std::fs::read_to_string(fixtures_dir.join("ei7/oswego.json")) else {
        return covered;
    };
    let Ok(ei7) = serde_json::from_str::<Value>(&text) else {
        return covered;
    };
    if let Some(outputs) = ei7.get("outputs").and_then(Value::as_array) {
        for output in outputs {
            if let Some(values) = output.get("values").and_then(Value::as_array) {
                for entry in values {
                    if let Some(key) = entry.get("key").and_then(Value::as_str) {
                        covered.insert(normalize_key(key));
                    }
                }
            }
        }
    }
    covered
}

/// The `## Epi Info 7 check` Markdown section.
pub(super) fn section(fixtures_dir: &Path) -> Result<String, EngineError> {
    let ei7_path = fixtures_dir.join("ei7/oswego.json");
    let ei7: Value = serde_json::from_str(
        &std::fs::read_to_string(&ei7_path)
            .map_err(|e| corpus(format!("read {}: {e}", ei7_path.display())))?,
    )
    .map_err(|e| corpus(format!("parse {}: {e}", ei7_path.display())))?;
    let ds_path = fixtures_dir.join("data/oswego.dataset.json");
    let ds = load_dataset(
        &std::fs::read_to_string(&ds_path)
            .map_err(|e| corpus(format!("read {}: {e}", ds_path.display())))?,
    )
    .map_err(|e| corpus(format!("load {}: {e}", ds_path.display())))?;
    let report = compare_ei7(&ei7, &actual_by_family(&ds)?)?;
    let mut out = String::from("\n## Epi Info 7 check\n\n");
    out.push_str(&format!(
        "`ei7/oswego.json` (Epi Info 7.2.7.0): {} printed values mapped to engine \
         keys, {} mismatch(es) at `ei7_display` (half a unit in the last displayed \
         decimal, spec §6.4).",
        report.checked,
        report.mismatches.len()
    ));
    for m in &report.mismatches {
        out.push_str(&format!("\n- {m}"));
    }
    if report.unmapped.is_empty() {
        out.push_str(" Every printed label maps to an engine key.\n");
    } else {
        out.push_str(&format!(
            " {} printed labels have no engine key (enumerated in `ei7/labels.json`): \
             every crosstab cell and Row%/Col%, `Total Percent`/`Total Cum. Percent`, \
             `Mid-p exact 1-tailed p`, all per-stratum single-table statistics, and \
             `Adjusted OR (MLE)` with its CI — out of v0.2 scope (v0.2 Task 9).\n",
            report.unmapped.len(),
        ));
    }
    Ok(out)
}

/// The engine outputs for the three `oswego.pgm` commands, keyed by
/// the family names the ei7 keys use.
fn actual_by_family(ds: &crate::dataset::Dataset) -> Result<BTreeMap<String, Value>, EngineError> {
    let ctx = || RunContext {
        timestamp: "2026-10-30T12:00:00Z".to_string(),
        project: None,
    };
    let var = |name: &str| VarSpec {
        name: name.to_string(),
        levels: None,
    };
    let freq_out = freq(
        ds,
        &FreqOptions {
            variables: vec!["SEX".to_string()],
            stratify_by: None,
            include_missing: false,
            ci_method: ProportionCi::ClopperPearson,
        },
        &ctx(),
    )?;
    let tables_out = tables(
        ds,
        &TablesOptions {
            exposure: var("VANILLA"),
            outcome: var("ILL"),
            stratify_by: vec![],
            include_missing: false,
        },
        &ctx(),
    )?;
    let TablesResult::TwoByTwo { crude, .. } = &tables_out.result else {
        return Err(corpus("VANILLA × ILL is not a 2×2"));
    };
    let strat_out = tables(
        ds,
        &TablesOptions {
            exposure: var("VANILLA"),
            outcome: var("ILL"),
            stratify_by: vec!["SEX".to_string()],
            include_missing: false,
        },
        &ctx(),
    )?;
    let TablesResult::TwoByTwo {
        stratified: Some(strat),
        ..
    } = &strat_out.result
    else {
        return Err(corpus("stratified run produced no strata"));
    };
    let first = freq_out
        .result
        .tables
        .first()
        .ok_or_else(|| corpus("FREQ SEX produced no tables"))?;
    let mut actual: BTreeMap<String, Value> = BTreeMap::new();
    for (family, v) in [
        ("freq", serde_json::to_value(first)),
        ("two_by_two", serde_json::to_value(crude)),
        ("stratified_two_by_two", serde_json::to_value(&strat.stats)),
    ] {
        actual.insert(
            family.to_string(),
            v.map_err(|e| corpus(format!("serialize {family}: {e}")))?,
        );
    }
    Ok(actual)
}
