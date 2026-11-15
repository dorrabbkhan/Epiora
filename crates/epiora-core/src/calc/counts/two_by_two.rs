//! `two_by_two` — the OpenEpi/EI7 counts table: per-stratum and
//! crude blocks (chi-squares, Fisher/mid-p exacts, risks, RR/RD/OR,
//! CMLE OR with mid-p and Fisher limits, etiologic/prevented
//! fractions) plus the `adjusted` block over strata (MH and directly
//! adjusted measures, Breslow–Day homogeneity, stratified CMLE).
//! Risks, RD and the etiologic fractions are percent-scaled as the
//! corpus records.

use super::{exact_or, martin, shared::*};
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
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0, m1, m0) = (a + b, c + d, a + c, b + d);
    let n = n1 + n0;
    // `stats::chisq.test` — Pearson and Yates from the (O−E)²/E sum,
    // not the determinant shortcut; Yates clamps the correction at 0.
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

/// `max(-1, min(1, x))` with R's NaN propagation (Rust's f64::min/max
/// would drop the NaN).
fn rclamp(x: f64) -> f64 {
    if x.is_nan() {
        x
    } else {
        x.clamp(-1.0, 1.0)
    }
}

/// `ef_risk` — the OpenEpi risk-based etiologic/prevented fractions
/// (`fixtures/R/methods/tables.R`).
fn ef_risk(t: &Table2x2, conf: f64) -> [[f64; 3]; 4] {
    let zz = z(conf);
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0, r1, tt) = (a + b, c + d, a + c, a + b + c + d);
    let (re, ru, ro) = (a / n1, c / n0, r1 / tt);
    let rr = re / ru;
    let se = libm::sqrt((1.0 - re) / (n1 * re) + (1.0 - ru) / (n0 * ru));
    let (rr_lo, rr_hi) = (rr * libm::exp(-zz * se), rr * libm::exp(zz * se));
    let clamp = rclamp;
    let efp = clamp((ro - ru) / ro);
    let efe = clamp((rr - 1.0) / rr);
    let pfp = (ru - ro) / ru;
    let pfe = (ru - re) / ru;
    let var_efp = c * tt * (a * d * (tt - c) + b * c * c) / (r1 * r1 * r1 * n0 * n0 * n0);
    let efp_lo = if (efp - zz * libm::sqrt(var_efp)).is_nan() {
        f64::NAN
    } else {
        (efp - zz * libm::sqrt(var_efp)).max(-1.0)
    };
    let efp_hi = if (efp + zz * libm::sqrt(var_efp)).is_nan() {
        f64::NAN
    } else {
        (efp + zz * libm::sqrt(var_efp)).min(1.0)
    };
    [
        [efp, efp_lo, efp_hi],
        [
            efe,
            clamp((rr_lo - 1.0) / rr_lo),
            clamp((rr_hi - 1.0) / rr_hi),
        ],
        [pfp, 1.0 - 1.0 / (1.0 - efp_hi), 1.0 - 1.0 / (1.0 - efp_lo)],
        [pfe, 1.0 - rr_hi, 1.0 - rr_lo],
    ]
}

