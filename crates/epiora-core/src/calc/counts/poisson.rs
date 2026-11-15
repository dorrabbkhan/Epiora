//! `poisson` — EI7 StatCalc's Poisson block: exact tails and the
//! point probability under `Poisson(expected)`.

use super::shared::*;
use crate::error::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let o = cnt(inputs, "observed");
    let e = num(inputs, "expected");
    let pt = crate::special::poisson_tail;
    let lt = if o == 0 { 0.0 } else { pt(o - 1, e, false) };
    let le = pt(o, e, false);
    let ge = pt(o, e, true);
    let gt = pt(o + 1, e, true);
    Ok(json!({
        "tail": {
            "lt": leaf(lt), "le": leaf(le), "ge": leaf(ge),
            "gt": leaf(gt), "eq": leaf(dpois(o, e)),
        },
    }))
}
