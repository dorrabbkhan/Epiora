//! `median_ci` — the normal-approximation CI for a percentile
//! expressed as order-statistic ranks, `np ∓ z√(npq)` per
//! `fixtures/R/methods/means.R::percentile_ci` (Conover; OpenEpi
//! `CIMedian.js`'s method, unrounded — the corpus records the raw
//! ranks, including out-of-range ones).

use super::super::counts::shared::{conf, leaf, num, undefined, z};
use crate::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let n = num(inputs, "n");
    let p = num(inputs, "percentile") / 100.0;
    // The method needs at least one expected observation on each side
    // of the percentile: `n·p < 1` or `n·q < 1` leaves nothing to
    // bracket — `Undefined { too_few }` (this also covers the
    // degenerate endpoints percentile = 0 or 100, where `npq` is 0).
    if n * p < 1.0 || n * (1.0 - p) < 1.0 {
        let u = undefined("too_few");
        return Ok(json!({ "normal": { "lower_rank": u, "upper_rank": u } }));
    }
    let mid = n * p;
    let half = z(conf(inputs)) * libm::sqrt(n * p * (1.0 - p));
    Ok(json!({
        "normal": {
            "lower_rank": leaf(mid - half),
            "upper_rank": leaf(mid + half),
        },
    }))
}
