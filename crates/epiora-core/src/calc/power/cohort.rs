//! `power_cohort` — OpenEpi `PowerCohort.js` (also used for
//! cross-sectional and RCT `design`s): the same Fleiss §3.18
//! inversion as `power_unmatched_cc` but the pooled proportion is
//! count-weighted (`p̄ = (n1·p1 + n2·p2)/(n1+n2)`, the denominator's
//! second term is `p2q2 + p1q1/r`) and the implied measures are the
//! risk ratio and risk difference instead of the odds ratio
//! (`fixtures/R/methods/sample_size.R::power_cohort`, `_cc`).

use super::parts::{
    cc_n1, cohort_inputs, group_ratio, pooled_cohort, powerz, rd_cohort, rr_cohort,
};
use crate::calc::counts::shared::{conf, leaf, z};
use crate::special::normal_cdf;
use crate::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let za = z(conf(inputs));
    let (p1, p2, n1, n2) = cohort_inputs(inputs);
    let r = group_ratio(n1, n2);
    let (pbar, denom) = pooled_cohort(p1, p2, n1, n2, r);

    let pz = powerz(n1, p1, p2, r, za, pbar, denom);
    let rr = rr_cohort(p1, p2);
    // The continuity correction adds/subtracts (r+1)/(r·d) from n1
    // following the risk ratio's direction.
    let nn1 = cc_n1(n1, r, p1, p2, rr > 1.0);
    let pz_cc = powerz(nn1, p1, p2, r, za, pbar, denom);

    Ok(json!({
        "normal": leaf(normal_cdf(pz) * 100.0),
        "normal_cc": leaf(normal_cdf(pz_cc) * 100.0),
        "risk_ratio": leaf(rr),
        // implied_rr_rd's RD is a fraction (p1 − p2), not a percent.
        "risk_difference": leaf(rd_cohort(p1, p2)),
    }))
}
