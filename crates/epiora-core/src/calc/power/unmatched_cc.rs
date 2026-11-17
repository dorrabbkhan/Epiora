//! `power_unmatched_cc` — OpenEpi `PowerCC.js`: the Fleiss §3.18
//! sample-size formula inverted, `powerz = (√(n1·d²) − zα√((1+1/r)·p̄q̄))
//! / √(p1q1 + p2q2/r)` with `r = n2/n1`, emitted as `Φ(powerz)·100`.
//! `normal_cc` adjusts n1 by `±(r+1)/(r·d)` (the sign follows the
//! odds ratio's direction) before the same formula; `odds_ratio` is
//! the implied OR from the two proportions
//! (`fixtures/R/methods/sample_size.R::power_cc`, `_cc`).

use super::parts::{cc_inputs, cc_n1, group_ratio, or_cc, pooled_cc, powerz};
use crate::calc::counts::shared::{conf, leaf, z};
use crate::special::normal_cdf;
use crate::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let za = z(conf(inputs));
    let (p1, p2, n1, n2) = cc_inputs(inputs);
    let r = group_ratio(n1, n2);
    let (pbar, denom) = pooled_cc(p1, p2, r);

    let pz = powerz(n1, p1, p2, r, za, pbar, denom);
    let or = or_cc(p1, p2);
    // The continuity correction adds/subtracts (r+1)/(r·d) from n1
    // following the OR's direction.
    let nn1 = cc_n1(n1, r, p1, p2, or > 1.0);
    let pz_cc = powerz(nn1, p1, p2, r, za, pbar, denom);

    Ok(json!({
        "normal": leaf(normal_cdf(pz) * 100.0),
        "normal_cc": leaf(normal_cdf(pz_cc) * 100.0),
        "odds_ratio": leaf(or),
    }))
}
