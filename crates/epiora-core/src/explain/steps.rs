//! `steps` — the small builders every explainer shares (v1.0 Task
//! 10, spec §6). A trace is one step per emitted statistic; the
//! step's `result` is the leaf the run itself produced, navigated
//! out of the serialized result tree (bit-identical by
//! construction).

use super::path::{leaf_estimate, navigate, Segment};
use super::{ExplainKind, ExplainStep, Explanation};
use crate::error::EngineError;
use crate::stats::Estimate;
use serde_json::Value;
use std::collections::BTreeMap;

/// A step's named intermediate values.
pub type Vals = BTreeMap<String, f64>;

/// Builds a `Vals` map from `(name, value)` pairs, dropping any
/// non-finite intermediate — `values` never carries NaN or
/// infinity (the step's `result` relays an `Undefined` leaf
/// instead).
pub fn vals(pairs: &[(&str, f64)]) -> Vals {
    pairs
        .iter()
        .filter(|(_, v)| v.is_finite())
        .map(|(k, v)| ((*k).to_string(), *v))
        .collect()
}

/// The `Estimate` at `segs` inside `tree`, or `ExplainUnknown` when
/// the path resolves to no leaf.
pub fn leaf(tree: &Value, segs: &[Segment], statistic: &str) -> Result<Estimate, EngineError> {
    navigate(tree, segs)
        .and_then(leaf_estimate)
        .ok_or_else(|| super::path::unknown(statistic))
}

/// A one-step closed-form trace.
pub fn closed(statistic: &str, formula: &str, values: Vals, result: Estimate) -> Explanation {
    Explanation {
        statistic: statistic.to_string(),
        kind: ExplainKind::ClosedForm,
        steps: vec![ExplainStep {
            formula: formula.to_string(),
            values,
            result,
        }],
    }
}

/// A one-step method trace — inputs and result only, for the
/// iterative, exact and method-computed statistics (spec §6).
pub fn method(statistic: &str, formula: &str, values: Vals, result: Estimate) -> Explanation {
    Explanation {
        statistic: statistic.to_string(),
        kind: ExplainKind::Method,
        steps: vec![ExplainStep {
            formula: formula.to_string(),
            values,
            result,
        }],
    }
}
