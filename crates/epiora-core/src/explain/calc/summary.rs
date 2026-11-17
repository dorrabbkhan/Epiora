//! `summary` — the continuous-summary calculators `mean_ci`,
//! `median_ci`, `t_test_summary` and `anova_summary`. The t-tests
//! and ANOVA/Bartlett reuse `stats::means_summary`'s shared parts,
//! identical to the `means` procedure's trace.

use super::super::path::Segment;
use super::super::steps::{closed, leaf, vals};
use super::super::Explanation;
use super::finite;
use crate::calc::counts::shared::{conf, num, table, z};
use crate::error::EngineError;
use crate::stats::{anova_from_summary, bartlett_parts, t_parts, Estimate, GroupSummary};
use serde_json::Value;

fn unknown(statistic: &str) -> EngineError {
    super::super::path::unknown(statistic)
}

/// `mean_ci`'s `se` — the run's `se_parts` in
/// `calc::continuous::mean_ci`.
fn mean_ci_se(inputs: &Value) -> f64 {
    crate::calc::continuous::mean_ci::se_parts(inputs).0
}

pub(super) fn mean_ci(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let mean = num(inputs, "mean");
    let n = num(inputs, "n");
    let c = conf(inputs);
    let se = mean_ci_se(inputs);
    let tc = crate::calc::counts::shared::t_crit(c, n - 1.0);

    match segs {
        [m, l] if m.key == "z" || m.key == "t" => {
            let crit = if m.key == "z" { z(c) } else { tc };
            let (fid, key) = match (m.key.as_str(), l.key.as_str()) {
                ("z", "lower") => ("mean_ci_z_lower", "z"),
                ("z", "upper") => ("mean_ci_z_upper", "z"),
                ("t", "lower") => ("mean_ci_t_lower", "t"),
                ("t", "upper") => ("mean_ci_t_upper", "t"),
                _ => return Err(u()),
            };
            Ok(closed(
                statistic,
                fid,
                finite(&[("mean", mean), ("se", se), (key, crit)]),
                est,
            ))
        }
        _ => Err(u()),
    }
}

pub(super) fn median_ci(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let n = num(inputs, "n");
    let p = num(inputs, "percentile") / 100.0;
    let zz = z(conf(inputs));
    match segs {
        [m, l] if m.key == "normal" => {
            let fid = match l.key.as_str() {
                "lower_rank" => "median_rank_lower",
                "upper_rank" => "median_rank_upper",
                _ => return Err(u()),
            };
            Ok(closed(
                statistic,
                fid,
                finite(&[("n", n), ("p", p), ("z", zz)]),
                est,
            ))
        }
        _ => Err(u()),
    }
}

/// `t_test_summary`'s two `GroupSummary`s — the run's own
/// `groups` constructor.
fn t_groups(inputs: &Value) -> (GroupSummary, GroupSummary) {
    crate::calc::continuous::t_test_summary::groups(inputs)
}

/// One `{t, df, p_2t, diff, lower, upper}` t-test block.
fn t_block(
    which: &str,
    parts: Option<&crate::stats::TParts>,
    pair: (&GroupSummary, &GroupSummary),
    conf: f64,
    statistic: &str,
    leaf_key: &str,
    est: Estimate,
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let (a, b) = pair;
    let (se, df) = match parts {
        Some(p) if which == "pooled" => (p.se_pooled, p.df_pooled),
        Some(p) => (p.se_welch, p.df_welch),
        None => (f64::NAN, f64::NAN),
    };
    let diff = parts.map(|p| p.diff).unwrap_or(f64::NAN);
    let t = match parts {
        Some(p) if which == "pooled" => p.t_pooled,
        Some(p) => p.t_welch,
        None => f64::NAN,
    };
    let crit = crate::calc::counts::shared::t_crit(conf, df);

    let (fid, v) = match leaf_key {
        "t" => ("t_stat", finite(&[("diff", diff), ("se", se)])),
        "df" => {
            if which == "pooled" {
                (
                    "df_pooled",
                    finite(&[("n1", a.n as f64), ("n2", b.n as f64)]),
                )
            } else {
                (
                    "df_welch",
                    match parts {
                        Some(p) => finite(&[
                            ("se_welch", p.se_welch),
                            ("v1", a.variance.unwrap_or(f64::NAN)),
                            ("v2", b.variance.unwrap_or(f64::NAN)),
                            ("n1", a.n as f64),
                            ("n2", b.n as f64),
                        ]),
                        None => vals(&[]),
                    },
                )
            }
        }
        "p_2t" => ("t_sf_2t", finite(&[("t", t), ("df", df)])),
        "diff" => ("mean_diff", finite(&[("m1", a.mean), ("m2", b.mean)])),
        "lower" => (
            "t_ci_lower",
            finite(&[("diff", diff), ("crit", crit), ("se", se)]),
        ),
        "upper" => (
            "t_ci_upper",
            finite(&[("diff", diff), ("crit", crit), ("se", se)]),
        ),
        _ => return Err(u()),
    };
    Ok(closed(statistic, fid, v, est))
}

