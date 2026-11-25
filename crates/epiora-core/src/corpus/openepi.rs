//! Compares engine calculator output with OpenEpi's displayed values
//! (`fixtures/openepi/<id>.json`, v1.0 spec §9). A value passes iff
//! `|engine − openepi| ≤ 0.5 × 10^−d`, `d` = the decimals OpenEpi
//! printed (tolerance class `openepi_display`); a negative `d` means
//! the capture restated a rate to a coarser per-unit scale and the
//! bound loosens accordingly.
//!
//! The stored values are already on the fixture's scale: the capture
//! (`fixtures/openepi/capture.mjs`, `scaleToR`) divided percent
//! displays by 100 (decimals + 2) and restated person-time rates to
//! the fixture's per-unit when it wrote the file.
//!
//! Each case runs `calc::run` on the `cases.json` inputs — the
//! capture's `--dry-run` check verifies they equal the matching Task 2
//! `fixtures/calc/<id>/<case>.json` inputs exactly, so the comparison
//! runs the same numbers R validated.

use super::compare::Mismatch;
use crate::procedures::RunContext;
use crate::EngineError;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The fixed `RunContext` timestamp — the same one `run_case` uses,
/// so runs are reproducible bit for bit.
const CASE_TIMESTAMP: &str = "2026-10-30T12:00:00Z";
/// The class label on every OpenEpi mismatch (spec §9).
const CLASS: &str = "openepi_display";
/// Files inside `fixtures/openepi/` that are not capture output.
const NON_OUTPUT_FILES: &[&str] = &[
    "cases.json",
    "labels.json",
    "package.json",
    "package-lock.json",
];

/// The outcome of comparing the `openepi/` capture files against the
/// engine.
#[derive(Debug)]
pub struct OpenEpiReport {
    /// How many mapped values were compared.
    pub checked: usize,
    /// Mapped and mismatched counts per calculator, in file order —
    /// the validation report's per-calculator rows.
    pub calculators: Vec<CalculatorCheck>,
    /// Labels OpenEpi printed with `statistic: null`, deduplicated in
    /// first-seen order as `"<calculator>: <label>"`.
    pub unmapped: Vec<String>,
    /// Every mapped value failing the display rule that is not a
    /// documented method difference.
    pub mismatches: Vec<Mismatch>,
    /// Mapped values failing the display rule on a statistic whose
    /// `OutputDef.method` carries the `OE_METHOD_MARK` label — a
    /// documented method difference (v1.0 Task 13, `docs/decisions.md`
    /// `### v1.0 Task 13 — method choices`). They are still reported,
    /// but they are not mismatches: OpenEpi computes a different
    /// method or approximation than the R-validated engine method.
    pub method_differences: Vec<Mismatch>,
}

/// One calculator's counts for the report's per-calculator row.
#[derive(Debug)]
pub struct CalculatorCheck {
    /// The calculator id (the output file's `calculator` field).
    pub id: String,
    /// Mapped values compared for this calculator.
    pub checked: usize,
    /// Mapped values failing the display rule without a documented
    /// method difference.
    pub mismatches: usize,
    /// Mapped values failing the display rule on a documented
    /// method-difference statistic.
    pub differences: usize,
}

/// The `OutputDef.method` substring marking a documented method
/// difference between the engine's method (the R fixture) and the
/// method OpenEpi shows. Set by Task 13's labels; each marked
/// statistic is row in `docs/decisions.md` `### v1.0 Task 13 —
/// method choices` with evidence.
pub(crate) const OE_METHOD_MARK: &str = "OpenEpi shows";

/// `calc.two_by_two.strata.0.z` → `calc.two_by_two.strata[].z`: a
/// numeric path segment is an array index — it folds into the
/// preceding segment as `[]` — so a captured statistic key matches
/// the registry's `[]` statistic id.
pub(crate) fn normalize_key(key: &str) -> String {
    let mut out = String::new();
    for seg in key.split('.') {
        if !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit()) {
            out.push_str("[]");
        } else {
            if !out.is_empty() {
                out.push('.');
            }
            out.push_str(seg);
        }
    }
    out
}

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: CASE_TIMESTAMP.to_string(),
        project: None,
        working_view: None,
    }
}

/// `0.5 × 10^−d` — half of OpenEpi's last displayed digit. `d` may be
/// negative (a rate restated to a coarser per-unit scale): the bound
/// grows accordingly. Repeated division/multiplication, never `powi`
/// (spec §3.4).
pub fn display_tolerance(decimals: i64) -> f64 {
    let mut t = 0.5;
    for _ in 0..decimals {
        t /= 10.0;
    }
    for _ in decimals..0 {
        t *= 10.0;
    }
    t
}

