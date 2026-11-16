//! The Kelsey/Fleiss/Fleiss-with-continuity-correction two-proportion
//! sample sizes shared by `ss_unmatched_cc` and `ss_cohort`
//! (`fixtures/R/methods/sample_size.R`; Kelsey et al. Table 12-15
//! for Kelsey and Fleiss-CC, Fleiss formula 3.19 for Fleiss).
//! `group1`/`group2` name the two sides — cases/controls or
//! exposed/unexposed — and `r` is the group-2-per-group-1 ratio.
//!
//! Review Focus 3: equal proportions (`p1 == p2`, reached directly or
//! through a unit effect measure) divide the formula by a zero
//! difference — the required size is infinite, so every leaf of all
//! three methods turns `{"undefined": "infinite_sample_size"}`,
//! never a giant or negative number. A derived `p1` outside
//! `[0, 1]` (impossible via `risk_ratio`/`risk_difference`) turns
//! them `out_of_range`.

use crate::calc::counts::shared::{leaf, undefined, z};
use crate::special::normal_quantile;
use serde_json::{json, Value};

/// One method's block: `group1`, `group2`, `total` and their `_real`
/// siblings — the real values are the formula's output before the
/// ceiling; the rounded leaves are `ceil` of them and `total` is the
/// sum of the two ceilings (the fixture convention).
fn method_block(g1_name: &str, g2_name: &str, vn: f64, r: f64) -> Value {
    let g1r = vn;
    let g2r = vn * r;
    let mut m = serde_json::Map::new();
    if g1r.is_finite() && g2r.is_finite() {
        let (a, b) = (g1r.ceil(), g2r.ceil());
        m.insert(g1_name.into(), leaf(a));
        m.insert(format!("{g1_name}_real"), leaf(g1r));
        m.insert(g2_name.into(), leaf(b));
        m.insert(format!("{g2_name}_real"), leaf(g2r));
        m.insert("total".into(), leaf(a + b));
        m.insert("total_real".into(), leaf(g1r + g2r));
    } else {
        let reason = if vn.is_nan() || g2r.is_nan() {
            undefined("out_of_range")
        } else {
            undefined("infinite_sample_size")
        };
        for k in [
            g1_name.to_string(),
            format!("{g1_name}_real"),
            g2_name.to_string(),
            format!("{g2_name}_real"),
            "total".into(),
            "total_real".into(),
        ] {
            m.insert(k, reason.clone());
        }
    }
    Value::Object(m)
}

/// The `kelsey`/`fleiss`/`fleiss_cc` result for conf/power `za`,`zb`
/// over proportions `p1`,`p2` and allocation ratio `r`.
pub(super) fn run(
    g1_name: &str,
    g2_name: &str,
    conf: f64,
    power: f64,
    r: f64,
    p1: f64,
    p2: f64,
) -> Value {
    // vn is +inf at p1 == p2 (division by zero d²); the Fleiss-CC
    // correction multiplies it by |d| = 0 into NaN, so the zero
    // effect is short-circuited here: every leaf of every method is
    // infinite_sample_size (Review Focus 3).
    if (0.0..=1.0).contains(&p1) && p1 == p2 {
        return json!({
            "kelsey": method_block(g1_name, g2_name, f64::INFINITY, r),
            "fleiss": method_block(g1_name, g2_name, f64::INFINITY, r),
            "fleiss_cc": method_block(g1_name, g2_name, f64::INFINITY, r),
        });
    }
    let (p1, p2) = if (0.0..=1.0).contains(&p1) {
        (p1, p2)
    } else {
        (f64::NAN, p2)
    };
    let za = z(conf);
    // zb is one-sided: qnorm(power/100), not the two-sided z().
    let zb = normal_quantile(power / 100.0).unwrap_or(f64::NAN);
    let pbar = (p1 + r * p2) / (1.0 + r);
    let qbar = 1.0 - pbar;
    let d = p1 - p2;
    let dd = d * d;

    let vk = (za + zb) * (za + zb) * pbar * qbar * (r + 1.0) / (dd * r);

    let fnum = za * libm::sqrt((r + 1.0) * pbar * qbar)
        + zb * libm::sqrt(r * p1 * (1.0 - p1) + p2 * (1.0 - p2));
    let vf = fnum * fnum / (r * dd);

    let vfc =
        vf * libm::pow(
            1.0 + libm::sqrt(1.0 + 2.0 * (r + 1.0) / (vf * r * d.abs())),
            2.0,
        ) / 4.0;

    json!({
        "kelsey": method_block(g1_name, g2_name, vk, r),
        "fleiss": method_block(g1_name, g2_name, vf, r),
        "fleiss_cc": method_block(g1_name, g2_name, vfc, r),
    })
}
