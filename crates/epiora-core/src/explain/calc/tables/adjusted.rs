//! `adjusted_leaf`, the `strata[]/crude` routers, and the
//! `two_by_two`/`two_by_n` family entry points.
use super::*;
use crate::calc::counts::two_by_two as c22;

/// `adjusted.*` of `two_by_two`.
fn adjusted_leaf(
    strata: &[Table2x2],
    conf: f64,
    statistic: &str,
    segs: &[Segment],
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let p = c22::adjusted_parts(strata);
    let zz = z(conf);
    let sd = libm::sqrt(
        p.rg_a / (2.0 * p.sd_ * p.sd_)
            + p.rg_b / (2.0 * p.sd_ * p.so_)
            + p.rg_c / (2.0 * p.so_ * p.so_),
    );
    let gr_se = libm::sqrt(p.gr_num / (p.rr_num * p.rr_den));

    match segs {
        [f, l] if f.key == "mh_chisq" => match l.key.as_str() {
            "value" => Ok(closed(
                statistic,
                "mh_chi_square",
                finite(&[("delta", p.delta), ("var", p.var)]),
                est,
            )),
            "p" => Ok(closed(
                statistic,
                "chi_sf",
                finite(&[("x", p.delta * p.delta / p.var), ("df", 1.0)]),
                est,
            )),
            _ => Err(u()),
        },
        [f, l] if f.key == "mh_or" => match l.key.as_str() {
            "est" => Ok(closed(
                statistic,
                "mh_or",
                finite(&[("num", p.sd_), ("den", p.so_)]),
                est,
            )),
            _ => stat_ci_wald_exp(statistic, &l.key, p.sd_ / p.so_, sd, conf, est),
        },
        [f, l] if f.key == "mh_rr" => match l.key.as_str() {
            "est" => Ok(closed(
                statistic,
                "mh_rr",
                finite(&[("num", p.rr_num), ("den", p.rr_den)]),
                est,
            )),
            _ => stat_ci_wald_exp(statistic, &l.key, p.rr_num / p.rr_den, gr_se, conf, est),
        },
        [f, l] if f.key.starts_with("direct_") => {
            let measure = f.key.trim_start_matches("direct_");
            let (adj, _, ws) = c22::woolf(strata, measure);
            let sw: f64 = ws.iter().sum();
            let fid = match l.key.as_str() {
                "est" => {
                    if measure == "rd" {
                        "direct_adjust"
                    } else {
                        "direct_adjust_ln"
                    }
                }
                "lower" => {
                    if measure == "rd" {
                        "direct_adjust_lower"
                    } else {
                        "direct_adjust_ln_lower"
                    }
                }
                "upper" => {
                    if measure == "rd" {
                        "direct_adjust_upper"
                    } else {
                        "direct_adjust_ln_upper"
                    }
                }
                _ => return Err(u()),
            };
            let swe = adj * sw;
            Ok(closed(
                statistic,
                fid,
                finite(&[("sum_w", sw), ("sum_we", swe), ("z", zz)]),
                est,
            ))
        }
        [f, l] if f.key == "breslow_day" => {
            let (measure, tail) = l.key.split_at(l.key.len().min(2));
            let _ = tail;
            let measure = match measure {
                "or" | "rr" | "rd" => measure,
                _ => return Err(u()),
            };
            let kind = if l.key.ends_with("_chisq") {
                "chisq"
            } else if l.key.ends_with("_p") {
                "p"
            } else {
                return Err(u());
            };
            let (adj, ests, ws) = c22::woolf(strata, measure);
            let chi: f64 = ests
                .iter()
                .zip(&ws)
                .map(|(e, w)| w * (e - adj) * (e - adj))
                .sum();
            match kind {
                "chisq" => {
                    let mut v = finite(&[("adj", adj)]);
                    for (i, (e, w)) in ests.iter().zip(&ws).enumerate() {
                        if e.is_finite() {
                            v.insert(format!("e_{i}"), *e);
                        }
                        if w.is_finite() {
                            v.insert(format!("w_{i}"), *w);
                        }
                    }
                    Ok(closed(statistic, "woolf_chisq", v, est))
                }
                _ => Ok(closed(
                    statistic,
                    "chi_sf",
                    finite(&[("x", chi), ("df", strata.len() as f64 - 1.0)]),
                    est,
                )),
            }
        }
        [f, m, l] if f.key == "cmle_or" => {
            let mut v = Vals::new();
            for (i, t) in strata.iter().enumerate() {
                v.insert(format!("a_{i}"), t.a as f64);
                v.insert(format!("b_{i}"), t.b as f64);
                v.insert(format!("c_{i}"), t.c as f64);
                v.insert(format!("d_{i}"), t.d as f64);
            }
            v.insert("conf".into(), conf);
            match (m.key.as_str(), l.key.as_str()) {
                ("midp", "lower") | ("midp", "upper") => {
                    Ok(method(statistic, "method.midp_ci", v, est))
                }
                ("fisher", "lower") | ("fisher", "upper") => {
                    Ok(method(statistic, "method.fisher_ci", v, est))
                }
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "cmle_or" => match l.key.as_str() {
            "est" => {
                let mut v = Vals::new();
                for (i, t) in strata.iter().enumerate() {
                    v.insert(format!("a_{i}"), t.a as f64);
                    v.insert(format!("b_{i}"), t.b as f64);
                    v.insert(format!("c_{i}"), t.c as f64);
                    v.insert(format!("d_{i}"), t.d as f64);
                }
                v.insert("conf".into(), conf);
                Ok(method(statistic, "method.cmle", v, est))
            }
            _ => Err(u()),
        },
        _ => Err(u()),
    }
}

/// Resolves the block prefix (`strata[i]` or `crude`) to its table,
/// then dispatches the leaf.
pub(crate) fn stratum_or_crude(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
    is_2n: bool,
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let conf = conf(inputs);
    let ss = strata(inputs);
    let (head, rest) = segs.split_first().ok_or_else(u)?;
    let (t, tree) = if head.key == "crude" {
        (crude(&ss), result.get("crude"))
    } else if head.key == "strata" {
        let i = head.index.ok_or_else(u)?;
        (
            *ss.get(i).ok_or_else(u)?,
            result.get("strata").and_then(|a| a.get(i)),
        )
    } else {
        return Err(u());
    };
    let tree = tree.ok_or_else(u)?;
    let est = leaf(tree, rest, statistic)?;
    table_leaf(&t, conf, statistic, rest, est, is_2n)
}

pub(crate) fn two_by_two(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let _u = || unknown(statistic);
    let conf = conf(inputs);
    if segs.first().map(|s| s.key.as_str()) == Some("adjusted") {
        let ss = strata(inputs);
        let est = leaf(&result["adjusted"], &segs[1..], statistic)?;
        return adjusted_leaf(&ss, conf, statistic, &segs[1..], est);
    }
    stratum_or_crude(result, inputs, statistic, segs, false)
}

pub(crate) fn two_by_n(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    stratum_or_crude(result, inputs, statistic, segs, true)
}

// --- r_by_c -----------------------------------------------------------------
