//! Tolerance classes and the comparison rule E7 (spec §6.2). Every
//! statistic the engine emits is compared to its fixture at one of these
//! classes; the classes themselves are the plan's fixed decision rule.

use serde::{Deserialize, Serialize};

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
