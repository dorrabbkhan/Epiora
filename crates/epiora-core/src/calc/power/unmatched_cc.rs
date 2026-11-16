//! `power_unmatched_cc` — OpenEpi `PowerCC.js`: the Fleiss §3.18
//! sample-size formula inverted, `powerz = (√(n1·d²) − zα√((1+1/r)·p̄q̄))
//! / √(p1q1 + p2q2/r)` with `r = n2/n1`, emitted as `Φ(powerz)·100`.
//! `normal_cc` adjusts n1 by `±(r+1)/(r·d)` (the sign follows the
//! odds ratio's direction) before the same formula; `odds_ratio` is
//! the implied OR from the two proportions
//! (`fixtures/R/methods/sample_size.R::power_cc`, `_cc`).

use crate::calc::counts::shared::{cnt, conf, leaf, num, z};
use crate::special::normal_cdf;
use crate::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let za = z(conf(inputs));
    let p1 = num(inputs, "percent_cases_exposed") / 100.0;
    let p2 = num(inputs, "percent_controls_exposed") / 100.0;
    let n1 = cnt(inputs, "n_cases") as f64;
    let n2 = cnt(inputs, "n_controls") as f64;
    let r = n2 / n1;

    let q1 = 1.0 - p1;
    let q2 = 1.0 - p2;
    let d = p1 - p2;
    let pbar = (p1 + r * p2) / (1.0 + r);
    let qbar = 1.0 - pbar;
    let denom = libm::sqrt(p1 * q1 + p2 * q2 / r);

    let powerz = (libm::sqrt(n1 * d * d) - za * libm::sqrt((1.0 + 1.0 / r) * pbar * qbar)) / denom;

    let or = (p1 * q2) / (p2 * q1);
    // The continuity correction adds/subtracts (r+1)/(r·d) from n1
    // following the OR's direction.
    let nn1 = if or > 1.0 {
        n1 - (r + 1.0) / (r * d)
    } else {
        n1 + (r + 1.0) / (r * d)
    };
    let powerz_cc =
        (libm::sqrt(nn1.abs() * d * d) - za * libm::sqrt((1.0 + 1.0 / r) * pbar * qbar)) / denom;

    Ok(json!({
        "normal": leaf(normal_cdf(powerz) * 100.0),
        "normal_cc": leaf(normal_cdf(powerz_cc) * 100.0),
        "odds_ratio": leaf(or),
    }))
}
