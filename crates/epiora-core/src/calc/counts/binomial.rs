//! `binomial` — EI7 StatCalc's binomial block: exact tails against
//! the expected proportion, the two-tailed p, and the
//! Clopper–Pearson 95% limits expressed as a count range.

use super::shared::*;
use crate::error::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let x = cnt(inputs, "numerator");
    let n = cnt(inputs, "denominator");
    if x > n {
        return Err(EngineError::CalcInputOutOfRange {
            input: "numerator".into(),
            reason: "above_max".into(),
        });
    }
    let p = num(inputs, "expected_percent") / 100.0;
    let t = binom_tails(x, n, p);
    let [cl, cu] = binom_cp_ci(x, n, 95.0);
    Ok(json!({
        "tail": {
            "lt": leaf(t.lt), "le": leaf(t.le),
            "ge": leaf(t.ge), "gt": leaf(t.gt),
            "eq": leaf(dbinom(x, n, p)),
        },
        "p_2t": leaf((2.0 * t.le.min(t.ge)).min(1.0)),
        "ci_95_count": {
            "lower": leaf((cl * n as f64).round()),
            "upper": leaf((cu * n as f64).round()),
        },
    }))
}