pub(super) fn t_test_summary(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let c = conf(inputs);
    let (a, b) = t_groups(inputs);
    let parts = t_parts(&a, &b);

    match segs {
        [w, l] if w.key == "pooled" || w.key == "welch" => {
            t_block(&w.key, parts.as_ref(), (&a, &b), c, statistic, &l.key, est)
        }
        [f, l] if f.key == "f_variance" => {
            let v1 = a.variance.unwrap_or(f64::NAN);
            let v2 = b.variance.unwrap_or(f64::NAN);
            match l.key.as_str() {
                "f" => Ok(closed(
                    statistic,
                    "f_stat",
                    finite(&[("v1", v1), ("v2", v2)]),
                    est,
                )),
                "df_num" => Ok(closed(
                    statistic,
                    "df_n_minus1",
                    finite(&[("n", a.n as f64)]),
                    est,
                )),
                "df_den" => Ok(closed(
                    statistic,
                    "df_n_minus1",
                    finite(&[("n", b.n as f64)]),
                    est,
                )),
                "p" => Ok(closed(
                    statistic,
                    "f_p_2t",
                    finite(&[
                        (
                            "f",
                            crate::calc::continuous::t_test_summary::f_ratio(&a, &b),
                        ),
                        ("df_num", (a.n - 1) as f64),
                        ("df_den", (b.n - 1) as f64),
                    ]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        _ => Err(u()),
    }
}

/// `anova_summary`'s `GroupSummary`s from the `table` rows — the
/// run's own `groups_of` constructor.
fn anova_groups(inputs: &Value) -> Vec<GroupSummary> {
    crate::calc::continuous::anova_summary::groups_of(&table(inputs, "table"))
}

pub(super) fn anova_summary(
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let u = || unknown(statistic);
    let est = leaf(result, segs, statistic)?;
    let groups = anova_groups(inputs);

    match segs {
        [a, rest @ ..] if a.key == "anova" => {
            super::super::means::anova_leaf(&groups, statistic, rest, est)
        }
        [b, l] if b.key == "bartlett" => {
            let parts = bartlett_parts(&groups);
            match l.key.as_str() {
                "chisq" => {
                    let (num_, corr) = parts
                        .as_ref()
                        .map(|p| (p.numerator, p.correction))
                        .unwrap_or((f64::NAN, f64::NAN));
                    Ok(closed(
                        statistic,
                        "bartlett",
                        finite(&[("numerator", num_), ("correction", corr)]),
                        est,
                    ))
                }
                "df" => Ok(closed(
                    statistic,
                    "df_between",
                    finite(&[("k", groups.len() as f64)]),
                    est,
                )),
                "p" => {
                    let x = parts.as_ref().map(|p| p.stat()).unwrap_or(f64::NAN);
                    Ok(closed(
                        statistic,
                        "chi_sf",
                        finite(&[
                            ("x", x),
                            ("df", parts.as_ref().map(|p| p.df).unwrap_or(f64::NAN)),
                        ]),
                        est,
                    ))
                }
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "group_ci" || f.key == "group_ci_pooled" => {
            let i = f.index.ok_or_else(u)?;
            let g = groups.get(i).ok_or_else(u)?;
            let an = anova_from_summary(&groups);
            let (crit, h_base) = if f.key == "group_ci" {
                let (tc, h, _) = crate::calc::continuous::anova_summary::group_ci_parts(g);
                (tc, h)
            } else {
                let ms_w = match an.ms_within {
                    Estimate::Value(v) => v,
                    Estimate::Undefined { .. } => f64::NAN,
                };
                let df_w = match an.df_within {
                    Estimate::Value(v) => v,
                    Estimate::Undefined { .. } => f64::NAN,
                };
                (
                    crate::special::t_quantile(0.975, df_w).unwrap_or(f64::NAN),
                    crate::calc::continuous::anova_summary::pooled_h_base(g, ms_w),
                )
            };
            let fid = match l.key.as_str() {
                "lower" => "group_ci_lower",
                "upper" => "group_ci_upper",
                _ => return Err(u()),
            };
            Ok(closed(
                statistic,
                fid,
                finite(&[("mean", g.mean), ("crit", crit), ("h_base", h_base)]),
                est,
            ))
        }
        _ => Err(u()),
    }
}
