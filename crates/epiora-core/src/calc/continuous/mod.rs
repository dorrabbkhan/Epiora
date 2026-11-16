//! The continuous-summary family (v1.0 Task 8): `mean_ci`,
//! `median_ci`, `t_test_summary` and `anova_summary` — calculators on
//! summary statistics (n, mean, sd) rather than raw data. The t-test
//! and ANOVA paths share `stats::means_summary` with MEANS itself.

mod anova_summary;
mod mean_ci;
mod median_ci;
mod t_test_summary;

use super::counts::shared::{leaf, undefined};
use crate::stats::Estimate;
use crate::EngineError;
use serde_json::Value;

/// An [`Estimate`] as a result leaf — the shared `leaf`/`undefined`
/// writers, kept in one place for the continuous calculators.
fn est_leaf(e: &Estimate) -> Value {
    match e {
        Estimate::Value(x) => leaf(*x),
        Estimate::Undefined { reason } => undefined(reason.code()),
    }
}

/// Runs continuous calculator `id` on canonical inputs — dispatched
/// from `calc::run` only for `Family::Continuous`.
pub(super) fn run(id: &str, inputs: &Value) -> Result<Value, EngineError> {
    match id {
        "mean_ci" => mean_ci::run(inputs),
        "median_ci" => median_ci::run(inputs),
        "t_test_summary" => t_test_summary::run(inputs),
        "anova_summary" => anova_summary::run(inputs),
        other => Err(EngineError::CalcUnknown { id: other.into() }),
    }
}
