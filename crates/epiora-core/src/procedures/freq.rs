//! `freq` — Epi Info's FREQ (spec §4.1): one table per variable (per
//! stratum when `stratify_by` is set), `proportions` statistics per
//! category, exclusion counts per variable, and a receipt.

use super::{build_receipt, check_ctx, options_json};
use super::{FreqOptions, FreqResult, FreqTable, ProcOutput, RunContext};
use crate::dataset::Dataset;
use crate::stats::proportions;
use crate::tabulate::{frequency, frequency_stratified, FreqCounts};
use crate::EngineError;
use std::collections::BTreeMap;

/// Runs FREQ (spec §4.1). Tables come in variable order, each variable
/// stratified in stratum order when `stratify_by` is set; a record
/// missing the stratify variable is excluded and counted under it.
/// Records missing the analysed variable are reported in
/// `FreqTable::missing`, never excluded silently.
pub fn freq(
    ds: &Dataset,
    opts: &FreqOptions,
    ctx: &RunContext,
) -> Result<ProcOutput<FreqResult>, EngineError> {
    check_ctx(ctx)?;
    let mut tables = Vec::new();
    let mut exclusions: BTreeMap<String, u64> = opts
        .variables
        .iter()
        .chain(opts.stratify_by.iter())
        .map(|n| (n.clone(), 0))
        .collect();

    for variable in &opts.variables {
        match &opts.stratify_by {
            None => {
                let counts = frequency(ds, variable, opts.include_missing)?;
                tables.push(table(variable, None, &counts, opts.ci_method));
            }
            Some(stratum) => {
                let strat = frequency_stratified(
                    ds,
                    variable,
                    std::slice::from_ref(stratum),
                    opts.include_missing,
                )?;
                for (v, n) in &strat.exclusions {
                    *exclusions.entry(v.clone()).or_default() += n;
                }
                for (label, counts) in strat.strata_labels.iter().zip(&strat.tables) {
                    tables.push(table(variable, Some(label.clone()), counts, opts.ci_method));
                }
            }
        }
    }

    let stats: Vec<serde_json::Value> = tables
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<_, _>>()
        .map_err(|e| EngineError::InvalidOptions {
            reason: format!("result does not serialize: {e}"),
        })?;
    let families: Vec<(&str, serde_json::Value)> = stats.into_iter().map(|v| ("freq", v)).collect();
    let receipt = build_receipt(
        ds,
        "freq",
        options_json(opts)?,
        &families,
        exclusions.clone(),
        ctx,
    );
    Ok(ProcOutput {
        result: FreqResult { tables, exclusions },
        receipt,
    })
}

fn table(
    variable: &str,
    stratum: Option<String>,
    counts: &FreqCounts,
    ci_method: super::ProportionCi,
) -> FreqTable {
    FreqTable {
        variable: variable.to_string(),
        stratum,
        labels: counts.labels.clone(),
        ci_method,
        total: counts.total,
        missing: counts.missing,
        categories: proportions(&counts.counts),
    }
}
