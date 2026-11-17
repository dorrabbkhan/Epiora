//! `two_by_two` — the OpenEpi/EI7 counts table: per-stratum and
//! crude blocks (chi-squares, Fisher/mid-p exacts, risks, RR/RD/OR,
//! CMLE OR with mid-p and Fisher limits, etiologic/prevented
//! fractions) plus the `adjusted` block over strata (MH and directly
//! adjusted measures, Breslow–Day homogeneity, stratified CMLE).
//! Risks, RD and the etiologic fractions are percent-scaled as the
//! corpus records.

use super::{
    adjusted::adjusted,
    ef::{ef_or, ef_risk},
    exact_or,
    shared::*,
};
use crate::error::EngineError;
use crate::special::{hypergeom_pmf, NcHypergeom};
use crate::stats::Table2x2;
use serde_json::{json, Value};

fn strata(inputs: &Value) -> Vec<Table2x2> {
    strata2x2(inputs)
}

/// `stats::chisq.test` Pearson and Yates statistics plus the
/// single-stratum `mantelhaen.test` (N−1 variance) chi-square;
/// `p_1t` is the halved p-value as the corpus records.
fn chi_leaf(t: &Table2x2) -> Value {
    let [unc, yat, mh] = chi_parts(t);
    let mk = |x: f64| {
        let p = chi_p(x, 1.0);
        json!({ "value": leaf(x), "p_1t": leaf(p / 2.0), "p_2t": leaf(p) })
    };
    json!({ "uncorrected": mk(unc), "yates": mk(yat), "mantel_haenszel": mk(mh) })
}

/// `tab_pvals` — the four directional and two-sided exacts of one
/// table. `p_1t` is the smaller directional tail; mid-p halves the
/// observed point mass; Fisher 2t is `fisher.test`'s probability-
/// ordered p (`stats::exact::fisher_p_two_sided`).
fn exacts(t: &Table2x2) -> (Value, Value) {
    let m1 = t.a + t.c;
    let n1 = t.a + t.b;
    let n0 = t.b + t.d;
    let h = NcHypergeom::new(m1, n0, n1);
    let lo = h.lower_tail(t.a, 1.0);
    let up = h.upper_tail(t.a, 1.0);
    let pm = hypergeom_pmf(t.a, m1, n0, n1) / 2.0;
    let lo_m = lo - pm;
    let up_m = up - pm;
    let p1_f = lo.min(up);
    let p2_f = crate::stats::fisher_p_two_sided(&h, m1, n0, n1, t.a);
    let p1_m = lo_m.min(up_m);
    (
        json!({ "p_1t": leaf(p1_f), "p_2t": leaf(p2_f) }),
        json!({ "p_1t": leaf(p1_m), "p_2t": leaf((2.0 * p1_m).min(1.0)) }),
    )
}

/// `two_by_two_table` — the per-table block (`strata[]` and `crude`).
fn table_block(t: &Table2x2, conf: f64) -> Value {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0) = (a + b, c + d);
    let n = n1 + n0;
    // Owner-decision size guard (PR #74): the exact-path leaves go
    // `too_large` before `NcHypergeom`'s support allocation when this
    // table's support exceeds `MAX_EXACT_SUPPORT`.
    let too_large = hypergeom_support(t) > MAX_EXACT_SUPPORT;
    let u = || undefined("too_large");
    let (fisher, midp) = if too_large {
        (
            json!({ "p_1t": u(), "p_2t": u() }),
            json!({ "p_1t": u(), "p_2t": u() }),
        )
    } else {
        exacts(t)
    };
    let zz = z(conf);

    let pc = |x: u64, nn: u64| {
        let mut v = prop_taylor(x, nn, conf);
        for x in &mut v {
            *x *= 100.0;
        }
        stat_ci(v)
    };
    let [_, _, rr, rr_se, rd, rd_se, or_, or_se] = wald_parts(t);

    let (cmle, ml, mu, fl, fu) = if too_large {
        (u(), u(), u(), u(), u())
    } else {
        let h = NcHypergeom::new(t.a + t.c, t.b + t.d, n1 as u64);
        let cmle = exact_or::cmle_or(&h, t.a);
        // `or.midp` first; where it fails (support boundaries, zero
        // cells) `exact2x2(midp = TRUE)` supplies the same leaves.
        let mut mid = exact_or::or_midp_ci(&h, t.a, conf);
        if mid.iter().any(|v| v.is_nan()) {
            mid = exact_or::midp_ci(&h, t.a, conf);
        }
        let [ml, mu] = mid;
        let [fl, fu] = exact_or::fisher_ci(&h, t.a, conf);
        (leaf(cmle), leaf(ml), leaf(mu), leaf(fl), leaf(fu))
    };

    let efr = ef_risk(t, conf);
    let efo = ef_or(t, conf);
    let ef_map = |v: [f64; 3]| stat_ci([v[0] * 100.0, v[1] * 100.0, v[2] * 100.0]);
    let _ = zz;

    json!({
        "chisq": chi_leaf(t),
        "fisher_exact": fisher,
        "midp_exact": midp,
        "risk": {
            "exposed": pc(t.a, n1 as u64),
            "unexposed": pc(t.c, n0 as u64),
            "overall": pc(t.a + t.c, n as u64),
        },
        "risk_ratio": stat_ci([rr, rr * libm::exp(-zz * rr_se), rr * libm::exp(zz * rr_se)]),
        "risk_difference": stat_ci([rd * 100.0, (rd - zz * rd_se) * 100.0, (rd + zz * rd_se) * 100.0]),
        "odds_ratio": stat_ci([or_, or_ * libm::exp(-zz * or_se), or_ * libm::exp(zz * or_se)]),
        "cmle_or": {
            "midp": { "est": cmle, "lower": ml, "upper": mu },
            "fisher": { "lower": fl, "upper": fu },
        },
        "etiologic": {
            "efp": ef_map(efr[0]), "efe": ef_map(efr[1]),
            "pfp": ef_map(efr[2]), "pfe": ef_map(efr[3]),
        },
        "etiologic_or": {
            "efp_or": ef_map(efo[0]), "efe_or": ef_map(efo[1]),
            "pfp_or": ef_map(efo[2]), "pfe_or": ef_map(efo[3]),
        },
    })
}

/// `direct_adjust` — inverse-variance pooling on the log (OR/RR) or
/// raw (RD) scale, Taylor CI.
pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let strata = strata(inputs);
    let conf = conf(inputs);
    let crude = strata.iter().fold(
        Table2x2 {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
        },
        |m, t| Table2x2 {
            a: m.a + t.a,
            b: m.b + t.b,
            c: m.c + t.c,
            d: m.d + t.d,
        },
    );
    let mut out = json!({
        "strata": strata.iter().map(|t| table_block(t, conf)).collect::<Vec<_>>(),
        "crude": table_block(&crude, conf),
    });
    if strata.len() > 1 {
        out["adjusted"] = adjusted(&strata, conf);
    }
    Ok(out)
}
