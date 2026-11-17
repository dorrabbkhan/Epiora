//! `means` explainer (v1.0 Task 10, spec §6). The stats object is
//! `stats_json(result, row_count)`; group descriptives use the same
//! `group_numeric` + `summarize` the run performs, the t-tests
//! `t_parts`, ANOVA recomputed from the group summaries, Bartlett
//! `bartlett_parts`, Kruskal–Wallis `kw_parts` — every `values`
//! entry an intermediate the run computed.

use super::path::Segment;
use super::steps::{closed, leaf, vals};
use super::{options, Explanation};
use crate::dataset::Dataset;
use crate::error::EngineError;
use crate::procedures::{means, MeansOptions, RunContext};
use crate::stats::{
    anova_parts, bartlett_parts, kw_parts, summarize, t_parts, Estimate, GroupStats, GroupSummary,
};
use crate::tabulate::group_numeric;
use serde_json::Value;

fn ctx() -> RunContext {
    RunContext {
        timestamp: super::EXPLAIN_TS.to_string(),
        project: None,
    }
}

fn finite(pairs: &[(&str, f64)]) -> std::collections::BTreeMap<String, f64> {
    pairs
        .iter()
        .filter(|(_, v)| v.is_finite())
        .map(|(k, v)| (k.to_string(), *v))
        .collect()
}

/// The descriptive leaves of one `GroupStats`/`overall` block —
/// `rest` is the single leaf segment after `groups[i]`/`overall`.
fn descriptives(
    g: &GroupStats,
    summary: &GroupSummary,
    statistic: &str,
    leaf_seg: &Segment,
    est: Estimate,
) -> Result<Explanation, EngineError> {
    let u = || super::path::unknown(statistic);
    let n = summary.n as f64;
    // Σ(x − mean)² — the sample-variance numerator `summarize`
    // accumulates.
    let ssq = summary.ssq();
    let (fid, v) = match leaf_seg.key.as_str() {
        "observations" => ("data", vals(&[("value", n)])),
        "total" => (
            "data",
            match g.total {
                Estimate::Value(x) => vals(&[("value", x)]),
                Estimate::Undefined { .. } => vals(&[]),
            },
        ),
        "mean" => ("mean", finite(&[("total", summary.total()), ("n", n)])),
        "variance" => ("sample_variance", finite(&[("ssq", ssq), ("df", n - 1.0)])),
        "std_dev" => (
            "sqrt",
            finite(&[("x", summary.variance.unwrap_or(f64::NAN))]),
        ),
        "min" | "p25" | "median" | "p75" | "max" | "mode" => (
            "data",
            match est.value() {
                Some(x) => vals(&[("value", x)]),
                None => vals(&[]),
            },
        ),
        _ => return Err(u()),
    };
    Ok(closed(statistic, fid, v, est))
}

/// `t_test.{pooled,welch}.*` leaves.
fn t_leaf(
    which: &str,
    parts: Option<&crate::stats::TParts>,
    summaries: (&GroupSummary, &GroupSummary),
    statistic: &str,
    rest: &[Segment],
    est: Estimate,
) -> Result<Explanation, EngineError> {
    let u = || super::path::unknown(statistic);
    let (a, b) = summaries;
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
    let crit = crate::special::t_quantile(0.975, df).unwrap_or(f64::NAN);

    match rest {
        [l] if l.key == "t" => {
            let mut v = finite(&[("diff", diff), ("se", se)]);
            if let Some(p) = parts.filter(|_| which == "pooled") {
                v.insert("var_pooled".into(), p.var_pooled);
            }
            Ok(closed(statistic, "t_stat", v, est))
        }
        [l] if l.key == "df" => {
            let (fid, v) = if which == "pooled" {
                (
                    "df_pooled",
                    finite(&[("n1", a.n as f64), ("n2", b.n as f64)]),
                )
            } else {
                (
                    "df_welch",
                    match parts {
                        Some(p) => vals(&[
                            ("se_welch", p.se_welch),
                            ("v1", a.variance.unwrap_or(f64::NAN)),
                            ("v2", b.variance.unwrap_or(f64::NAN)),
                            ("n1", a.n as f64),
                            ("n2", b.n as f64),
                        ]),
                        None => vals(&[]),
                    },
                )
            };
            Ok(closed(statistic, fid, v, est))
        }
        [l] if l.key == "p_value" => Ok(closed(
            statistic,
            "t_sf_2t",
            finite(&[("t", t), ("df", df)]),
            est,
        )),
        [l] if l.key == "diff" => Ok(closed(
            statistic,
            "mean_diff",
            finite(&[("m1", a.mean), ("m2", b.mean)]),
            est,
        )),
        [ci, side] if ci.key == "diff_ci_95" => {
            let fid = match side.key.as_str() {
                "lower" => "t_ci_lower",
                "upper" => "t_ci_upper",
                _ => return Err(u()),
            };
            Ok(closed(
                statistic,
                fid,
                finite(&[("diff", diff), ("crit", crit), ("se", se)]),
                est,
            ))
        }
        _ => Err(u()),
    }
}

