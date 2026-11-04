//! `tables` — Epi Info's TABLES (spec §4.1): a 2×2 when both variables
//! have exactly two levels (crude plus optional stratified
//! Mantel–Haenszel), otherwise an R×C Pearson chi-square table.

use super::{build_receipt, check_ctx, options_json};
use super::{Labels2x2, ProcOutput, RunContext, StratifiedResult, TablesOptions, TablesResult};
use crate::dataset::Dataset;
use crate::stats::{rxc, stratified, two_by_two, Table2x2};
use crate::tabulate::cross;
use crate::EngineError;

/// Runs TABLES (spec §4.1). Stratification is defined only for the
/// 2×2 result — an R×C `stratify_by` is `InvalidOptions` rather than a
/// silently pooled table.
pub fn tables(
    ds: &Dataset,
    opts: &TablesOptions,
    ctx: &RunContext,
) -> Result<ProcOutput<TablesResult>, EngineError> {
    check_ctx(ctx)?;
    let xt = cross(
        ds,
        &opts.exposure,
        &opts.outcome,
        &opts.stratify_by,
        opts.include_missing,
    )?;

    let pooled = |counts: &[Vec<Vec<u64>>]| -> Vec<Vec<u64>> {
        let mut sum = vec![vec![0_u64; xt.col_labels.len()]; xt.row_labels.len()];
        for t in counts {
            for (r, row) in t.iter().enumerate() {
                for (c, &n) in row.iter().enumerate() {
                    sum[r][c] = sum[r][c].saturating_add(n);
                }
            }
        }
        sum
    };

    let is_2x2 = xt.row_labels.len() == 2 && xt.col_labels.len() == 2;
    let (result, stats): (TablesResult, Vec<(&'static str, serde_json::Value)>) = if is_2x2 {
        let to_2x2 = |t: &Vec<Vec<u64>>| Table2x2 {
            a: t[0][0],
            b: t[0][1],
            c: t[1][0],
            d: t[1][1],
        };
        let tables2x2: Vec<Table2x2> = xt.counts.iter().map(to_2x2).collect();
        let sum = tables2x2.iter().fold(
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
        );
        let stratified = if opts.stratify_by.is_empty() {
            None
        } else {
            Some(StratifiedResult {
                strata_labels: xt.strata_labels.clone(),
                stats: stratified(&tables2x2),
            })
        };
        // `stratified` sums only non-empty strata, but an empty stratum
        // adds only zeros, so its `crude` is `two_by_two` on the same
        // pooled table — reuse it instead of computing the 2×2
        // statistics (exact-method roots included) a second time.
        let crude = match &stratified {
            Some(s) => s.stats.crude.clone(),
            None => two_by_two(&sum),
        };
        let mut stats = vec![("two_by_two", serde_json::to_value(&crude).map_err(ser_err)?)];
        if let Some(s) = &stratified {
            stats.push((
                "stratified_two_by_two",
                serde_json::to_value(&s.stats).map_err(ser_err)?,
            ));
        }
        (
            TablesResult::TwoByTwo {
                labels: Labels2x2 {
                    exposure: xt.row_labels.clone(),
                    outcome: xt.col_labels.clone(),
                },
                crude,
                stratified,
            },
            stats,
        )
    } else {
        if !opts.stratify_by.is_empty() {
            return Err(EngineError::InvalidOptions {
                reason: "stratify_by requires a two-by-two table".into(),
            });
        }
        let counts = pooled(&xt.counts);
        let stats = rxc(&counts);
        (
            TablesResult::RxC {
                row_labels: xt.row_labels.clone(),
                col_labels: xt.col_labels.clone(),
                counts,
                stats: stats.clone(),
            },
            vec![("rxc", serde_json::to_value(&stats).map_err(ser_err)?)],
        )
    };

    let receipt = build_receipt(
        ds,
        "tables",
        options_json(opts)?,
        &stats,
        xt.exclusions,
        ctx,
    );
    Ok(ProcOutput { result, receipt })
}

fn ser_err(e: serde_json::Error) -> EngineError {
    EngineError::InvalidOptions {
        reason: format!("result does not serialize: {e}"),
    }
}
