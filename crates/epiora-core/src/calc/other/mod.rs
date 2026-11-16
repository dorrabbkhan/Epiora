//! The `other` family (v1.0 Task 9): `random_numbers`, the seeded
//! uniform-integer generator. It has no R fixture (spec §5.2 — R
//! cannot reproduce a ChaCha stream); its oracle is the golden file
//! `tests/golden/random_numbers.json` plus the distribution property
//! tests in `tests/calc_random.rs`.

pub mod random_numbers;

use crate::EngineError;
use serde_json::Value;

/// Runs `other`-family calculator `id` on canonical inputs.
pub(super) fn run(id: &str, inputs: &Value) -> Result<Value, EngineError> {
    match id {
        "random_numbers" => random_numbers::run(inputs),
        other => Err(EngineError::CalcUnknown { id: other.into() }),
    }
}
