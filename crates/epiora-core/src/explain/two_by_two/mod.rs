//! `tables` explainers (v1.0 Task 10, spec §6): the `two_by_two`,
//! `stratified_two_by_two` and `rxc` families. The tables are
//! re-derived through the same `cross` + `to_2x2`/`pooled` code the
//! procedure runs, and every `values` entry is an intermediate the
//! run computed — so a trace's last step is the run's leaf by
//! construction. Exact/iterative leaves are one-step `Method` traces
//! whose `values` are the method's inputs.

use super::path::Segment;
use super::steps::{closed, leaf, method, vals};
use super::{options, Explanation};
use crate::dataset::Dataset;
use crate::error::EngineError;
use crate::procedures::{tables, RunContext, TablesOptions, TablesResult};
use crate::stats::{two_by_two_parts, Table2x2, TwoByTwoStats};
use crate::tabulate::cross;
use serde_json::Value;

mod rxc;
mod stratified;

fn ctx() -> RunContext {
    RunContext {
        timestamp: super::EXPLAIN_TS.to_string(),
        project: None,
        working_view: None,
    }
}

/// Sum of a table's cells — the kept-strata test (`stats::stratified`
/// keeps strata whose total is positive).
fn total(t: &Table2x2) -> u64 {
    t.a.saturating_add(t.b)
        .saturating_add(t.c)
        .saturating_add(t.d)
}

/// The pooled 2×2 of `strata` — the fold the explainers applied to
/// `strata_of`'s output.
fn pooled(strata: &[Table2x2]) -> Table2x2 {
    strata.iter().fold(
        Table2x2 {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
        },
        |s, t| Table2x2 {
            a: s.a.saturating_add(t.a),
            b: s.b.saturating_add(t.b),
            c: s.c.saturating_add(t.c),
            d: s.d.saturating_add(t.d),
        },
    )
}

/// The run half of a `tables` explain (v1.0 Task 10 fix round 3):
/// the procedure result plus the strata-derived tables the
/// `two_by_two`/`stratified_two_by_two` traces read — the pooled
/// table and the kept strata (totals > 0, as `stats::stratified`
/// filters them). Both are computed only for a `TwoByTwo` result:
/// `strata_of` re-runs the run's own `cross`, which such a result
/// guarantees is 2×2.
pub(crate) struct TablesRun {
    result: TablesResult,
    sum: Table2x2,
    kept: Vec<Table2x2>,
}

/// `options` + `tables` + the `cross` re-derivation once — the
/// public `explain`/`explain_stratified`/`explain_rxc` and the
/// corpus tests' run-once path share this.
pub(crate) fn run(ds: &Dataset, opts_json: &Value) -> Result<TablesRun, EngineError> {
    let opts: TablesOptions = options(opts_json)?;
    let out = tables(ds, &opts, &ctx())?;
    let (sum, kept) = match &out.result {
        TablesResult::TwoByTwo { .. } => {
            let strata = strata_of(ds, &opts)?;
            let kept = strata.iter().copied().filter(|t| total(t) > 0).collect();
            (pooled(&strata), kept)
        }
        _ => (
            Table2x2 {
                a: 0,
                b: 0,
                c: 0,
                d: 0,
            },
            Vec::new(),
        ),
    };
    Ok(TablesRun {
        result: out.result,
        sum,
        kept,
    })
}

/// A `vals` map with the non-finite intermediates dropped — an
/// `Undefined` leaf never carries a NaN/inf into `values`.
fn finite(pairs: &[(&str, f64)]) -> std::collections::BTreeMap<String, f64> {
    pairs
        .iter()
        .filter(|(_, v)| v.is_finite())
        .map(|(k, v)| (k.to_string(), *v))
        .collect()
}

