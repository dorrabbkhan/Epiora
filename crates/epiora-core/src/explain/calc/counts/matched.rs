//! `matched_case_control` — pair-table leaves and the matched-N
//! design.
use super::*;
use crate::calc::counts::matched_case_control::matched_n_parts;

/// The pair block (`controls_per_case == 1`): McNemar, the
/// discordant-pairs OR and the binomial exacts on `(db, n)`.
pub(crate) fn pair_leaf(
    db: u64,
    dc: u64,
    conf: f64,
    statistic: &str,
    segs: &[Segment],
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let n = db + dc;
    let dbcf = db as f64;
    let dcc = dc as f64;
    let nf = n as f64;
    match segs {
        [f, l] if f.key == "mcnemar" || f.key == "mcnemar_cc" => {
            let chi = if n == 0 {
                f64::NAN
            } else if f.key == "mcnemar" {
                let d = dbcf - dcc;
                d * d / nf
            } else {
                let d = if db == dc {
                    0.0
                } else {
                    (dbcf - dcc).abs() - 1.0
                };
                d * d / nf
            };
            let fid = if f.key == "mcnemar" {
                "mcnemar"
            } else {
                "mcnemar_cc"
            };
            match l.key.as_str() {
                "value" => Ok(closed(
                    statistic,
                    fid,
                    vals(&[("db", dbcf), ("dc", dcc)]),
                    est,
                )),
                "p" => Ok(closed(
                    statistic,
                    "chi_sf",
                    finite(&[("x", chi), ("df", 1.0)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "or" => match l.key.as_str() {
            "est" => Ok(closed(
                statistic,
                "or_pairs",
                vals(&[("db", dbcf), ("dc", dcc)]),
                est,
            )),
            "lower" | "upper" => {
                let or = dbcf / dcc;
                let se = libm::sqrt(1.0 / dbcf + 1.0 / dcc);
                wald_exp_block(or, se, conf, statistic, &l.key, est)
            }
            _ => Err(u()),
        },
        [f, m, l] if f.key == "cmle_or" => {
            let v = vals(&[("db", dbcf), ("n", nf), ("conf", conf)]);
            match (m.key.as_str(), l.key.as_str()) {
                ("midp", "est") => Ok(method(statistic, "method.matched_or", v, est)),
                ("midp", "lower") | ("midp", "upper") => {
                    Ok(method(statistic, "method.matched_midp", v, est))
                }
                ("fisher", "lower") | ("fisher", "upper") => {
                    Ok(method(statistic, "method.matched_fisher", v, est))
                }
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "exact" => {
            let v = vals(&[("db", dbcf), ("n", nf)]);
            match l.key.as_str() {
                "midp_p" => Ok(method(statistic, "method.matched_midp_p", v, est)),
                "fisher_p_1t" => Ok(method(statistic, "method.matched_fisher_1t", v, est)),
                "fisher_p_2t" => Ok(method(statistic, "method.matched_fisher_2t", v, est)),
                _ => Err(u()),
            }
        }
        [f] if f.key == "discordant_pairs" => Ok(closed(
            statistic,
            "discordant_pairs",
            vals(&[("db", dbcf), ("dc", dcc)]),
            est,
        )),
        _ => Err(u()),
    }
}

/// The 1:n block (`controls_per_case ≥ 2`): the multiplicity-
/// weighted MH sums of `matched_n_parts`.
pub(crate) fn matched_n_leaf(
    t: &[Vec<u64>],
    m: u64,
    conf: f64,
    statistic: &str,
    segs: &[Segment],
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let p = matched_n_parts(t, m);
    let _zz = z(conf);
    let sd = libm::sqrt(
        p.rg_a / (2.0 * p.sd_ * p.sd_)
            + p.rg_b / (2.0 * p.sd_ * p.so_)
            + p.rg_c / (2.0 * p.so_ * p.so_),
    );

    // The `matched_sets` cells — a method step's inputs.
    let mut cells = Vals::new();
    for (r, row) in t.iter().enumerate().take(2) {
        for (k, &mult) in row.iter().enumerate() {
            cells.insert(format!("m{r}_{k}"), mult as f64);
        }
    }
    cells.insert("controls_per_case".into(), m as f64);
    cells.insert("conf".into(), conf);

    match segs {
        [f, blk, rest @ ..] if f.key == "matched_n" => match blk.key.as_str() {
            "mh_or" => match rest {
                [l] if l.key == "est" => Ok(closed(
                    statistic,
                    "mh_or",
                    finite(&[("num", p.sd_), ("den", p.so_)]),
                    est,
                )),
                [l] => wald_exp_block(p.sd_ / p.so_, sd, conf, statistic, &l.key, est),
                _ => Err(u()),
            },
            "mh_chisq" | "mh_chisq_cc" => {
                let chi = if blk.key == "mh_chisq" {
                    p.delta * p.delta / p.var
                } else {
                    let s = (p.delta.abs() - 0.5).max(0.0);
                    s * s / p.var
                };
                let fid = if blk.key == "mh_chisq" {
                    "mh_chi_square"
                } else {
                    "mh_chi_square_cc"
                };
                match rest {
                    [l] if l.key == "value" => Ok(closed(
                        statistic,
                        fid,
                        finite(&[("delta", p.delta), ("var", p.var)]),
                        est,
                    )),
                    [l] if l.key == "p" => Ok(closed(
                        statistic,
                        "chi_sf",
                        finite(&[("x", chi), ("df", 1.0)]),
                        est,
                    )),
                    _ => Err(u()),
                }
            }
            "cmle_or" => match rest {
                [l] if l.key == "est" || l.key == "lower" || l.key == "upper" => {
                    Ok(method(statistic, "method.clogit", cells, est))
                }
                _ => Err(u()),
            },
            _ => Err(u()),
        },
        _ => Err(u()),
    }
}

pub(crate) fn matched_case_control(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let m = cnt(inputs, "controls_per_case");
    let conf = conf(inputs);
    let t = count_table(inputs, "matched_sets");
    match segs {
        [f, ..] if f.key == "matched_n" => {
            if t.len() != 2 || t.iter().any(|r| r.len() != (m + 1) as usize) {
                return Err(u());
            }
            matched_n_leaf(&t, m, conf, statistic, segs, est)
        }
        _ => {
            if m != 1 || t.len() < 2 || t[0].len() < 2 || t[1].is_empty() {
                return Err(u());
            }
            pair_leaf(t[0][1], t[1][0], conf, statistic, segs, est)
        }
    }
}
