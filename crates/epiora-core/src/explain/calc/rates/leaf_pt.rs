//! `ef_pt_leaf` and `pt_block_leaf` — one stratum/crude block's
//! leaf surface.
use super::*;

/// The `etiologic` fractions of one person-time stratum —
/// `ef_person_time`'s intermediates, percent-scaled.
pub(crate) fn ef_pt_leaf(
    t: &crate::calc::person_time::martin_pt::PtStratum,
    conf: f64,
    name: &str,
    side: &str,
    statistic: &str,
    est: Estimate,
) -> Result<Explanation, EngineError> {
    let (a, b, n1, n0) = (t.a, t.b, t.n1, t.n0);
    // `ef_person_time`'s intermediates, as the run computes them.
    let p = rates::ef_pt_parts(a, b, n1, n0, conf);
    let (re, ru, ro) = (p.re, p.ru, p.ro);
    let (rr, rr_lo, rr_hi) = (p.rr, p.rr_lo, p.rr_hi);
    let share = p.share;
    let (efp_lo, efp_hi) = (p.efp_lo, p.efp_hi);

    let (fid, v) = match (name, side) {
        ("efp", "est") => ("ef_pt_efp", finite(&[("ro", ro), ("ru", ru)])),
        ("efp", "lower") => (
            "ef_pt_efp_lower",
            finite(&[("share", share), ("rr_lo", rr_lo)]),
        ),
        ("efp", "upper") => (
            "ef_pt_efp_upper",
            finite(&[("share", share), ("rr_hi", rr_hi)]),
        ),
        ("efe", "est") => ("ef_pt_efe", finite(&[("rr", rr)])),
        ("efe", "lower") => ("ef_pt_efe_lower", finite(&[("rr_lo", rr_lo)])),
        ("efe", "upper") => ("ef_pt_efe_upper", finite(&[("rr_hi", rr_hi)])),
        ("pfp", "est") => ("ef_pt_pfp", finite(&[("ru", ru), ("ro", ro)])),
        ("pfp", "lower") => ("ef_pt_pfp_lower", finite(&[("efp_hi", efp_hi)])),
        ("pfp", "upper") => ("ef_pt_pfp_upper", finite(&[("efp_lo", efp_lo)])),
        ("pfe", "est") => ("ef_pt_pfe", finite(&[("ru", ru), ("re", re)])),
        ("pfe", "lower") => ("ef_pt_pfe_lower", finite(&[("rr_hi", rr_hi)])),
        ("pfe", "upper") => ("ef_pt_pfe_upper", finite(&[("rr_lo", rr_lo)])),
        _ => return Err(unknown(statistic)),
    };
    Ok(closed(statistic, fid, v, est))
}

/// One `rate_compare_table` block's leaf (`strata[i].*` or `crude.*`).
pub(crate) fn pt_block_leaf(
    t: &crate::calc::person_time::martin_pt::PtStratum,
    pu: f64,
    conf: f64,
    statistic: &str,
    segs: &[Segment],
    est: Estimate,
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let (a, b, n1, n0) = (t.a, t.b, t.n1, t.n0);
    let zz = z(conf);

    // `rate_stat` — the `pois.approx` CI scaled by `per_unit`.
    let rate_leaf = |x: f64, pt: f64, side: &str, est: Estimate| {
        let fid = match side {
            "est" => "rate",
            "lower" => "rate_taylor_lower",
            "upper" => "rate_taylor_upper",
            _ => return Err(u()),
        };
        Ok(closed(
            statistic,
            fid,
            vals(&[("x", x), ("pt", pt), ("z", zz), ("per_unit", pu)]),
            est,
        ))
    };

    match segs {
        [f, l] if f.key == "zscore" => {
            let zs = rates::pt_zscore(a, b, n1, n0);
            match l.key.as_str() {
                "value" => Ok(closed(
                    statistic,
                    "pt_zscore",
                    vals(&[("a", a), ("b", b), ("n1", n1), ("n0", n0)]),
                    est,
                )),
                "p_1t" => Ok(closed(
                    statistic,
                    "normal_cdf_neg",
                    finite(&[("z", zs[0])]),
                    est,
                )),
                "p_2t" => Ok(closed(statistic, "p_2t", finite(&[("p", zs[1])]), est)),
                _ => Err(u()),
            }
        }
        [f, r, l] if f.key == "rate" => match r.key.as_str() {
            "exposed" => rate_leaf(a, n1, &l.key, est),
            "unexposed" => rate_leaf(b, n0, &l.key, est),
            "overall" => rate_leaf(a + b, n1 + n0, &l.key, est),
            _ => Err(u()),
        },
        [f, l] if f.key == "rate_ratio" => {
            let (re, ru, rr, se) = rates::rate_ratio_parts(a, b, n1, n0);
            match l.key.as_str() {
                "est" => Ok(closed(
                    statistic,
                    "rate_ratio",
                    finite(&[("re", re), ("ru", ru)]),
                    est,
                )),
                "lower" | "upper" => Ok(closed(
                    statistic,
                    if l.key == "lower" {
                        "wald_exp_lower"
                    } else {
                        "wald_exp_upper"
                    },
                    finite(&[("est", rr), ("se", se), ("z", zz)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "rate_difference" => {
            let (re, ru, dp, se) = rates::rd_byar_parts(a, b, n1, n0);
            match l.key.as_str() {
                "est" => Ok(closed(
                    statistic,
                    "rate_difference",
                    finite(&[("re", re), ("ru", ru), ("per_unit", pu)]),
                    est,
                )),
                "lower" | "upper" => Ok(closed(
                    statistic,
                    if l.key == "lower" {
                        "wald_lower_scaled"
                    } else {
                        "wald_upper_scaled"
                    },
                    finite(&[("est", dp), ("se", se), ("z", zz), ("per_unit", pu)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "cmle_rr" => {
            let v = vals(&[("a", a), ("b", b), ("n1", n1), ("n0", n0), ("conf", conf)]);
            match l.key.as_str() {
                "est" => Ok(method(statistic, "method.cmle", v, est)),
                "midp_lower" | "midp_upper" => Ok(method(statistic, "method.midp_ci", v, est)),
                "fisher_lower" | "fisher_upper" => {
                    Ok(method(statistic, "method.fisher_ci", v, est))
                }
                _ => Err(u()),
            }
        }
        [f, name, l] if f.key == "etiologic" => {
            ef_pt_leaf(t, conf, &name.key, &l.key, statistic, est)
        }
        _ => Err(u()),
    }
}
