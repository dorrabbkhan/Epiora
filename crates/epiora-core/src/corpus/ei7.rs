//! Compares engine output with Epi Info 7's printed values
//! (`fixtures/ei7/oswego.json`, spec §6.4). A value passes iff
//! `|engine − ei7| ≤ 0.5 × 10^−d`, `d` = the decimals Epi Info printed
//! (tolerance class `ei7_display`).
//!
//! Two method choices from the owner decision recorded in
//! docs/decisions.md (v0.2 Task 9, PR #18):
//!
//! - Epi Info builds z-based 95% intervals with z = 1.96; the engine
//!   uses `qnorm(0.975)`. The bounds of those intervals are rescaled to
//!   z = 1.96 *here, in the comparison only*, before the display rule.
//! - Epi Info prints Risk Difference as `RD%`; the engine's
//!   `risk.difference` leaves are 0–1, so they are compared × 100.

use super::compare::Mismatch;
use crate::EngineError;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// `qnorm(0.975)` — the engine's z for a two-sided 95% interval. Used
/// to undo the engine's z before re-applying Epi Info's [`EI7_Z`].
const QNORM_0_975: f64 = 1.959963984540054;
/// Epi Info 7's z for its 95% z-based intervals (owner decision, PR #18).
const EI7_Z: f64 = 1.96;
/// The class label on every ei7 mismatch (spec §6.4).
const CLASS: &str = "ei7_display";

/// The outcome of comparing one ei7 output file against the engine.
#[derive(Debug)]
pub struct Ei7Report {
    /// How many mapped values were compared.
    pub checked: usize,
    /// Labels Epi Info printed with no engine key (`key: null`),
    /// deduplicated in first-seen order.
    pub unmapped: Vec<String>,
    /// Every mapped value failing the display rule.
    pub mismatches: Vec<Mismatch>,
}

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

/// Compares each mapped `values[]` entry of `ei7` against the matching
/// leaf of `actual_by_family`: a `key` is `"<family>.<path>"` (Task 2/3
/// key paths; array indices as `[i]`). Unmapped labels are collected;
/// a missing family, missing path or non-numeric engine leaf is a
/// mismatch.
pub fn compare_ei7(
    ei7: &Value,
    actual_by_family: &BTreeMap<String, Value>,
) -> Result<Ei7Report, EngineError> {
    let outputs = ei7
        .get("outputs")
        .and_then(Value::as_array)
        .ok_or_else(|| corpus("ei7 file has no \"outputs\" array"))?;
    let mut report = Ei7Report {
        checked: 0,
        unmapped: Vec::new(),
        mismatches: Vec::new(),
    };
    let mut seen_unmapped = BTreeSet::new();
    for output in outputs {
        let command = output
            .get("command")
            .and_then(Value::as_str)
            .ok_or_else(|| corpus("ei7 output has no \"command\" string"))?;
        let values = output
            .get("values")
            .and_then(Value::as_array)
            .ok_or_else(|| corpus(format!("{command:?}: no \"values\" array")))?;
        for entry in values {
            check_entry(
                command,
                entry,
                actual_by_family,
                &mut report,
                &mut seen_unmapped,
            )?;
        }
    }
    Ok(report)
}

fn check_entry(
    command: &str,
    entry: &Value,
    actual_by_family: &BTreeMap<String, Value>,
    report: &mut Ei7Report,
    seen_unmapped: &mut BTreeSet<String>,
) -> Result<(), EngineError> {
    let label = entry
        .get("label")
        .and_then(Value::as_str)
        .ok_or_else(|| corpus(format!("{command:?}: value without a label")))?;
    let bad = |what: &str| corpus(format!("{command:?}: {label:?} {what}"));
    let expected = entry
        .get("value")
        .and_then(Value::as_f64)
        .filter(|v| v.is_finite())
        .ok_or_else(|| bad("has no finite \"value\""))?;
    let decimals = entry
        .get("decimals")
        .and_then(Value::as_u64)
        .ok_or_else(|| bad("has no non-negative \"decimals\""))?;
    match entry.get("key") {
        Some(Value::String(key)) => {
            report.checked += 1;
            check_mapped(key, expected, decimals, actual_by_family, report)
        }
        Some(Value::Null) | None => {
            if seen_unmapped.insert(label.to_string()) {
                report.unmapped.push(label.to_string());
            }
            Ok(())
        }
        _ => Err(bad("has a non-string \"key\"")),
    }
}