/// `anova.*` leaves — the same `AnovaParts` `anova_from_summary`
/// computes from the `GroupSummary`s.
pub(crate) fn anova_leaf(
    groups: &[GroupSummary],
    statistic: &str,
    rest: &[Segment],
    est: Estimate,
) -> Result<Explanation, EngineError> {
    let u = || super::path::unknown(statistic);
    let k = groups.len() as f64;
    let p = anova_parts(groups);
    let n_tot = p.n_tot;
    let grand = p.grand;
    let ss_b = p.ss_b;
    let ss_w = p.ss_w.unwrap_or(f64::NAN);
    let (df_b, df_w) = (p.df_b, p.df_w);
    let (ms_b, ms_w) = (p.ms_b(), p.ms_w());

    let group_vals = |prefix: &str, f: &dyn Fn(&GroupSummary) -> f64| {
        let mut v = std::collections::BTreeMap::new();
        for (i, g) in groups.iter().enumerate() {
            v.insert(format!("{prefix}_{i}"), f(g));
        }
        v
    };

    match rest {
        [l] => {
            let (fid, v) = match l.key.as_str() {
                "ss_between" => {
                    let mut v = group_vals("n", &|g| g.n as f64);
                    for (i, g) in groups.iter().enumerate() {
                        v.insert(format!("mean_{i}"), g.mean);
                    }
                    v.insert("grand".into(), grand);
                    (
                        "ss_between",
                        finite(&v.iter().map(|(k, v)| (k.as_str(), *v)).collect::<Vec<_>>()),
                    )
                }
                "ss_within" => {
                    let mut v = group_vals("df", &|g| (g.n - 1) as f64);
                    for (i, g) in groups.iter().enumerate() {
                        if let Some(var) = g.variance {
                            v.insert(format!("var_{i}"), var);
                        }
                    }
                    (
                        "ss_within",
                        finite(&v.iter().map(|(k, v)| (k.as_str(), *v)).collect::<Vec<_>>()),
                    )
                }
                "ss_total" => ("sum_two", finite(&[("a", ss_b), ("b", ss_w)])),
                "df_between" => ("df_between", finite(&[("k", k)])),
                "df_within" => ("df_within", finite(&[("n", n_tot), ("k", k)])),
                "df_total" => ("df_n", finite(&[("n", n_tot)])),
                "ms_between" => ("ratio", finite(&[("num", ss_b), ("den", df_b)])),
                "ms_within" => ("ratio", finite(&[("num", ss_w), ("den", df_w)])),
                "f" => ("ratio", finite(&[("num", ms_b), ("den", ms_w)])),
                // `means` emits `p_value`; `anova_summary` emits `p`.
                "p_value" | "p" => (
                    "f_sf",
                    finite(&[("f", p.f_ratio()), ("df1", df_b), ("df2", df_w)]),
                ),
                _ => return Err(u()),
            };
            Ok(closed(statistic, fid, v, est))
        }
        _ => Err(u()),
    }
}

pub(super) fn explain(
    ds: &Dataset,
    opts_json: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let opts: MeansOptions = options(opts_json)?;
    let out = means(ds, &opts, &ctx())?;
    let tree = crate::procedures::stats_json(&out.result, ds.row_count() as u64)?;
    let est = leaf(&tree, segs, statistic)?;
    let u = || super::path::unknown(statistic);

    let grouped = group_numeric(
        ds,
        &opts.variable,
        opts.group_by.as_deref(),
        opts.include_missing,
    )?;
    let summaries: Vec<GroupSummary> = grouped.values.iter().map(|v| summarize(v)).collect();

    match segs {
        [s] if s.key == "total" => Ok(closed(
            statistic,
            "data",
            vals(&[("value", ds.row_count() as f64)]),
            est,
        )),
        [s] if s.key == "missing" => Ok(closed(
            statistic,
            "data",
            vals(&[(
                "value",
                ds.row_count() as f64 - out.result.overall.observations as f64,
            )]),
            est,
        )),
        [g, l] if g.key == "groups" => {
            let i = g.index.ok_or_else(u)?;
            let gs = out.result.groups.get(i).ok_or_else(u)?;
            let s = summaries.get(i).ok_or_else(u)?;
            descriptives(gs, s, statistic, l, est)
        }
        [g, l] if g.key == "overall" => {
            let all_summary = summarize(&grouped.all);
            descriptives(&out.result.overall, &all_summary, statistic, l, est)
        }
        [t, which, rest @ ..] if t.key == "t_test" => {
            let p = if summaries.len() == 2 {
                t_parts(&summaries[0], &summaries[1])
            } else {
                None
            };
            if summaries.len() == 2 {
                t_leaf(
                    &which.key,
                    p.as_ref(),
                    (&summaries[0], &summaries[1]),
                    statistic,
                    rest,
                    est,
                )
            } else {
                Err(u())
            }
        }
        [a, rest @ ..] if a.key == "anova" => anova_leaf(&summaries, statistic, rest, est),
        [b, l] if b.key == "bartlett" => {
            let parts = bartlett_parts(&summaries);
            match l.key.as_str() {
                "statistic" => {
                    let (num, corr) = parts
                        .as_ref()
                        .map(|p| (p.numerator, p.correction))
                        .unwrap_or((f64::NAN, f64::NAN));
                    Ok(closed(
                        statistic,
                        "bartlett",
                        finite(&[("numerator", num), ("correction", corr)]),
                        est,
                    ))
                }
                "df" => Ok(closed(
                    statistic,
                    "df_between",
                    finite(&[("k", summaries.len() as f64)]),
                    est,
                )),
                "p_value" => {
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
        [k, l] if k.key == "kruskal_wallis" => {
            let refs: Vec<&[f64]> = grouped.values.iter().map(Vec::as_slice).collect();
            let p = kw_parts(&refs);
            match l.key.as_str() {
                "h" => Ok(closed(
                    statistic,
                    "kruskal_wallis",
                    finite(&[("h_raw", p.h_raw), ("correction", p.correction)]),
                    est,
                )),
                "df" => Ok(closed(
                    statistic,
                    "df_between",
                    finite(&[("k", summaries.len() as f64)]),
                    est,
                )),
                "p_value" => Ok(closed(
                    statistic,
                    "chi_sf",
                    finite(&[("x", p.h()), ("df", p.df)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        _ => Err(u()),
    }
}
