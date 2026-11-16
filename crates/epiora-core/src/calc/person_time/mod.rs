//! The person-time family (v1.0 Task 8): `rate_one` (a Poisson rate
//! with five CI methods) and `rate_compare` (the two-rate comparison
//! over strata). The corpus `fixtures/calc/<id>` trees are the
//! oracle.

mod martin_pt;
mod rate_compare;
mod rate_one;

use crate::EngineError;
use serde_json::Value;

/// Runs person-time calculator `id` on canonical inputs — dispatched
/// from `calc::run` only for `Family::PersonTime`.
pub(super) fn run(id: &str, inputs: &Value) -> Result<Value, EngineError> {
    match id {
        "rate_one" => rate_one::run(inputs),
        "rate_compare" => rate_compare::run(inputs),
        other => Err(EngineError::CalcUnknown { id: other.into() }),
    }
}
