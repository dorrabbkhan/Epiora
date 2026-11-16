//! `path` — statistic-path parsing and result-tree navigation (v1.0
//! Task 10, spec §6). A statistic is a concrete path with indices:
//! `freq.categories[1].percent`, `two_by_two.odds_ratio.ci_95_wald.lower`,
//! `calc.screening.sensitivity.estimate`. `[]` marks an array step in a
//! registry pattern; a concrete path carries `[i]`.

use crate::error::EngineError;
use crate::stats::Estimate;
use serde_json::Value;

/// One path segment: the object key, plus the array index when the
/// segment addresses one element (`categories[1]` → `("categories", Some(1))`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The key before any `[i]`.
    pub key: String,
    /// `Some(i)` for `key[i]`; `None` for a bare `key` or the `[]`
    /// wildcard of a registry pattern.
    pub index: Option<usize>,
}

/// Splits a concrete path into segments. `a.b[2].c` →
/// `[a, b[2], c]`; the registry `[]` spelling maps to `index: None`.
pub fn parse(path: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    for part in path.split('.') {
        match part.find('[') {
            Some(open) if part.ends_with(']') => {
                let key = part[..open].to_string();
                let inner = &part[open + 1..part.len() - 1];
                let index = if inner.is_empty() {
                    None
                } else {
                    inner.parse::<usize>().ok()
                };
                out.push(Segment { key, index });
            }
            _ => out.push(Segment {
                key: part.to_string(),
                index: None,
            }),
        }
    }
    out
}

/// Follows `segments` through a serialized result tree. `None` when a
/// key or index is absent.
pub fn navigate<'a>(tree: &'a Value, segments: &[Segment]) -> Option<&'a Value> {
    let mut node = tree;
    for s in segments {
        node = node.get(&s.key)?;
        if let Some(i) = s.index {
            node = node.get(i)?;
        }
    }
    Some(node)
}

/// The `Estimate` a serialized leaf encodes: a finite number, or
/// `{"undefined": "<code>"}`. Numbers are read as f64; non-finite
/// JSON numbers cannot exist (serde_json rejects them), so a bare
/// number is always a `Value`.
pub fn leaf_estimate(node: &Value) -> Option<Estimate> {
    match node {
        Value::Number(n) => n.as_f64().map(Estimate::Value),
        Value::Object(m) => m
            .get("undefined")
            .and_then(Value::as_str)
            .and_then(crate::stats::UndefinedReason::from_code)
            .map(|reason| Estimate::Undefined { reason }),
        _ => None,
    }
}

/// `ExplainUnknown` for a statistic that resolves to no leaf.
pub fn unknown(statistic: &str) -> EngineError {
    EngineError::ExplainUnknown {
        statistic: statistic.to_string(),
    }
}
