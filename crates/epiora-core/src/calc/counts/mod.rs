//! The counts family (v1.0 Task 7): the ten calculators that run on
//! count inputs. `defs.rs` holds the registry data; each module
//! computes the result tree the `fixtures/calc/<id>` corpus records.

mod binomial;
pub(crate) mod defs;
pub(crate) mod defs_rest;
mod dose_response;
mod exact_or;
mod martin;
mod matched_case_control;
mod poisson;
mod proportion;
mod r_by_c;
mod screening;
mod shared;
mod shared_exact;
mod smr;
mod two_by_n;
mod two_by_two;

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
