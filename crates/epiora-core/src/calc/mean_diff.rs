//! The `mean_1`/`mean_2` | `difference` and `sd_*` | `var_*` input
//! derivations shared by `ss_mean_difference` and
//! `power_mean_difference` (inventory `## ss_mean_difference`,
//! `## power_mean_difference`). The registry's `groups` guarantee the
//! forms' cardinality; these resolve which one was given.

use super::counts::shared::num;
use crate::EngineError;
use serde_json::Value;

/// The effect difference: `difference` when given, else
/// `mean_1 − mean_2`.
pub(crate) fn difference(inputs: &Value) -> f64 {
    if let Some(d) = inputs.get("difference").and_then(Value::as_f64) {
        return d;
    }
    num(inputs, "mean_1") - num(inputs, "mean_2")
}

/// The two group sds: `(sd_1, sd_2)` or `(√var_1, √var_2)` — the
/// inventory's pair forms ("`sd_1`,`sd_2` or `var_1`,`var_2`"). The
/// per-side `sd_i`/`var_i` one-of groups let a mixed pair (e.g.
/// `sd_1` with `var_2`) through validation; the inventory admits only
/// matched pairs, so a mixed pair is the `bad_choice` typed input
/// error.
pub(crate) fn sds(inputs: &Value) -> Result<(f64, f64), EngineError> {
    let pair = |i: usize| {
        (
            inputs.get(format!("sd_{i}")).and_then(Value::as_f64),
            inputs.get(format!("var_{i}")).and_then(Value::as_f64),
        )
    };
    match (pair(1), pair(2)) {
        ((Some(a), None), (Some(b), None)) => Ok((a, b)),
        ((None, Some(a)), (None, Some(b))) => Ok((libm::sqrt(a), libm::sqrt(b))),
        _ => Err(EngineError::CalcInputOutOfRange {
            input: "sd_1/sd_2/var_1/var_2".into(),
            reason: "bad_choice".into(),
        }),
    }
}
