//! `two_by_n` — EI7 StatCalc's 2×2 statistics block: per-stratum
//! and crude chi-squares (Pearson/Yates/Mantel, two-sided only),
//! Fisher and mid-p exacts, Wald RR/RD/OR, and the MLE odds ratio
//! with mid-p and Fisher limits (`exact2x2`).

use super::{exact_or, shared::*};
use crate::error::EngineError;
use crate::special::{hypergeom_pmf, NcHypergeom};
use crate::stats::Table2x2;
use serde_json::{json, Value};

fn chi_leaf(t: &Table2x2) -> Value {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0, m1, m0) = (a + b, c + d, a + c, b + d);
    let n = n1 + n0;
    // `stats::chisq.test` — (O−E)²/E sums, Yates clamps at 0.
    let cells = [
        [a, n1 * m1 / n],
        [b, n1 * m0 / n],
        [c, n0 * m1 / n],
        [d, n0 * m0 / n],
    ];
    let unc: f64 = cells.iter().map(|&[o, e]| (o - e) * (o - e) / e).sum();
    let yat: f64 = cells
        .iter()
        .map(|&[o, e]| {
            let s = (o - e).abs() - 0.5;
            s.max(0.0) * s.max(0.0) / e
        })
        .sum();
    let delta = a - n1 * m1 / n;
    let mh = delta * delta / (n1 * n0 * m1 * m0 / (n * n * (n - 1.0)));
    let mk = |x: f64| json!({ "value": leaf(x), "p_2t": leaf(chi_p(x, 1.0)) });
    json!({ "uncorrected": mk(unc), "mantel": mk(mh), "yates": mk(yat) })
}

fn table_block(t: &Table2x2, conf: f64) -> Value {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0) = (a + b, c + d);
    let zz = z(conf);
    // Owner-decision size guard (PR #74): the exact-path leaves go
    // `too_large` before `NcHypergeom`'s support allocation when this
    // table's support exceeds `MAX_EXACT_SUPPORT`.
    let too_large = hypergeom_support(t) > MAX_EXACT_SUPPORT;
    let (m1, n_tot_m0) = (t.a + t.c, t.b + t.d);
    let u = || undefined("too_large");
    let (midp_2t, fisher_1t, fisher_2t, mle, ml, mu, fl, fu) = if too_large {
        (u(), u(), u(), u(), u(), u(), u(), u())
    } else {
        let h = NcHypergeom::new(m1, n_tot_m0, n1 as u64);
        let lo = h.lower_tail(t.a, 1.0);
        let up = h.upper_tail(t.a, 1.0);
        let pm = hypergeom_pmf(t.a, m1, n_tot_m0, n1 as u64) / 2.0;
        let p1_f = lo.min(up);
        let p2_f = crate::stats::fisher_p_two_sided(&h, m1, n_tot_m0, n1 as u64, t.a);
        let p1_m = (lo - pm).min(up - pm);
        let mle = exact_or::cmle_or(&h, t.a);
        let [ml, mu] = exact_or::midp_ci(&h, t.a, conf);
        let [fl, fu] = exact_or::fisher_ci(&h, t.a, conf);
        (
            leaf((2.0 * p1_m).min(1.0)),
            leaf(p1_f),
            leaf(p2_f),
            leaf(mle),
            leaf(ml),
            leaf(mu),
            leaf(fl),
            leaf(fu),
        )
    };

    let pe = a / n1;
    let pu = c / n0;
    let rr = pe / pu;
    let rr_se = libm::sqrt(b / (a * n1) + d / (c * n0));
    let rd = pe - pu;
    let rd_se = libm::sqrt(pe * (1.0 - pe) / n1 + pu * (1.0 - pu) / n0);
    let or_ = (a * d) / (b * c);
    let or_se = libm::sqrt(1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d);

    json!({
        "odds_ratio": stat_ci([or_, or_ * libm::exp(-zz * or_se), or_ * libm::exp(zz * or_se)]),
        "odds_ratio_mle": {
            "midp": { "est": mle, "lower": ml, "upper": mu },
            "fisher": { "lower": fl, "upper": fu },
        },
        "risk_ratio": stat_ci([rr, rr * libm::exp(-zz * rr_se), rr * libm::exp(zz * rr_se)]),
        "risk_difference": stat_ci([rd * 100.0, (rd - zz * rd_se) * 100.0, (rd + zz * rd_se) * 100.0]),
        "chisq": chi_leaf(t),
        "exact": {
            "midp_2t": midp_2t,
            "fisher_1t": fisher_1t,
            "fisher_2t": fisher_2t,
        },
    })
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let strata = strata2x2(inputs);
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
    Ok(json!({
        "strata": strata.iter().map(|t| table_block(t, conf)).collect::<Vec<_>>(),
        "crude": table_block(&crude, conf),
    }))
}