/// One mapped value: resolve the engine leaf, apply the z-rescale and
/// percent-scale adjustments, then the display rule.
fn check_mapped(
    key: &str,
    expected: f64,
    decimals: u64,
    actual_by_family: &BTreeMap<String, Value>,
    report: &mut Ei7Report,
) -> Result<(), EngineError> {
    let (family, path) = key
        .split_once('.')
        .ok_or_else(|| corpus(format!("key {key:?} has no family prefix")))?;
    let family_actual = actual_by_family.get(family);
    let node = family_actual.and_then(|a| resolve(a, path));

    let compared = match node.and_then(Value::as_f64) {
        Some(v) if is_z_based(key) => {
            let sibling = family_actual
                .and_then(|a| resolve(a, &sibling_bound(path)))
                .and_then(Value::as_f64);
            let lower = path.ends_with(".lower");
            let (lo, hi) = if lower {
                (Some(v), sibling)
            } else {
                (sibling, Some(v))
            };
            match (lo, hi) {
                (Some(lo), Some(hi)) => Some(rescale_z(key, lo, hi, lower)),
                _ => None,
            }
        }
        v => v,
    };
    let compared = compared.map(|x| if is_percent(key) { x * 100.0 } else { x });

    let tol = display_tol(decimals);
    if !compared.is_some_and(|x| (x - expected).abs() <= tol) {
        report.mismatches.push(Mismatch {
            path: key.to_string(),
            expected: Some(expected),
            actual: compared.map_or_else(|| describe(node), |x| x.to_string()),
            class: CLASS.to_string(),
        });
    }
    Ok(())
}

/// Walks `a.b[0].c` inside `actual`.
fn resolve<'a>(actual: &'a Value, path: &str) -> Option<&'a Value> {
    let mut node = actual;
    for seg in path.split('.') {
        if let Some((name, idx)) = seg
            .strip_suffix(']')
            .and_then(|s| s.split_once('['))
            .and_then(|(n, i)| i.parse::<usize>().ok().map(|i| (n, i)))
        {
            node = node.get(name)?.get(idx)?;
        } else {
            node = node.get(seg)?;
        }
    }
    Some(node)
}

/// The keys whose bounds Epi Info builds with z = 1.96: every
/// `*.ci_95_wald.lower|upper` (OR, RR and RD) and the Mantel–Haenszel
/// `*.ci_95.lower|upper` bounds. Fisher/mid-p exact limits, Clopper–
/// Pearson limits, point estimates and p-values are not z-based.
fn is_z_based(key: &str) -> bool {
    let bound = key.ends_with(".lower") || key.ends_with(".upper");
    bound
        && (key.contains("ci_95_wald.")
            || (key.contains("mantel_haenszel.") && key.contains(".ci_95.")))
}

/// OR and RR intervals are symmetric on the log scale; the risk
/// difference interval is additive.
fn is_ratio(key: &str) -> bool {
    key.contains("odds_ratio") || key.contains("risk_ratio")
}

/// Keys Epi Info prints on the percent scale (`RD%`) while the engine
/// keeps them 0–1.
fn is_percent(key: &str) -> bool {
    key.contains("risk.difference")
}

/// `.lower` ↔ `.upper` in the same interval.
fn sibling_bound(path: &str) -> String {
    if let Some(base) = path.strip_suffix(".lower") {
        format!("{base}.upper")
    } else if let Some(base) = path.strip_suffix(".upper") {
        format!("{base}.lower")
    } else {
        path.to_string()
    }
}

/// Rescales an engine 95% interval bound from `qnorm(0.975)` to Epi
/// Info's z = 1.96: ratio intervals on the log scale
/// (`c = sqrt(lo·hi)`, `c·exp(∓z·se)`), additive intervals on the
/// natural scale (`m ∓ z·se`). `se` is recovered from the engine's own
/// bounds.
fn rescale_z(key: &str, lo: f64, hi: f64, lower: bool) -> f64 {
    let sign = if lower { -EI7_Z } else { EI7_Z };
    if is_ratio(key) {
        let c = libm::sqrt(lo * hi);
        let se = (libm::log(hi) - libm::log(lo)) / (2.0 * QNORM_0_975);
        c * libm::exp(sign * se)
    } else {
        let m = (lo + hi) / 2.0;
        let se = (hi - lo) / (2.0 * QNORM_0_975);
        m + sign * se
    }
}

/// `0.5 × 10^−d` — half of Epi Info's last displayed digit.
fn display_tol(decimals: u64) -> f64 {
    let mut t = 0.5;
    for _ in 0..decimals {
        t /= 10.0;
    }
    t
}

fn describe(actual: Option<&Value>) -> String {
    match actual {
        Some(v) => v.to_string(),
        None => "missing".to_string(),
    }
}
