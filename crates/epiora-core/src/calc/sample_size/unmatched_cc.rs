//! `ss_unmatched_cc` — unmatched case-control sample sizes (Kelsey,
//! Fleiss, Fleiss with continuity correction). The case exposure
//! proportion comes from `percent_cases_exposed` directly or is
//! derived from `odds_ratio` on the control proportion — the
//! registry's one-of group guarantees exactly one form
//! (`fixtures/R/methods/sample_size.R`).

use super::two_props;
use crate::calc::counts::shared::{conf, num};
use crate::EngineError;
use serde_json::Value;

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let p2 = num(inputs, "percent_controls_exposed") / 100.0;
    // p1 = p2·OR / (1 + p2·(OR − 1)) — the odds-ratio inversion.
    let p1 = inputs
        .get("percent_cases_exposed")
        .and_then(Value::as_f64)
        .map_or_else(
            || {
                let or = num(inputs, "odds_ratio");
                p2 * or / (1.0 + p2 * (or - 1.0))
            },
            |pct| pct / 100.0,
        );
    Ok(two_props::run(
        "cases",
        "controls",
        conf(inputs),
        num(inputs, "power"),
        num(inputs, "ratio_controls_per_case"),
        p1,
        p2,
    ))
}
