//! `means` — Epi Info's MEANS (spec §4.2): per-group and overall
//! descriptives of one numeric (Integer/Real/Date) variable, and for
//! grouped runs the pooled and Welch (Satterthwaite) t-tests, one-way
//! ANOVA, Bartlett's test and the tie-corrected Kruskal–Wallis H.

use super::{build_receipt, check_ctx, options_json};
use super::{MeansOptions, MeansResult, ProcOutput, RunContext};
use crate::dataset::Dataset;
use crate::stats::{
    anova_from_summary, bartlett_from_summary, group_stats, kruskal_wallis, summarize,
    t_tests_from_summary, GroupSummary,
};
use crate::tabulate::group_numeric;
use crate::EngineError;
use serde_json::{json, Value as Json};

/// Runs MEANS (spec §4.2). Group order is the v0.2 category order with
/// `"(missing)"` last; the tests are emitted only for a grouped run —
/// `t_test` when R's `t.test` pair would run on exactly two non-empty
/// groups (both variance-defined, neither standard error "essentially
/// zero"), `bartlett` when `bartlett.test` would run (every group ≥ 2
/// observations), `anova`/`kruskal_wallis` whenever ≥ 2 non-empty
/// groups exist.
pub fn means(
    ds: &Dataset,
    opts: &MeansOptions,
    ctx: &RunContext,
) -> Result<ProcOutput<MeansResult>, EngineError> {
    check_ctx(ctx)?;
    let grouped = group_numeric(
        ds,
        &opts.variable,
        opts.group_by.as_deref(),
        opts.include_missing,
    )?;

    let groups: Vec<_> = grouped
        .labels
        .iter()
        .cloned()
        .zip(&grouped.values)
        .map(|(label, values)| group_stats(label, values))
        .collect();
    let overall = group_stats("(all)".to_string(), &grouped.all);

    let summaries: Vec<GroupSummary> = grouped.values.iter().map(|v| summarize(v)).collect();
    let grouped_run = opts.group_by.is_some() && summaries.len() >= 2;
    let t_test = if grouped_run && summaries.len() == 2 {
        t_tests_from_summary(&summaries[0], &summaries[1])
    } else {
        None
    };
    let anova = grouped_run.then(|| anova_from_summary(&summaries));
    let bartlett = grouped_run
        .then(|| bartlett_from_summary(&summaries))
        .flatten();
    let kruskal_wallis = grouped_run.then(|| {
        let refs: Vec<&[f64]> = grouped.values.iter().map(Vec::as_slice).collect();
        kruskal_wallis(&refs)
    });

    let result = MeansResult {
        variable: opts.variable.clone(),
        group_by: opts.group_by.clone(),
        groups,
        overall,
        t_test,
        anova,
        bartlett,
        kruskal_wallis,
        exclusions: grouped.exclusions.clone(),
    };

    let receipt = build_receipt(
        ds,
        "means",
        options_json(opts)?,
        &[("means", stats_json(&result, ds.row_count() as u64)?)],
        grouped.exclusions,
        ctx,
    );
    Ok(ProcOutput { result, receipt })
}

/// The statistics object the fixture `expected` tree and the receipt's
/// validation list share (spec §4.3): the result without the
/// `exclusions` block plus the run's `total` (rows in scope) and
/// `missing` (records dropped for a missing variable or group value).
pub(crate) fn stats_json(result: &MeansResult, row_count: u64) -> Result<Json, EngineError> {
    let mut v = serde_json::to_value(result).map_err(|e| EngineError::InvalidOptions {
        reason: format!("result does not serialize: {e}"),
    })?;
    let map = v.as_object_mut().ok_or(EngineError::CorpusFormat {
        reason: "means result is not an object".into(),
    })?;
    map.remove("exclusions");
    let observations = result.overall.observations;
    map.insert("total".to_string(), json!(row_count));
    map.insert("missing".to_string(), json!(row_count - observations));
    Ok(v)
}