/// `ef_or` — the OR-derived fractions.
fn ef_or(t: &Table2x2, conf: f64) -> [[f64; 3]; 4] {
    let zz = z(conf);
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (r1, r2) = (a + c, b + d);
    let od = (a * d) / (b * c);
    let se = libm::sqrt(1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d);
    let (od_lo, od_hi) = (od * libm::exp(-zz * se), od * libm::exp(zz * se));
    let clamp = rclamp;
    let share = b / r2;
    let efpor = clamp((share * (od - 1.0)) / (share * (od - 1.0) + 1.0));
    let efeor = clamp((od - 1.0) / od);
    let pfpor = share * (1.0 - od);
    let pfeor = 1.0 - od;
    let v = (c * r2) / (d * r1);
    let var = v * v * (a / (c * r1) + b / (d * r2));
    let raw_lo = efpor - zz * libm::sqrt(var);
    let raw_hi = efpor + zz * libm::sqrt(var);
    let lo = if raw_lo.is_nan() {
        raw_lo
    } else {
        raw_lo.max(-1.0)
    };
    let hi = if raw_hi.is_nan() {
        raw_hi
    } else {
        raw_hi.min(1.0)
    };
    [
        [efpor, lo, hi],
        [
            efeor,
            clamp((od_lo - 1.0) / od_lo),
            clamp((od_hi - 1.0) / od_hi),
        ],
        [pfpor, 1.0 - 1.0 / (1.0 - hi), 1.0 - 1.0 / (1.0 - lo)],
        [pfeor, 1.0 - od_hi, 1.0 - od_lo],
    ]
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
    let pe = a / n1;
    let pu = c / n0;
    let rr = pe / pu;
    let rr_se = libm::sqrt(b / (a * n1) + d / (c * n0));
    let rd = pe - pu;
    let rd_se = libm::sqrt(pe * (1.0 - pe) / n1 + pu * (1.0 - pu) / n0);
    let or_ = (a * d) / (b * c);
    let or_se = libm::sqrt(1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d);

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
fn direct_adjust(strata: &[Table2x2], conf: f64, measure: &str) -> [f64; 3] {
    let zz = z(conf);
    let (mut sw, mut swe) = (0.0, 0.0);
    for t in strata {
        let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
        let (n1, n0) = (a + b, c + d);
        let (e, w) = match measure {
            "or" => (
                libm::log(a * d / (b * c)),
                1.0 / (1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d),
            ),
            "rr" => (
                libm::log((a / n1) / (c / n0)),
                1.0 / (b / (a * n1) + d / (c * n0)),
            ),
            _ => (
                a / n1 - c / n0,
                1.0 / (a * b / (n1 * n1 * n1) + c * d / (n0 * n0 * n0)),
            ),
        };
        sw += w;
        swe += w * e;
    }
    let adj = swe / sw;
    let half = zz / libm::sqrt(sw);
    if measure == "rd" {
        [adj, adj - half, adj + half]
    } else {
        [libm::exp(adj), libm::exp(adj - half), libm::exp(adj + half)]
    }
}

/// `breslow_day` — Woolf homogeneity chi-squares for OR, RR and RD
/// around their directly adjusted estimates.
fn breslow_day(strata: &[Table2x2]) -> Value {
    let k = strata.len() as f64;
    let mut out = serde_json::Map::new();
    for measure in ["or", "rr", "rd"] {
        let (mut sw, mut swe) = (0.0, 0.0);
        let mut ests = Vec::with_capacity(strata.len());
        let mut ws = Vec::with_capacity(strata.len());
        for t in strata {
            let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
            let (n1, n0) = (a + b, c + d);
            let (e, w) = match measure {
                "or" => (
                    libm::log(a * d / (b * c)),
                    1.0 / (1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d),
                ),
                "rr" => (
                    libm::log((a / n1) / (c / n0)),
                    1.0 / (b / (a * n1) + d / (c * n0)),
                ),
                _ => (
                    a / n1 - c / n0,
                    1.0 / (a * b / (n1 * n1 * n1) + c * d / (n0 * n0 * n0)),
                ),
            };
            ests.push(e);
            ws.push(w);
            sw += w;
            swe += w * e;
        }
        let adj = swe / sw;
        let chi: f64 = ests
            .iter()
            .zip(&ws)
            .map(|(e, w)| w * (e - adj) * (e - adj))
            .sum();
        out.insert(format!("{measure}_chisq"), leaf(chi));
        out.insert(format!("{measure}_p"), leaf(chi_p(chi, k - 1.0)));
    }
    Value::Object(out)
}

/// The `adjusted` block — `mantelhaen.test(correct=FALSE)`'s
/// chi-square and RBG OR interval, the Greenland–Robins MH RR, the
/// three directly adjusted measures, Breslow–Day, and the
/// stratified CMLE.
fn adjusted(strata: &[Table2x2], conf: f64) -> Value {
    let zz = z(conf);
    let (mut delta, mut var): (f64, f64) = (0.0, 0.0);
    let (mut sd_, mut so_, mut rg_a, mut rg_b, mut rg_c) = (0.0, 0.0, 0.0, 0.0, 0.0);
    let (mut rr_num, mut rr_den, mut gr_num) = (0.0, 0.0, 0.0);
    for t in strata {
        let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
        let (n1, n0, m1, m0, n) = (a + b, c + d, a + c, b + d, a + b + c + d);
        delta += a - n1 * m1 / n;
        var += n1 * n0 * m1 * m0 / (n * n * (n - 1.0));
        sd_ += a * d / n;
        so_ += b * c / n;
        rg_a += (a + d) * a * d / (n * n);
        rg_b += ((a + d) * b * c + (b + c) * a * d) / (n * n);
        rg_c += (b + c) * b * c / (n * n);
        rr_num += a * n0 / n;
        rr_den += c * n1 / n;
        gr_num += m1 * n1 * n0 / (n * n) - a * c / n;
    }
    let mh_chi = delta * delta / var;
    let or = sd_ / so_;
    let sd =
        libm::sqrt(rg_a / (2.0 * sd_ * sd_) + rg_b / (2.0 * sd_ * so_) + rg_c / (2.0 * so_ * so_));
    let rr = rr_num / rr_den;
    let gr_se = libm::sqrt(gr_num / (rr_num * rr_den));
    // Owner-decision work guard (PR #74): the stratified CMLE leaves
    // go `too_large` before any polynomial is allocated.
    let cmle_or = if martin::too_large(strata) {
        let u = || undefined("too_large");
        json!({
            "est": u(),
            "midp": { "lower": u(), "upper": u() },
            "fisher": { "lower": u(), "upper": u() },
        })
    } else {
        let cmle = martin::stratified_cmle(strata, conf);
        json!({
            "est": leaf(cmle[0]),
            "midp": { "lower": leaf(cmle[1]), "upper": leaf(cmle[2]) },
            "fisher": { "lower": leaf(cmle[3]), "upper": leaf(cmle[4]) },
        })
    };
    json!({
        "mh_chisq": { "value": leaf(mh_chi), "p": leaf(chi_p(mh_chi, 1.0)) },
        "mh_or": stat_ci([or, or * libm::exp(-zz * sd), or * libm::exp(zz * sd)]),
        "mh_rr": stat_ci([rr, libm::exp(libm::log(rr) - zz * gr_se), libm::exp(libm::log(rr) + zz * gr_se)]),
        "direct_or": stat_ci(direct_adjust(strata, conf, "or")),
        "direct_rr": stat_ci(direct_adjust(strata, conf, "rr")),
        "direct_rd": stat_ci(direct_adjust(strata, conf, "rd")),
        "breslow_day": breslow_day(strata),
        "cmle_or": cmle_or,
    })
}

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
