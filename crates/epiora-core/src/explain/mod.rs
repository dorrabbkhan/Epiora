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
/// `two_by_two.odds_ratio.ci_95_wald.lower`. The call is
/// run-then-trace: the procedure runs once and the trace reads the
/// run's output.
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
        ("freq", "freq")
        | ("tables", "two_by_two")
        | ("tables", "stratified_two_by_two")
        | ("tables", "rxc")
        | ("means", "means") => {
            run_procedure(ds, procedure, options)?.trace_sub(family, statistic, sub)
        }
        _ => Err(path::unknown(statistic)),
    }
}

/// The explain trace of one calculator statistic (spec §6).
/// `id` is the calculator's registry id (`"screening"`),
/// `inputs` the calculator inputs object, `statistic` the
/// `calc.<id>.<path>` id with concrete indices —
/// `calc.screening.sensitivity.estimate`. Run-then-trace like
/// [`explain_procedure`].
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
    run_calculator(id, inputs)?.trace_sub(statistic, &segs[2..])
}

/// The run half of [`explain_procedure`] — one procedure run every
/// emitted statistic traces from (v1.0 Task 10 fix round 3). The
/// corpus explain tests run each corpus case once and `trace` every
/// leaf off the same output; the public entry point stays
/// `explain_procedure` ("run, then trace" — no behaviour change).
/// Not part of the supported API.
#[doc(hidden)]
pub struct ProcedureRun(ProcedureRunInner);

enum ProcedureRunInner {
    Freq(Box<freq::FreqRun>),
    Means(Box<means::MeansRun>),
    Tables(Box<two_by_two::TablesRun>),
}

impl ProcedureRun {
    fn trace_sub(
        &self,
        family: &str,
        statistic: &str,
        sub: &[path::Segment],
    ) -> Result<Explanation, EngineError> {
        match (&self.0, family) {
            (ProcedureRunInner::Freq(r), "freq") => r.trace(statistic, sub),
            (ProcedureRunInner::Tables(r), "two_by_two") => r.trace(statistic, sub),
            (ProcedureRunInner::Tables(r), "stratified_two_by_two") => {
                r.trace_stratified(statistic, sub)
            }
            (ProcedureRunInner::Tables(r), "rxc") => r.trace_rxc(statistic, sub),
            (ProcedureRunInner::Means(r), "means") => r.trace(statistic, sub),
            _ => Err(path::unknown(statistic)),
        }
    }

    /// The trace entry the corpus tests use — parses `statistic`
    /// and dispatches exactly as `explain_procedure` does after its
    /// run, so the two paths cannot diverge.
    #[doc(hidden)]
    pub fn trace(&self, statistic: &str) -> Result<Explanation, EngineError> {
        let segs = path::parse(statistic);
        let (family, sub) = match segs.split_first() {
            Some((f, rest)) if !rest.is_empty() => (f.key.as_str(), rest),
            _ => return Err(path::unknown(statistic)),
        };
        self.trace_sub(family, statistic, sub)
    }
}

/// Runs `procedure` (`"freq"`, `"tables"` or `"means"`) once for
/// [`ProcedureRun::trace`]; test support for the corpus explain
/// tests, not part of the supported API.
#[doc(hidden)]
pub fn run_procedure(
    ds: &Dataset,
    procedure: &str,
    options: &Value,
) -> Result<ProcedureRun, EngineError> {
    match procedure {
        "freq" => {
            freq::run(ds, options).map(|r| ProcedureRun(ProcedureRunInner::Freq(Box::new(r))))
        }
        "means" => {
            means::run(ds, options).map(|r| ProcedureRun(ProcedureRunInner::Means(Box::new(r))))
        }
        "tables" => two_by_two::run(ds, options)
            .map(|r| ProcedureRun(ProcedureRunInner::Tables(Box::new(r)))),
        _ => Err(path::unknown(procedure)),
    }
}

/// The run half of [`explain_calculator`] — one calculator run
/// every `calc.<id>.*` statistic traces from. Test support, not
/// part of the supported API.
#[doc(hidden)]
pub struct CalcRun(calc::CalcRun);

impl CalcRun {
    fn trace_sub(
        &self,
        statistic: &str,
        sub: &[path::Segment],
    ) -> Result<Explanation, EngineError> {
        self.0.trace(statistic, sub)
    }

    /// The trace entry the corpus tests use — same
    /// `calc.<id>.<path>` check `explain_calculator` performs.
    #[doc(hidden)]
    pub fn trace(&self, statistic: &str) -> Result<Explanation, EngineError> {
        let segs = path::parse(statistic);
        if segs.len() < 3
            || segs[0].key != "calc"
            || segs[0].index.is_some()
            || segs[1].key != self.0.id()
            || segs[1].index.is_some()
        {
            return Err(path::unknown(statistic));
        }
        self.trace_sub(statistic, &segs[2..])
    }
}

/// Runs calculator `id` once for [`CalcRun::trace`]; test support
/// for the corpus explain tests, not part of the supported API.
#[doc(hidden)]
pub fn run_calculator(id: &str, inputs: &Value) -> Result<CalcRun, EngineError> {
    calc::run(id, inputs).map(CalcRun)
}

/// Deserialize `options` into a procedure's options type.
pub(crate) fn options<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, EngineError> {
    serde_json::from_value(v.clone()).map_err(|e| EngineError::InvalidOptions {
        reason: format!("options do not parse: {e}"),
    })
}
