//! `power_cohort` — OpenEpi `PowerCohort.js` (also used for
//! cross-sectional and RCT `design`s): the same Fleiss §3.18
//! inversion as `power_unmatched_cc` but the pooled proportion is
//! count-weighted (`p̄ = (n1·p1 + n2·p2)/(n1+n2)`, the denominator's
//! second term is `p2q2 + p1q1/r`) and the implied measures are the
//! risk ratio and risk difference instead of the odds ratio
//! (`fixtures/R/methods/sample_size.R::power_cohort`, `_cc`).

use crate::calc::counts::shared::{cnt, conf, leaf, num, z};
use crate::special::normal_cdf;
use crate::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let za = z(conf(inputs));
    let p1 = num(inputs, "percent_exposed_outcome") / 100.0;
    let p2 = num(inputs, "percent_unexposed_outcome") / 100.0;
    let n1 = cnt(inputs, "n_exposed") as f64;
    let n2 = cnt(inputs, "n_unexposed") as f64;
    let r = n2 / n1;

    let q1 = 1.0 - p1;
    let q2 = 1.0 - p2;
    let d = p1 - p2;
    let pbar = (n1 * p1 + n2 * p2) / (n1 + n2);
    let qbar = 1.0 - pbar;
    let denom = libm::sqrt(p2 * q2 + p1 * q1 / r);

    let powerz = (libm::sqrt(n1 * d * d) - za * libm::sqrt((1.0 + 1.0 / r) * pbar * qbar)) / denom;

    let rr = p1 / p2;
    // The continuity correction adds/subtracts (r+1)/(r·d) from n1
    // following the risk ratio's direction.
    let nn1 = if rr > 1.0 {
        n1 - (r + 1.0) / (r * d)
    } else {
        n1 + (r + 1.0) / (r * d)
    };
    let powerz_cc =
        (libm::sqrt(nn1.abs() * d * d) - za * libm::sqrt((1.0 + 1.0 / r) * pbar * qbar)) / denom;

    Ok(json!({
        "normal": leaf(normal_cdf(powerz) * 100.0),
        "normal_cc": leaf(normal_cdf(powerz_cc) * 100.0),
        "risk_ratio": leaf(rr),
        // implied_rr_rd's RD is a fraction (p1 − p2), not a percent.
        "risk_difference": leaf(d),
    }))
}
