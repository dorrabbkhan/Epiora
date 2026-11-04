//! Tolerance classes and the comparison rule E7 (spec §6.2), plus the
//! static validation registry (spec §4.3): every statistic the engine
//! can emit maps to the corpus fixtures that cover it. Every fixture
//! leaf is compared at one of these classes; the classes themselves
//! are the plan's fixed decision rule.

mod registry;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

/// How tightly an expected value must be matched (spec §6.2).
/// `ei7_display` (Epi Info 7 comparisons only, spec §6.4) is not a class
/// here — its tolerance is per-value (`0.5 × 10^−d`) and arrives with
/// Task 9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToleranceClass {
    /// Directly computed values: 1e-9 relative.
    ClosedForm,
    /// Root-found values: 1e-6 relative.
    Iterative,
}

impl ToleranceClass {
    /// The plan's fixed tolerance for this class.
    pub fn tolerance(self) -> f64 {
        match self {
            Self::ClosedForm => 1e-9,
            Self::Iterative => 1e-6,
        }
    }

    /// Comparison rule E7 (spec §6.2): pass iff
    /// `|actual − expected| ≤ tol × |expected|`, or `|actual| ≤ tol`
    /// when `expected = 0`.
    pub fn passes(actual: f64, expected: f64, tol: f64) -> bool {
        if expected == 0.0 {
            actual.abs() <= tol
        } else {
            (actual - expected).abs() <= tol * expected.abs()
        }
    }
}

/// One registry row: the corpus fixtures covering a statistic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Coverage {
    /// Family-prefixed dotted leaf path, `[]` for array positions —
    /// e.g. `two_by_two.odds_ratio.cmle`, `freq.categories[].percent`.
    pub statistic: &'static str,
    /// `dir/id` fixture names under `fixtures/`.
    pub fixtures: &'static [&'static str],
    /// The tolerance class the fixtures assign to the leaf.
    pub class: ToleranceClass,
}

const fn cov(
    statistic: &'static str,
    fixtures: &'static [&'static str],
    class: ToleranceClass,
) -> Coverage {
    Coverage {
        statistic,
        fixtures,
        class,
    }
}

/// Every registry entry for `family` (the prefix before the first `.`).
pub fn coverage_for(family: &str) -> Vec<&'static Coverage> {
    REGISTRY
        .iter()
        .filter(|c| {
            c.statistic
                .strip_prefix(family)
                .is_some_and(|rest| rest.starts_with('.'))
        })
        .collect()
}

/// The leaf ids a serialized stats object emits (spec §4.3): numbers
/// and `{"undefined": …}` leaves; `[]` marks array positions. Strings
/// and booleans (labels, flags) are not statistics.
pub fn statistic_ids(v: &Value) -> Vec<String> {
    fn ids(v: &Value, path: &str, out: &mut BTreeSet<String>) {
        match v {
            Value::Object(m) if m.contains_key("undefined") => {
                out.insert(path.to_string());
            }
            Value::Object(m) => {
                for (k, c) in m {
                    let child = if path.is_empty() {
                        k.clone()
                    } else {
                        format!("{path}.{k}")
                    };
                    ids(c, &child, out);
                }
            }
            Value::Array(items) => {
                for c in items {
                    ids(c, &format!("{path}[]"), out);
                }
            }
            Value::Number(_) => {
                out.insert(path.to_string());
            }
            _ => {}
        }
    }
    let mut out = BTreeSet::new();
    ids(v, "", &mut out);
    out.into_iter().collect()
}

/// The validation registry (spec §4.3): one entry per statistic any
/// procedure can emit, listing the corpus fixtures that cover it.
/// Derived from the fixture trees — `tests/registry.rs` asserts that
/// every emitted statistic is registered and that every named fixture
/// exists and contains the leaf. The entries live in per-family
/// submodules under `validation/registry/` (`registry::ALL` flattens
/// them in registry order: freq, rxc, stratified_two_by_two,
/// two_by_two).
pub static REGISTRY: &[Coverage] = &registry::ALL;
