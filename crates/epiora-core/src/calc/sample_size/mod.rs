//! The sample-size family (v1.0 Task 9): `ss_proportion` (OpenEpi
//! population-survey and EI7 clustered forms at OpenEpi's seven
//! displayed confidence levels), `ss_unmatched_cc` and `ss_cohort`
//! (Kelsey, Fleiss and Fleiss-with-continuity-correction) and
//! `ss_mean_difference` (OpenEpi `SSMean.js`).

pub(crate) mod cohort;
pub(crate) mod mean_difference;
pub(crate) mod proportion;
pub(crate) mod two_props;
pub(crate) mod unmatched_cc;

use crate::EngineError;
use serde_json::Value;

/// Runs sample-size calculator `id` on canonical inputs.
pub(super) fn run(id: &str, inputs: &Value) -> Result<Value, EngineError> {
    match id {
        "ss_proportion" => proportion::run(inputs),
        "ss_unmatched_cc" => unmatched_cc::run(inputs),
        "ss_cohort" => cohort::run(inputs),
        "ss_mean_difference" => mean_difference::run(inputs),
        other => Err(EngineError::CalcUnknown { id: other.into() }),
    }
}
