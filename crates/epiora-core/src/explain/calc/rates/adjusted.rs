//! `adjusted_leaf` — direct/MH adjusted person-time leaves.
use super::*;

/// `adjusted.*` — `adjusted_rate_parts`' pooled sums.
pub(crate) fn adjusted_leaf(
    rows: &[[f64; 4]],
    conf: f64,
    statistic: &str,
    segs: &[Segment],
    est: Estimate,
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let p = rates::adjusted_rate_parts(rows);
    let zz = z(conf);
    let k = rows.len() as f64;

    match segs {
        [f, l] if f.key == "zscore" => match l.key.as_str() {
            "value" => Ok(closed(
                statistic,
                "pt_zscore_pooled",
                finite(&[("sum_a", p.sum_a), ("sum_e", p.sum_e), ("var", p.mh_var)]),
                est,
            )),
            "p" => Ok(closed(
                statistic,
                "normal_cdf_neg_2",
                finite(&[("z", p.zs)]),
                est,
            )),
            _ => Err(u()),
        },
        [f, l] if f.key == "direct_rr" => match l.key.as_str() {
            "est" => Ok(closed(
                statistic,
                "direct_rr",
                finite(&[("adj_ln", p.adj_ln_rr)]),
                est,
            )),
            "lower" | "upper" => Ok(closed(
                statistic,
                if l.key == "lower" {
                    "direct_rr_ln_lower"
                } else {
                    "direct_rr_ln_upper"
                },
                finite(&[("adj", p.adj_ln_rr), ("sw", p.sw_rr), ("z", zz)]),
                est,
            )),
            _ => Err(u()),
        },
        [f, l] if f.key == "mh_rr" => {
            let mh = p.mh_num / p.mh_den;
            match l.key.as_str() {
                "est" => Ok(closed(
                    statistic,
                    "mh_rr",
                    finite(&[("num", p.mh_num), ("den", p.mh_den)]),
                    est,
                )),
                "lower" | "upper" => Ok(closed(
                    statistic,
                    if l.key == "lower" {
                        "wald_exp_lower"
                    } else {
                        "wald_exp_upper"
                    },
                    finite(&[("est", mh), ("se", p.se_mh), ("z", zz)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "direct_rd" => match l.key.as_str() {
            "est" => Ok(closed(
                statistic,
                "direct_adjust",
                finite(&[("adj", p.adj_rd)]),
                est,
            )),
            "lower" | "upper" => {
                let se = 1.0 / libm::sqrt(p.sw_rd);
                Ok(closed(
                    statistic,
                    if l.key == "lower" {
                        "wald_lower"
                    } else {
                        "wald_upper"
                    },
                    finite(&[("est", p.adj_rd), ("se", se), ("z", zz)]),
                    est,
                ))
            }
            _ => Err(u()),
        },
        [f, l] if f.key == "breslow_day" => {
            // Per-stratum estimates and weights, as the parts sum
            // them — `woolf_chisq` over `e_i`/`w_i` around `adj`.
            let (measure, tail) = match l.key.as_str() {
                "rr_chisq" => ("rr", "chisq"),
                "rr_p" => ("rr", "p"),
                "rd_chisq" => ("rd", "chisq"),
                "rd_p" => ("rd", "p"),
                _ => return Err(u()),
            };
            let (adj, chi) = if measure == "rr" {
                (p.adj_ln_rr, p.chi_rr)
            } else {
                (p.adj_rd, p.chi_rd)
            };
            match tail {
                "chisq" => {
                    let mut v = Vals::new();
                    v.insert("adj".into(), adj);
                    for (i, t) in rows.iter().enumerate() {
                        let (a, b, n1, n0) = (t[0], t[1], t[2], t[3]);
                        let m1 = a + b;
                        let (e_i, w_i) = if measure == "rr" {
                            (libm::log((a / n1) / (b / n0)), a * b / m1)
                        } else {
                            (a / n1 - b / n0, 1.0 / (a / (n1 * n1) + b / (n0 * n0)))
                        };
                        if e_i.is_finite() {
                            v.insert(format!("e_{i}"), e_i);
                        }
                        if w_i.is_finite() {
                            v.insert(format!("w_{i}"), w_i);
                        }
                    }
                    Ok(closed(statistic, "woolf_chisq", v, est))
                }
                _ => Ok(closed(
                    statistic,
                    "chi_sf",
                    finite(&[("x", chi), ("df", k - 1.0)]),
                    est,
                )),
            }
        }
        _ => Err(u()),
    }
}
