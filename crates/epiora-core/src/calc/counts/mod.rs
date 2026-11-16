//! The counts family (v1.0 Task 7): the ten calculators that run on
//! count inputs. `defs.rs` holds the registry data; each module
//! computes the result tree the `fixtures/calc/<id>` corpus records.

pub(crate) mod binomial;
pub(crate) mod defs;
pub(crate) mod defs_rest;
pub(crate) mod dose_response;
pub(crate) mod exact_or;
pub(crate) mod martin;
pub(crate) mod matched_case_control;
pub(crate) mod poisson;
pub(crate) mod proportion;
pub(crate) mod r_by_c;
pub(crate) mod screening;
pub(crate) mod shared;
pub(crate) mod shared_exact;
pub(crate) mod smr;
pub(crate) mod two_by_n;
pub(crate) mod two_by_two;

use crate::EngineError;
use serde_json::Value;

/// Runs counts calculator `id` on canonical inputs. `run` in
/// `calc/mod.rs` dispatches here only for `Family::Counts`, so an
/// unknown id cannot reach this match.
pub(super) fn run(id: &str, inputs: &Value) -> Result<Value, EngineError> {
    match id {
        "proportion" => proportion::run(inputs),
        "two_by_two" => two_by_two::run(inputs),
        "two_by_n" => two_by_n::run(inputs),
        "r_by_c" => r_by_c::run(inputs),
        "dose_response" => dose_response::run(inputs),
        "matched_case_control" => matched_case_control::run(inputs),
        "screening" => screening::run(inputs),
        "smr" => smr::run(inputs),
        "binomial" => binomial::run(inputs),
        "poisson" => poisson::run(inputs),
        other => Err(EngineError::CalcUnknown { id: other.into() }),
    }
}
