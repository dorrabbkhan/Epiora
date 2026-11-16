//! The power family (v1.0 Task 9): `power_unmatched_cc` and
//! `power_cohort` — OpenEpi's `PowerCC.js`/`PowerCohort.js` normal
//! approximation with and without the continuity correction (Fleiss
//! §3.18 inverted) plus the implied effect measure — and
//! `power_mean_difference` (`PowerMean.js`).

mod cohort;
mod mean_difference;
mod unmatched_cc;

use crate::EngineError;
use serde_json::Value;

/// Runs power calculator `id` on canonical inputs.
pub(super) fn run(id: &str, inputs: &Value) -> Result<Value, EngineError> {
    match id {
        "power_unmatched_cc" => unmatched_cc::run(inputs),
        "power_cohort" => cohort::run(inputs),
        "power_mean_difference" => mean_difference::run(inputs),
        other => Err(EngineError::CalcUnknown { id: other.into() }),
    }
}
