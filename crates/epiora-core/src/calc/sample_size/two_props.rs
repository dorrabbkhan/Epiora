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

/// One method's finite block intermediates — the pre-ceiling group-2
/// size `g2r` and the ceiled `(a, b)` whose sum is `total` — shared
/// by `method_block` and the explain trace (v1.0 Task 10, spec §6).
pub(crate) struct TwoPropParts {
    /// `vn·r` — group-2 before the ceiling.
    pub g2r: f64,
    /// `vn.ceil()` — reported group-1 size.
    pub a: f64,
    /// `g2r.ceil()` — reported group-2 size.
    pub b: f64,
    /// `a + b` — the reported total.
    pub total: f64,
}

/// `(g2r, a, b, total)` — shared by the run and the explain trace.
pub(crate) fn block_parts(vn: f64, r: f64) -> TwoPropParts {
    let g2r = group2(vn, r);
    let (a, b) = (vn.ceil(), g2r.ceil());
    TwoPropParts {
        g2r,
        a,
        b,
        total: a + b,
    }
}

/// One method's block: `group1`, `group2`, `total` and their `_real`
/// siblings — the real values are the formula's output before the
/// ceiling; the rounded leaves are `ceil` of them and `total` is the
/// sum of the two ceilings (the fixture convention).
fn method_block(g1_name: &str, g2_name: &str, vn: f64, r: f64) -> Value {
    let g1r = vn;
    let p = block_parts(vn, r);
    let g2r = p.g2r;
    let mut m = serde_json::Map::new();
    if g1r.is_finite() && g2r.is_finite() {
        m.insert(g1_name.into(), leaf(p.a));
        m.insert(format!("{g1_name}_real"), leaf(g1r));
        m.insert(g2_name.into(), leaf(p.b));
        m.insert(format!("{g2_name}_real"), leaf(g2r));
        m.insert("total".into(), leaf(p.total));
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

/// One method's `vn` — the per-group-1 required size before the
/// ceiling: `kelsey`, `fleiss`, or `fleiss_cc`. The equal-
/// proportions zero-effect and out-of-range `p1` guards are the
/// run's (Review Focus 3). Shared by the result and the explain
/// trace (v1.0 Task 10, spec §6).
pub(crate) fn vn(method: &str, p1: f64, p2: f64, r: f64, za: f64, zb: f64) -> f64 {
    if (0.0..=1.0).contains(&p1) && p1 == p2 {
        return f64::INFINITY;
    }
    let p1 = if (0.0..=1.0).contains(&p1) {
        p1
    } else {
        f64::NAN
    };
    let pbar = (p1 + r * p2) / (1.0 + r);
    let qbar = 1.0 - pbar;
    let d = p1 - p2;
    let dd = d * d;
    match method {
        "kelsey" => (za + zb) * (za + zb) * pbar * qbar * (r + 1.0) / (dd * r),
        "fleiss" => {
            let fnum = za * libm::sqrt((r + 1.0) * pbar * qbar)
                + zb * libm::sqrt(r * p1 * (1.0 - p1) + p2 * (1.0 - p2));
            fnum * fnum / (r * dd)
        }
        _ => {
            let fnum = za * libm::sqrt((r + 1.0) * pbar * qbar)
                + zb * libm::sqrt(r * p1 * (1.0 - p1) + p2 * (1.0 - p2));
            let vf = fnum * fnum / (r * dd);
            vf * libm::pow(
                1.0 + libm::sqrt(1.0 + 2.0 * (r + 1.0) / (vf * r * d.abs())),
                2.0,
            ) / 4.0
        }
    }
}

/// `vn·r` — the group-2 required size before the ceiling.
pub(crate) fn group2(vn: f64, r: f64) -> f64 {
    vn * r
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

    json!({
        "kelsey": method_block(g1_name, g2_name, vn("kelsey", p1, p2, r, za, zb), r),
        "fleiss": method_block(g1_name, g2_name, vn("fleiss", p1, p2, r, za, zb), r),
        "fleiss_cc": method_block(g1_name, g2_name, vn("fleiss_cc", p1, p2, r, za, zb), r),
    })
}