/// The comparison rule (spec §9): `|actual − expected| ≤ 0.5 × 10^−d`.
pub fn within_display(actual: f64, expected: f64, decimals: i64) -> bool {
    (actual - expected).abs() <= display_tolerance(decimals)
}

/// The `<id>.json` capture output files inside `dir`, sorted for a
/// deterministic run order; `cases.json`, `labels.json` and the
/// package manifests are skipped.
pub(crate) fn capture_files(dir: &Path) -> Result<Vec<PathBuf>, EngineError> {
    let mut files = Vec::new();
    for entry in
        std::fs::read_dir(dir).map_err(|e| corpus(format!("read {}: {e}", dir.display())))?
    {
        let path = entry
            .map_err(|e| corpus(format!("read {}: {e}", dir.display())))?
            .path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let skip = path
            .file_name()
            .is_some_and(|n| NON_OUTPUT_FILES.contains(&n.to_string_lossy().as_ref()));
        if !skip {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

/// `cases.json` as a `(calculator, case) → inputs` lookup.
fn load_cases(dir: &Path) -> Result<BTreeMap<(String, String), Value>, EngineError> {
    let path = dir.join("cases.json");
    let cases: Value = serde_json::from_str(
        &std::fs::read_to_string(&path)
            .map_err(|e| corpus(format!("read {}: {e}", path.display())))?,
    )
    .map_err(|e| corpus(format!("parse {}: {e}", path.display())))?;
    let mut map = BTreeMap::new();
    for entry in cases
        .as_array()
        .ok_or_else(|| corpus("cases.json is not an array"))?
    {
        let calculator = entry
            .get("calculator")
            .and_then(Value::as_str)
            .ok_or_else(|| corpus("cases.json entry has no \"calculator\" string"))?;
        let case = entry
            .get("case")
            .and_then(Value::as_str)
            .ok_or_else(|| corpus(format!("{calculator}: entry has no \"case\" string")))?;
        let inputs = entry
            .get("inputs")
            .ok_or_else(|| corpus(format!("{calculator}/{case}: no \"inputs\"")))?;
        map.insert((calculator.to_string(), case.to_string()), inputs.clone());
    }
    Ok(map)
}

/// Walks the engine result along `calc.<id>.` remainder segments; a
/// numeric segment indexes an array (`strata.0.zscore` → `strata[0]`).
fn resolve<'a>(actual: &'a Value, segs: &[&str]) -> Option<&'a Value> {
    let mut node = actual;
    for seg in segs {
        node = match node {
            Value::Array(items) => items.get(seg.parse::<usize>().ok()?)?,
            _ => node.get(*seg)?,
        };
    }
    Some(node)
}

/// Runs `calc::run` for every `openepi/*.json` case and compares each
/// mapped value at `openepi_display`. Unmapped labels (`statistic`
/// null) are collected; a missing case in `cases.json`, a mapped
/// non-numeric value or a `calc::run` failure is an error, not a
/// mismatch — a number OpenEpi showed but the engine left undefined
/// is a mismatch.
pub fn compare_openepi(dir: &Path) -> Result<OpenEpiReport, EngineError> {
    let cases = load_cases(dir)?;
    let mut report = OpenEpiReport {
        checked: 0,
        calculators: Vec::new(),
        unmapped: Vec::new(),
        mismatches: Vec::new(),
        method_differences: Vec::new(),
    };
    let mut seen_unmapped = BTreeSet::new();
    for path in capture_files(dir)? {
        check_file(&path, &cases, &mut report, &mut seen_unmapped)?;
    }
    Ok(report)
}

fn check_file(
    path: &Path,
    cases: &BTreeMap<(String, String), Value>,
    report: &mut OpenEpiReport,
    seen_unmapped: &mut BTreeSet<String>,
) -> Result<(), EngineError> {
    let doc: Value = serde_json::from_str(
        &std::fs::read_to_string(path)
            .map_err(|e| corpus(format!("read {}: {e}", path.display())))?,
    )
    .map_err(|e| corpus(format!("parse {}: {e}", path.display())))?;
    let calculator = doc
        .get("calculator")
        .and_then(Value::as_str)
        .ok_or_else(|| corpus(format!("{}: no \"calculator\" string", path.display())))?;
    if path.file_stem().and_then(|s| s.to_str()) != Some(calculator) {
        return Err(corpus(format!(
            "{}: \"calculator\" {calculator:?} does not match the file name",
            path.display()
        )));
    }
    let mut check = CalculatorCheck {
        id: calculator.to_string(),
        checked: 0,
        mismatches: 0,
        differences: 0,
    };
    for case in doc
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| corpus(format!("{calculator}: no \"cases\" array")))?
    {
        check_case(calculator, case, cases, report, &mut check, seen_unmapped)?;
    }
    report.checked += check.checked;
    report.calculators.push(check);
    Ok(())
}

