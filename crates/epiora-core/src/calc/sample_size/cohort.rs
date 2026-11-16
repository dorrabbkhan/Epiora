//! `ss_cohort` — cohort/cross-sectional/RCT sample sizes (Kelsey,
//! Fleiss, Fleiss with continuity correction). The exposed outcome
//! proportion comes from `percent_exposed_outcome` directly or is
//! derived from the unexposed proportion through `odds_ratio`,
//! `risk_ratio` or `risk_difference` — the registry's four-member
//! one-of group guarantees exactly one form
//! (`fixtures/R/methods/sample_size.R`).

use super::two_props;
use crate::calc::counts::shared::{conf, num};
use crate::EngineError;
use serde_json::Value;

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let p2 = num(inputs, "percent_unexposed_outcome") / 100.0;
    let p1 = if let Some(pct) = inputs
        .get("percent_exposed_outcome")
        .and_then(Value::as_f64)
    {
        pct / 100.0
    } else if let Some(or) = inputs.get("odds_ratio").and_then(Value::as_f64) {
        // p1 = p2·OR / (1 + p2·(OR − 1)).
        p2 * or / (1.0 + p2 * (or - 1.0))
    } else if let Some(rr) = inputs.get("risk_ratio").and_then(Value::as_f64) {
        p2 * rr
    } else {
        // risk_difference, in percent.
        p2 + num(inputs, "risk_difference") / 100.0
    };
    Ok(two_props::run(
        "exposed",
        "unexposed",
        conf(inputs),
        num(inputs, "power"),
        num(inputs, "ratio_unexposed_per_exposed"),
        p1,
        p2,
    ))
}
