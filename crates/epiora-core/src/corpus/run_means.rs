//! `run_means` — the `"means"` leg of `run_case` (spec §6.3): runs the
//! procedure on the fixture's dataset and returns the statistics
//! object the `expected` tree describes (`{total, missing, groups,
//! overall, t_test?, anova?, bartlett?, kruskal_wallis?}`), plus the
//! canonical receipt.

use super::super::procedures::{means, stats_json, MeansOptions};
use crate::dataset::Dataset;
use crate::EngineError;
use serde_json::Value;

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

/// Runs a `"means"` fixture. `inputs.dataset`/`inputs.variable` are
/// required; `inputs.group_by` and `inputs.include_missing` optional.
/// The output is the procedure's [`crate::procedures::ProcOutput`]
/// `(actual, receipt)` pair.
pub fn run_means(
    fixture: &Value,
    ds: &Dataset,
    ctx: &crate::procedures::RunContext,
) -> Result<
    (
        Value,
        crate::procedures::ProcOutput<crate::procedures::MeansResult>,
    ),
    EngineError,
> {
    let inputs = &fixture["inputs"];
    let opts = MeansOptions {
        variable: inputs["variable"]
            .as_str()
            .ok_or_else(|| corpus("means fixture needs inputs.variable"))?
            .to_string(),
        group_by: inputs
            .get("group_by")
            .and_then(Value::as_str)
            .map(str::to_string),
        include_missing: inputs["include_missing"].as_bool().unwrap_or(false),
    };
    let output = means(ds, &opts, ctx)?;
    let actual = stats_json(&output.result, ds.row_count() as u64)?;
    Ok((actual, output))
}