fn check_case(
    calculator: &str,
    case: &Value,
    cases: &BTreeMap<(String, String), Value>,
    report: &mut OpenEpiReport,
    check: &mut CalculatorCheck,
    seen_unmapped: &mut BTreeSet<String>,
) -> Result<(), EngineError> {
    let case_id = case
        .get("case")
        .and_then(Value::as_str)
        .ok_or_else(|| corpus(format!("{calculator}: case without a \"case\" string")))?;
    let inputs = cases
        .get(&(calculator.to_string(), case_id.to_string()))
        .ok_or_else(|| corpus(format!("{calculator}/{case_id}: not in cases.json")))?;
    let output = crate::calc::run(calculator, inputs, &ctx())
        .map_err(|e| corpus(format!("{calculator}/{case_id}: calc::run failed: {e}")))?;
    let values = case
        .get("values")
        .and_then(Value::as_array)
        .ok_or_else(|| corpus(format!("{calculator}/{case_id}: no \"values\" array")))?;
    for entry in values {
        check_entry(
            calculator,
            case_id,
            entry,
            &output.result,
            report,
            check,
            seen_unmapped,
        )?;
    }
    Ok(())
}

fn check_entry(
    calculator: &str,
    case_id: &str,
    entry: &Value,
    actual: &Value,
    report: &mut OpenEpiReport,
    check: &mut CalculatorCheck,
    seen_unmapped: &mut BTreeSet<String>,
) -> Result<(), EngineError> {
    let label = entry
        .get("label")
        .and_then(Value::as_str)
        .ok_or_else(|| corpus(format!("{calculator}/{case_id}: value without a label")))?;
    let bad = |what: &str| corpus(format!("{calculator}/{case_id}: {label:?} {what}"));
    let statistic = match entry.get("statistic") {
        Some(Value::String(s)) => s.clone(),
        Some(Value::Null) | None => {
            let key = format!("{calculator}: {label}");
            if seen_unmapped.insert(key.clone()) {
                report.unmapped.push(key);
            }
            return Ok(());
        }
        _ => return Err(bad("has a non-string \"statistic\"")),
    };
    let expected = entry
        .get("value")
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .ok_or_else(|| bad("is mapped but has no finite \"value\""))?;
    let decimals = entry
        .get("decimals")
        .and_then(Value::as_i64)
        .ok_or_else(|| bad("has no integer \"decimals\""))?;
    let segs: Vec<&str> = statistic.split('.').collect();
    if segs.len() < 3 || segs[0] != "calc" || segs[1] != calculator {
        return Err(bad(&format!(
            "statistic {statistic:?} is not a {calculator:?} output path"
        )));
    }
    check.checked += 1;
    let node = resolve(actual, &segs[2..]);
    if node
        .and_then(Value::as_f64)
        .is_some_and(|v| within_display(v, expected, decimals))
    {
        return Ok(());
    }
    let mm = Mismatch {
        path: format!("{calculator}/{case_id} {label:?} -> {statistic} ({decimals} decimals)"),
        expected: Some(expected),
        actual: node.map_or_else(|| "missing".to_string(), |v| v.to_string()),
        class: CLASS.to_string(),
    };
    // A numeric disagreement on a statistic the registry labels with
    // the OpenEpi method (`OE_METHOD_MARK`, Task 13's table) is a
    // documented method difference, not a mismatch — the engine
    // value is the R-validated one. A missing engine value stays a
    // mismatch either way.
    if node.and_then(Value::as_f64).is_some() && documented_difference(calculator, &statistic) {
        check.differences += 1;
        report.method_differences.push(mm);
    } else {
        check.mismatches += 1;
        report.mismatches.push(mm);
    }
    Ok(())
}

/// Whether `statistic` (a `calc.<id>.…` capture key) names an
/// `OutputDef` whose `method` marks a documented OpenEpi method
/// difference — i.e. carries `OE_METHOD_MARK`. Numeric segments are
/// array indices, matched against the registry's `[]` id.
fn documented_difference(calculator: &str, statistic: &str) -> bool {
    let normalized = normalize_key(statistic);
    crate::calc::definition(calculator)
        .map(|d| {
            d.outputs
                .iter()
                .any(|o| o.statistic == normalized && o.method.contains(OE_METHOD_MARK))
        })
        .unwrap_or(false)
}
