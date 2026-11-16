//! `explain` — engine-computed explain traces (v1.0 Task 10, spec
//! §6). Every statistic a procedure or calculator emits resolves to
//! an [`Explanation`]: an ordered list of steps, each a formula id
//! and the named intermediate values the engine computed for it.
//! The last step's `result` is the leaf value of the run itself —
//! read back from the serialized result, bit-identical by
//! construction. Iterative, exact and method-computed statistics
//! return `ExplainKind::Method` with a single step carrying their
//! inputs and result only; explain describes the method in words
//! and does not pretend to show a closed form.
//!
//! A statistic whose path resolves to no leaf (or no family
//! prefix) is `EngineError::ExplainUnknown`.

mod calc;
pub mod formulas;
mod freq;
mod means;
mod path;
mod steps;
mod two_by_two;

pub use formulas::FORMULAS;

use crate::dataset::Dataset;
use crate::error::EngineError;
use crate::stats::Estimate;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// One step of a trace: the formula id, the named intermediate
/// values the engine computed, and the step's result — the leaf
/// itself for the final step.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ExplainStep {
    /// A formula id from `FORMULAS`.
    pub formula: String,
    /// The named intermediates the formula consumes (finite only —
    /// a non-finite intermediate is omitted and the step's `result`
    /// is `Undefined`).
    pub values: BTreeMap<String, f64>,
    /// The step's result — the run's own leaf value, bit-identical.
    pub result: Estimate,
}

/// Whether the statistic has a closed-form formula (`ClosedForm`)
/// or is computed by an iterative/exact/method algorithm
/// (`Method`, one step of inputs and result only — spec §6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum ExplainKind {
    /// A formula the trace's `values` evaluate.
    ClosedForm,
    /// An iterative, exact or method-computed statistic.
    Method,
}

/// The explain trace of one statistic (spec §6).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Explanation {
    /// The concrete statistic path the caller asked for.
    pub statistic: String,
    /// `ClosedForm` or `Method`.
    pub kind: ExplainKind,
    /// The ordered steps; the last `result` is the run's leaf.
    pub steps: Vec<ExplainStep>,
}

/// The `RunContext` explain runs procedures under — the receipt is
/// discarded, so the timestamp is a fixed marker string (the engine
/// reads no clock).
const EXPLAIN_TS: &str = "explain";

/// The explain trace of one `means`/`freq`/`tables`-emitted
/// statistic (spec §6). `procedure` is `"freq"`, `"tables"` or
/// `"means"`; `options` is the procedure's options object;
/// `statistic` is the registry id with concrete indices —
/// `freq.categories[1].percent`,
/// `two_by_two.odds_ratio.ci_95_wald.lower`.
pub fn explain_procedure(
    ds: &Dataset,
    procedure: &str,
    options: &Value,
    statistic: &str,
) -> Result<Explanation, EngineError> {
    let segs = path::parse(statistic);
    let (family, sub) = match segs.split_first() {
        Some((f, rest)) if !rest.is_empty() => (f.key.as_str(), rest),
        _ => return Err(path::unknown(statistic)),
    };
    match (procedure, family) {
        ("freq", "freq") => freq::explain(ds, options, statistic, sub),
        ("tables", "two_by_two") => two_by_two::explain(ds, options, statistic, sub),
        ("tables", "stratified_two_by_two") => {
            two_by_two::explain_stratified(ds, options, statistic, sub)
        }
        ("tables", "rxc") => two_by_two::explain_rxc(ds, options, statistic, sub),
        ("means", "means") => means::explain(ds, options, statistic, sub),
        _ => Err(path::unknown(statistic)),
    }
}

/// The explain trace of one calculator statistic (spec §6).
/// `id` is the calculator's registry id (`"screening"`),
/// `inputs` the calculator inputs object, `statistic` the
/// `calc.<id>.<path>` id with concrete indices —
/// `calc.screening.sensitivity.estimate`.
pub fn explain_calculator(
    id: &str,
    inputs: &Value,
    statistic: &str,
) -> Result<Explanation, EngineError> {
    let segs = path::parse(statistic);
    if segs.len() < 3
        || segs[0].key != "calc"
        || segs[0].index.is_some()
        || segs[1].key != id
        || segs[1].index.is_some()
    {
        return Err(path::unknown(statistic));
    }
    calc::explain(id, inputs, statistic, &segs[2..])
}

/// Deserialize `options` into a procedure's options type.
pub(crate) fn options<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, EngineError> {
    serde_json::from_value(v.clone()).map_err(|e| EngineError::InvalidOptions {
        reason: format!("options do not parse: {e}"),
    })
}
