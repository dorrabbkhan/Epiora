//! `freq` explainer (v1.0 Task 10, spec §6). The procedure's stats
//! object is the serialized `FreqTable`; a `freq.*` statistic is a
//! path into the first table (the registry's `freq.*` ids are
//! per-table — a multi-variable run shares the same ids per table
//! and the path does not name one, so the first is the table a
//! trace names).

use super::path::Segment;
use super::steps::{closed, leaf, method, vals};
use super::{options, Explanation};
use crate::dataset::Dataset;
use crate::error::EngineError;
use crate::procedures::{freq, FreqOptions, FreqTable, RunContext};
use crate::special::normal_quantile;
use serde_json::Value;

fn ctx() -> RunContext {
    RunContext {
        timestamp: super::EXPLAIN_TS.to_string(),
        project: None,
    }
}

/// `qnorm(0.975)` — the freq CI z, as `stats::proportion` computes.
fn z_975() -> f64 {
    normal_quantile(0.975).unwrap_or(1.959963984540054)
}

/// The run half of a `freq` explain (v1.0 Task 10 fix round 3):
/// the procedure's output kept so every `freq.*` statistic traces
/// from one run. `freq.*` paths name no table, so the first table
/// — the one a trace reads — and its serialized tree are kept.
pub(crate) struct FreqRun {
    table: Option<(FreqTable, Value)>,
}

/// `options` + `freq` once — the public `explain` and the corpus
/// tests' run-once path share this.
pub(crate) fn run(ds: &Dataset, opts_json: &Value) -> Result<FreqRun, EngineError> {
    let opts: FreqOptions = options(opts_json)?;
    let out = freq(ds, &opts, &ctx())?;
    let table = match out.result.tables.into_iter().next() {
        Some(t) => {
            let tree = serde_json::to_value(&t).map_err(|e| EngineError::InvalidOptions {
                reason: format!("result does not serialize: {e}"),
            })?;
            Some((t, tree))
        }
        None => None,
    };
    Ok(FreqRun { table })
}

impl FreqRun {
    /// The trace half — resolves `segs` against the run's first
    /// table exactly as the inline explainer did.
    pub(crate) fn trace(
        &self,
        statistic: &str,
        segs: &[Segment],
    ) -> Result<Explanation, EngineError> {
        let (table, tree) = self
            .table
            .as_ref()
            .ok_or_else(|| super::path::unknown(statistic))?;
        let est = leaf(tree, segs, statistic)?;
        let u = || super::path::unknown(statistic);

        let total = table.total as f64;
        match segs {
            [s] if s.key == "total" && s.index.is_none() => {
                Ok(closed(statistic, "data", vals(&[("value", total)]), est))
            }
            [s] if s.key == "missing" && s.index.is_none() => Ok(closed(
                statistic,
                "data",
                vals(&[("value", table.missing as f64)]),
                est,
            )),
            [cat, leaf_seg] if cat.key == "categories" => {
                let i = cat.index.ok_or_else(u)?;
                let count = table.categories.get(i).map(|c| c.count).ok_or_else(u)?;
                let k = count as f64;
                match leaf_seg.key.as_str() {
                    "count" => Ok(closed(statistic, "data", vals(&[("value", k)]), est)),
                    "percent" => Ok(closed(
                        statistic,
                        "proportion_pct",
                        vals(&[("x", k), ("n", total)]),
                        est,
                    )),
                    "cumulative_percent" => {
                        let cumulative: f64 = table
                            .categories
                            .iter()
                            .take(i + 1)
                            .map(|c| c.count as f64)
                            .sum();
                        Ok(closed(
                            statistic,
                            "cumulative_percent_pct",
                            vals(&[("cumulative", cumulative), ("total", total)]),
                            est,
                        ))
                    }
                    _ => Err(u()),
                }
            }
            [cat, ci_seg, side] if cat.key == "categories" => {
                let i = cat.index.ok_or_else(u)?;
                let k = table
                    .categories
                    .get(i)
                    .map(|c| c.count as f64)
                    .ok_or_else(u)?;
                match ci_seg.key.as_str() {
                    "ci_95_wilson" => {
                        let formula = match side.key.as_str() {
                            "lower" => "wilson_lower_pct",
                            "upper" => "wilson_upper_pct",
                            _ => return Err(u()),
                        };
                        Ok(closed(
                            statistic,
                            formula,
                            vals(&[("x", k), ("n", total), ("z", z_975())]),
                            est,
                        ))
                    }
                    "ci_95_clopper_pearson" => match side.key.as_str() {
                        "lower" | "upper" => Ok(method(
                            statistic,
                            "method.clopper_pearson",
                            vals(&[("x", k), ("n", total), ("conf", 95.0)]),
                            est,
                        )),
                        _ => Err(u()),
                    },
                    _ => Err(u()),
                }
            }
            _ => Err(u()),
        }
    }
}