/// Re-derives the 2×2 strata the `tables` run tabulated — the same
/// `cross` + fold it performs.
fn strata_of(ds: &Dataset, opts: &TablesOptions) -> Result<Vec<Table2x2>, EngineError> {
    let xt = cross(
        ds,
        &opts.exposure,
        &opts.outcome,
        &opts.stratify_by,
        opts.include_missing,
    )?;
    if xt.row_labels.len() != 2 || xt.col_labels.len() != 2 {
        return Err(EngineError::InvalidOptions {
            reason: "not a two-by-two table".into(),
        });
    }
    Ok(xt
        .counts
        .iter()
        .map(|t| Table2x2 {
            a: t[0][0],
            b: t[0][1],
            c: t[1][0],
            d: t[1][1],
        })
        .collect())
}
fn leaf_2x2(
    t: &Table2x2,
    stats: &TwoByTwoStats,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    let tree = serde_json::to_value(stats).map_err(|e| EngineError::InvalidOptions {
        reason: format!("result does not serialize: {e}"),
    })?;
    let est = leaf(&tree, segs, statistic)?;
    let u = || super::path::unknown(statistic);
    let p = two_by_two_parts(t);
    let cells = vals(&[
        ("a", t.a as f64),
        ("b", t.b as f64),
        ("c", t.c as f64),
        ("d", t.d as f64),
    ]);
    let oe = || {
        vals(&[
            ("o_a", t.a as f64),
            ("o_b", t.b as f64),
            ("o_c", t.c as f64),
            ("o_d", t.d as f64),
            ("e_a", p.e[0]),
            ("e_b", p.e[1]),
            ("e_c", p.e[2]),
            ("e_d", p.e[3]),
        ])
    };
    let wald = |est_v: f64, se: f64| finite(&[("est", est_v), ("se", se), ("z", p.z)]);

    match segs {
        [s] if s.key == "risk" => Err(u()),
        [f, l] if f.key == "risk" => match l.key.as_str() {
            "exposed" => Ok(closed(
                statistic,
                "proportion",
                finite(&[("x", t.a as f64), ("n", p.n1)]),
                est,
            )),
            "unexposed" => Ok(closed(
                statistic,
                "proportion",
                finite(&[("x", t.c as f64), ("n", p.n0)]),
                est,
            )),
            "difference" => Ok(closed(
                statistic,
                "risk_difference",
                finite(&[("p1", p.pe), ("p0", p.pu)]),
                est,
            )),
            _ => Err(u()),
        },
        [f, ci, side] if f.key == "risk" && ci.key == "difference_ci_95_wald" => {
            let f_id = match side.key.as_str() {
                "lower" => "wald_lower",
                "upper" => "wald_upper",
                _ => return Err(u()),
            };
            Ok(closed(statistic, f_id, wald(p.rd, p.se_rd), est))
        }
        [f, l] if f.key == "risk_ratio" => match l.key.as_str() {
            "estimate" => Ok(closed(
                statistic,
                "risk_ratio",
                finite(&[("p1", p.pe), ("p0", p.pu)]),
                est,
            )),
            _ => Err(u()),
        },
        [f, ci, side] if f.key == "risk_ratio" && ci.key == "ci_95_wald" => {
            let f_id = match side.key.as_str() {
                "lower" => "wald_exp_lower",
                "upper" => "wald_exp_upper",
                _ => return Err(u()),
            };
            Ok(closed(statistic, f_id, wald(p.rr, p.se_rr), est))
        }
        [f, ss, l] if f.key == "risk_ratio" && ss.key == "small_sample" => match l.key.as_str() {
            "estimate" => Ok(closed(
                statistic,
                "rr_small",
                finite(&[("pe", p.pe), ("c", t.c as f64), ("n0", p.n0)]),
                est,
            )),
            "lower" => Ok(closed(
                statistic,
                "wald_exp_lower",
                wald(p.rr_small, p.se_rr),
                est,
            )),
            "upper" => Ok(closed(
                statistic,
                "wald_exp_upper",
                wald(p.rr_small, p.se_rr),
                est,
            )),
            _ => Err(u()),
        },
        [f, l] if f.key == "odds_ratio" => match l.key.as_str() {
            "estimate" => Ok(closed(statistic, "odds_ratio", cells.clone(), est)),
            "cmle" => {
                let mut v = cells.clone();
                v.insert("conf".into(), 95.0);
                Ok(method(statistic, "method.cmle", v, est))
            }
            _ => Err(u()),
        },
        [f, ci, side] if f.key == "odds_ratio" => {
            let side_l = match side.key.as_str() {
                "lower" | "upper" => side.key.as_str(),
                _ => return Err(u()),
            };
            match ci.key.as_str() {
                "ci_95_wald" => {
                    let f_id = if side_l == "lower" {
                        "wald_exp_lower"
                    } else {
                        "wald_exp_upper"
                    };
                    Ok(closed(statistic, f_id, wald(p.or, p.se_or), est))
                }
                "ci_95_fisher_exact" => Ok(method(
                    statistic,
                    "method.fisher_ci",
                    vals(&[
                        ("a", t.a as f64),
                        ("b", t.b as f64),
                        ("c", t.c as f64),
                        ("d", t.d as f64),
                        ("conf", 95.0),
                    ]),
                    est,
                )),
                "ci_95_midp_exact" => Ok(method(
                    statistic,
                    "method.midp_ci",
                    vals(&[
                        ("a", t.a as f64),
                        ("b", t.b as f64),
                        ("c", t.c as f64),
                        ("d", t.d as f64),
                        ("conf", 95.0),
                    ]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "chi_square" && l.key == "df" => Ok(closed(
            statistic,
            "chi_df",
            finite(&[("rows", 2.0), ("cols", 2.0)]),
            est,
        )),
        [f, which, l] if f.key == "chi_square" => {
            let (x, v) = match which.key.as_str() {
                "uncorrected" => (p.chi_uncorrected, oe()),
                "yates" => {
                    let mut v = oe();
                    v.insert("yates_adj".into(), p.yates_adj);
                    (p.chi_yates, v)
                }
                "mantel_haenszel" => (
                    p.chi_mh,
                    finite(&[
                        ("delta", p.delta),
                        ("n", p.n),
                        ("n1", p.n1),
                        ("n0", p.n0),
                        ("m1", p.m1),
                        ("m0", p.m0),
                    ]),
                ),
                _ => return Err(u()),
            };
            let fid = match (which.key.as_str(), l.key.as_str()) {
                ("uncorrected", "statistic") => "chi_square_cells",
                ("yates", "statistic") => "chi_square_yates",
                ("mantel_haenszel", "statistic") => "chi_square_mh_delta",
                (_, "p_value") => "chi_sf",
                _ => return Err(u()),
            };
            let v = if l.key == "p_value" {
                finite(&[("x", x), ("df", 1.0)])
            } else {
                v
            };
            Ok(closed(statistic, fid, v, est))
        }
        [f, l] if f.key == "fisher_exact" => match l.key.as_str() {
            "p_lower" | "p_upper" | "p_two_sided" => Ok(method(
                statistic,
                "method.fisher_exact_p",
                cells.clone(),
                est,
            )),
            _ => Err(u()),
        },
        [f, l] if f.key == "midp_exact" && l.key == "p_two_sided" => {
            Ok(method(statistic, "method.midp_p", cells.clone(), est))
        }
        _ => Err(u()),
    }
}

impl TablesRun {
    /// `two_by_two.*` — the crude statistics of the pooled table.
    pub(crate) fn trace(
        &self,
        statistic: &str,
        segs: &[Segment],
    ) -> Result<Explanation, EngineError> {
        let crude = match &self.result {
            TablesResult::TwoByTwo { crude, .. } => crude,
            _ => return Err(super::path::unknown(statistic)),
        };
        leaf_2x2(&self.sum, crude, statistic, segs)
    }

    /// `stratified_two_by_two.*` — crude, excluded strata, MH and
    /// Breslow–Day–Tarone leaves.
    pub(crate) fn trace_stratified(
        &self,
        statistic: &str,
        segs: &[Segment],
    ) -> Result<Explanation, EngineError> {
        let strat = match &self.result {
            TablesResult::TwoByTwo {
                stratified: Some(s),
                ..
            } => s,
            _ => return Err(super::path::unknown(statistic)),
        };
        let u = || super::path::unknown(statistic);

        match segs.split_first() {
            Some((head, rest)) if head.key == "crude" => {
                leaf_2x2(&self.sum, &strat.stats.crude, statistic, rest)
            }
            Some((head, _rest)) if head.key == "excluded_strata" => {
                let tree = serde_json::to_value(&strat.stats).map_err(|e| {
                    EngineError::InvalidOptions {
                        reason: format!("result does not serialize: {e}"),
                    }
                })?;
                let est = leaf(&tree, segs, statistic)?;
                let idx = head.index.ok_or_else(u)?;
                let v = strat
                    .stats
                    .excluded_strata
                    .get(idx)
                    .map(|i| *i as f64)
                    .ok_or_else(u)?;
                Ok(closed(statistic, "data", vals(&[("value", v)]), est))
            }
            Some((head, rest)) if head.key == "mantel_haenszel" => {
                stratified::mh_leaf(&strat.stats, &self.kept, statistic, rest)
            }
            Some((head, rest)) if head.key == "breslow_day_tarone" => {
                stratified::bdt_leaf(&strat.stats, &self.kept, statistic, rest)
            }
            _ => Err(u()),
        }
    }

    /// `rxc.*` — the Pearson chi-square of the pooled R×C table.
    pub(crate) fn trace_rxc(
        &self,
        statistic: &str,
        segs: &[Segment],
    ) -> Result<Explanation, EngineError> {
        match &self.result {
            TablesResult::RxC { stats, counts, .. } => rxc::trace(stats, counts, statistic, segs),
            _ => Err(super::path::unknown(statistic)),
        }
    }
}
