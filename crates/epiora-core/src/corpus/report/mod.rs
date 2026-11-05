//! Builds the v0.2 validation report (platform §12.10, v0.2 plan
//! Task 13): one Markdown row per `validation::REGISTRY` statistic,
//! every number measured by running the covering corpus fixtures
//! through `run_case` — nothing is transcribed. The header names the
//! build-time git SHA (fixed at build time, never read at run time),
//! the CI run URL, the wasm `gzip -9` size and the Chromium bench
//! median, which the caller passes in from the `engine` and `bench`
//! jobs of `engine.yml`.

mod ei7;

use super::compare::compare;
use super::load_dataset_dir;
use super::run_case::run_case;
use crate::dataset::Dataset;
use crate::validation::{Coverage, ToleranceClass, REGISTRY};
use crate::EngineError;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// One report row: the measured error for one registry statistic.
pub struct Row {
    /// Registry statistic id, e.g. `two_by_two.odds_ratio.cmle`.
    pub statistic: &'static str,
    /// Reference software: the R version and packages that generated
    /// the covering fixtures, plus `Epi Info 7` when the ei7 check
    /// maps a label to this statistic.
    pub reference: String,
    /// The tolerance class the fixtures assign to the leaf.
    pub class: ToleranceClass,
    /// Maximum `|actual − expected| / |expected|` over every leaf of
    /// the covering fixtures (`|actual|` when the reference is 0);
    /// `f64::INFINITY` when a leaf could not be compared numerically
    /// (number where the reference says undefined, or vice versa).
    pub worst_error: f64,
    /// Number of covering fixtures.
    pub fixture_count: usize,
}

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

/// Measured corpus output of one fixture plus the fixture itself
/// (for `expected` and `source`).
struct Ran {
    fixture: Value,
    actual: Value,
}

/// Loads and runs fixture `dir/name` once, caching by id; the run
/// also goes through `compare` so a malformed fixture is a
/// `CorpusFormat` error exactly as in the corpus tests.
fn run_fixture<'a>(
    cache: &'a mut BTreeMap<String, Ran>,
    root: &Path,
    datasets: &BTreeMap<String, Dataset>,
    id: &str,
) -> Result<&'a Ran, EngineError> {
    if !cache.contains_key(id) {
        let path = root.join(format!("{id}.json"));
        let text = std::fs::read_to_string(&path)
            .map_err(|e| corpus(format!("read {}: {e}", path.display())))?;
        let fixture: Value = serde_json::from_str(&text)
            .map_err(|e| corpus(format!("parse {}: {e}", path.display())))?;
        let output =
            run_case(&fixture, datasets).map_err(|e| corpus(format!("{}: {e}", path.display())))?;
        compare(&fixture, &output.actual)?;
        cache.insert(
            id.to_string(),
            Ran {
                fixture,
                actual: output.actual,
            },
        );
    }
    cache
        .get(id)
        .ok_or_else(|| corpus(format!("fixture {id} vanished from cache")))
}

/// Adds the leaf's measured error to `errors`: E7's error term
/// `|a − e| / |e|` (`|a|` when `e = 0`), `f64::INFINITY` when the
/// engine's value is not the reference's kind.
fn leaf_error(
    expected: &Value,
    actual: Option<&Value>,
    errors: &mut Vec<f64>,
) -> Result<(), EngineError> {
    let leaf = expected
        .as_object()
        .filter(|m| m.contains_key("value"))
        .ok_or_else(|| corpus("statistic path does not end at a leaf"))?;
    match leaf.get("value") {
        Some(Value::Null) => {
            let ok = actual
                .and_then(|a| a.get("undefined"))
                .is_some_and(Value::is_string);
            if !ok {
                errors.push(f64::INFINITY);
            }
            Ok(())
        }
        Some(v @ Value::Number(_)) => {
            let e = v
                .as_f64()
                .ok_or_else(|| corpus("leaf value is not finite"))?;
            match actual.and_then(Value::as_f64) {
                Some(a) => errors.push(if e == 0.0 {
                    a.abs()
                } else {
                    (a - e).abs() / e.abs()
                }),
                None => errors.push(f64::INFINITY),
            }
            Ok(())
        }
        _ => Err(corpus("leaf has a non-numeric, non-null \"value\"")),
    }
}

/// Walks `expected` along the registry path (`.`-separated, `[]` for
/// every array position), collecting each leaf's measured error.
fn collect_errors(
    expected: &Value,
    actual: Option<&Value>,
    segs: &[&str],
    errors: &mut Vec<f64>,
) -> Result<(), EngineError> {
    match segs.split_first() {
        None => leaf_error(expected, actual, errors),
        Some((seg, rest)) => match seg.strip_suffix("[]") {
            Some(key) => {
                let arr = expected
                    .get(key)
                    .and_then(Value::as_array)
                    .ok_or_else(|| corpus(format!("{key}: not an array in expected")))?;
                let actual_arr = actual.and_then(|a| a.get(key)).and_then(Value::as_array);
                for (i, item) in arr.iter().enumerate() {
                    collect_errors(item, actual_arr.and_then(|a| a.get(i)), rest, errors)?;
                }
                Ok(())
            }
            None => {
                let child = expected
                    .get(*seg)
                    .ok_or_else(|| corpus(format!("{seg}: path not in expected")))?;
                collect_errors(child, actual.and_then(|a| a.get(*seg)), rest, errors)
            }
        },
    }
}

fn row(
    coverage: &Coverage,
    root: &Path,
    datasets: &BTreeMap<String, Dataset>,
    cache: &mut BTreeMap<String, Ran>,
    ei7_stats: &BTreeSet<String>,
) -> Result<Row, EngineError> {
    let rel = coverage
        .statistic
        .split_once('.')
        .map_or(coverage.statistic, |(_, r)| r);
    let segs: Vec<&str> = rel.split('.').collect();
    let mut errors = Vec::new();
    let mut r_versions = BTreeSet::new();
    let mut packages = BTreeSet::new();
    for id in coverage.fixtures {
        let ran = run_fixture(cache, root, datasets, id)?;
        collect_errors(
            &ran.fixture["expected"],
            Some(&ran.actual),
            &segs,
            &mut errors,
        )
        .map_err(|e| corpus(format!("{id}: {e}")))?;
        if let Some(v) = ran.fixture["source"]["r_version"].as_str() {
            r_versions.insert(v.to_string());
        }
        if let Some(pk) = ran.fixture["source"]["packages"].as_object() {
            for (name, ver) in pk {
                let ver = ver.as_str().unwrap_or("?");
                packages.insert(format!("{name} {ver}"));
            }
        }
    }
    let mut reference = format!(
        "{}: {}",
        r_versions.into_iter().collect::<Vec<_>>().join(", "),
        packages.into_iter().collect::<Vec<_>>().join(", ")
    );
    if ei7_stats.contains(coverage.statistic) {
        reference.push_str("; Epi Info 7");
    }
    Ok(Row {
        statistic: coverage.statistic,
        reference,
        class: coverage.class,
        worst_error: errors.iter().copied().fold(0.0, f64::max),
        fixture_count: coverage.fixtures.len(),
    })
}

/// Runs every fixture any registry entry names (each once) and returns
/// one measured row per `REGISTRY` entry, in registry order.
pub fn build_rows(fixtures_dir: &Path) -> Result<Vec<Row>, EngineError> {
    let datasets = load_dataset_dir(&fixtures_dir.join("data"))
        .map_err(|e| corpus(format!("load datasets: {e}")))?;
    let ei7_stats = ei7::covered_statistics(fixtures_dir);
    let mut cache = BTreeMap::new();
    let mut rows = Vec::with_capacity(REGISTRY.len());
    for coverage in REGISTRY {
        rows.push(row(
            coverage,
            fixtures_dir,
            &datasets,
            &mut cache,
            &ei7_stats,
        )?);
    }
    Ok(rows)
}

fn class_name(class: ToleranceClass) -> &'static str {
    match class {
        ToleranceClass::ClosedForm => "closed_form",
        ToleranceClass::Iterative => "iterative",
    }
}

/// `1e-9` / `1e-6`.
fn fmt_tol(tol: f64) -> String {
    format!("{tol:.0e}")
}

/// `0`, `inf` or `d.dde-n`.
fn fmt_error(worst: f64) -> String {
    if worst == 0.0 {
        "0".to_string()
    } else if worst.is_infinite() {
        "inf".to_string()
    } else {
        format!("{worst:.2e}")
    }
}

/// `130445` → `130,445`; `412.345…` → `412.3`; anything else verbatim
/// (e.g. "not measured").
fn fmt_header_number(raw: &str) -> String {
    if let Ok(n) = raw.parse::<u64>() {
        let digits = n.to_string();
        let mut out = String::new();
        for (i, c) in digits.chars().enumerate() {
            if i > 0 && (digits.len() - i) % 3 == 0 {
                out.push(',');
            }
            out.push(c);
        }
        out
    } else if let Ok(f) = raw.parse::<f64>() {
        format!("{f:.1}")
    } else {
        raw.to_string()
    }
}

/// The full Markdown report (platform §12.10). `run_url` is the CI
/// run that produced it; `wasm_gzip_bytes` and `bench_median_ms` are
/// the `engine`/`bench` job measurements, or "not measured" when the
/// caller has none.
pub fn build_report(
    fixtures_dir: &Path,
    run_url: &str,
    wasm_gzip_bytes: &str,
    bench_median_ms: &str,
) -> Result<String, EngineError> {
    let rows = build_rows(fixtures_dir)?;
    let mut md = String::from("# Epiora v0.2 validation report\n\n");
    md.push_str("Generated by `validation-report` (v0.2 Task 13); every row is measured by running the corpus.\n\n");
    md.push_str(&format!(
        "- Commit: `{}`\n- CI run: {run_url}\n- Optimised engine `.wasm` after `gzip -9`: {} bytes \
         (budget 409,600)\n- Chromium bench median (1,000,000 rows, `datasetFromColumns` + \
         stratified `tables`): {} ms (budget < 1000 ms)\n- Epidemiologist adviser sign-off: pending \
         (required before v1.0, platform §12.9)\n",
        env!("EPIORA_GIT_SHA"),
        fmt_header_number(wasm_gzip_bytes),
        fmt_header_number(bench_median_ms),
    ));
    md.push_str(
        "\n## Corpus coverage\n\nOne row per `validation::REGISTRY` statistic; *worst \
         relative error* is the maximum `|actual − expected| / |expected|` over every leaf of the \
         covering fixtures (`|actual|` when the reference is 0), measured by `run_case`. `inf` \
         marks a leaf the engine did not produce in the reference's kind (number vs undefined). \
         A row passes iff its worst error is within the class tolerance.\n\n",
    );
    md.push_str("| Procedure | Statistic | Reference | Class | Tolerance | Worst relative error | Fixtures |\n");
    md.push_str("|---|---|---|---|---|---|---|\n");
    for r in &rows {
        let procedure = r.statistic.split('.').next().unwrap_or(r.statistic);
        md.push_str(&format!(
            "| {procedure} | {} | {} | {} | {} | {} | {} |\n",
            r.statistic,
            r.reference,
            class_name(r.class),
            fmt_tol(r.class.tolerance()),
            fmt_error(r.worst_error),
            r.fixture_count,
        ));
    }
    md.push_str(&ei7::section(fixtures_dir)?);
    Ok(md)
}
