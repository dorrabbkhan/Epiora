//! Compares an engine result (as JSON) with a fixture's `expected` tree
//! under the E7 rule (spec §6.2). Each failure message names the JSON
//! path, expected, actual and class (spec §6.3); the caller adds the
//! fixture id.

use crate::validation::ToleranceClass;
use crate::EngineError;
use serde_json::{Map, Value};
use std::fmt;

/// One differing value between fixture and engine output.
#[derive(Debug, Clone, PartialEq)]
pub struct Mismatch {
    /// JSON path inside `expected`, e.g. `odds_ratio.ci_95_wald.lower`
    /// or `categories[0].percent`.
    pub path: String,
    /// The expected number, or `None` when the fixture says undefined.
    pub expected: Option<f64>,
    /// What the engine produced at that path: the compact JSON value or
    /// "missing".
    pub actual: String,
    /// The leaf's tolerance class (`closed_form` or `iterative`).
    pub class: String,
}

impl fmt::Display for Mismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let expected = match self.expected {
            Some(v) => v.to_string(),
            None => "undefined".to_string(),
        };
        write!(
            f,
            "{}: expected {}, actual {}, class {}",
            self.path, expected, self.actual, self.class
        )
    }
}

fn corpus_format(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

/// Compares `actual` against `fixture["expected"]` (spec §6.2, E7).
///
/// `fixture["tolerance_classes"]` must be exactly
/// `{"closed_form": 1e-9, "iterative": 1e-6}` — anything else is
/// `CorpusFormat`, so a loosened class can never pass silently.
/// Keys present only in `actual` are ignored; missing paths are
/// mismatches.
pub fn compare(fixture: &Value, actual: &Value) -> Result<Vec<Mismatch>, EngineError> {
    check_tolerance_classes(fixture)?;
    let expected = fixture
        .get("expected")
        .ok_or_else(|| corpus_format("fixture has no \"expected\""))?;
    let mut mismatches = Vec::new();
    walk(expected, Some(actual), String::new(), &mut mismatches)?;
    Ok(mismatches)
}

fn check_tolerance_classes(fixture: &Value) -> Result<(), EngineError> {
    let classes = fixture
        .get("tolerance_classes")
        .and_then(Value::as_object)
        .ok_or_else(|| corpus_format("missing or invalid tolerance_classes"))?;
    let exact = classes.len() == 2
        && classes.get("closed_form").and_then(Value::as_f64)
            == Some(ToleranceClass::ClosedForm.tolerance())
        && classes.get("iterative").and_then(Value::as_f64)
            == Some(ToleranceClass::Iterative.tolerance());
    if !exact {
        return Err(corpus_format(
            "tolerance_classes must be exactly {\"closed_form\": 1e-9, \"iterative\": 1e-6}",
        ));
    }
    Ok(())
}

fn walk(
    expected: &Value,
    actual: Option<&Value>,
    path: String,
    out: &mut Vec<Mismatch>,
) -> Result<(), EngineError> {
    match expected {
        Value::Object(map) => {
            if map.contains_key("value") {
                leaf(map, actual, path, out)
            } else {
                for (key, value) in map {
                    let child = actual.and_then(|a| a.get(key));
                    walk(value, child, join(&path, key), out)?;
                }
                Ok(())
            }
        }
        Value::Array(items) => {
            for (i, value) in items.iter().enumerate() {
                let child = actual.and_then(|a| a.get(i));
                walk(value, child, format!("{path}[{i}]"), out)?;
            }
            Ok(())
        }
        _ => Err(corpus_format(format!(
            "expected value at {path:?} is neither an object nor an array"
        ))),
    }
}

/// A leaf `{value, class, r_default?}` (spec §6.2). `{value: <number>}`
/// needs a JSON number at the same path passing E7 at the leaf's class;
/// `{value: null}` needs `{"undefined": <string>}`.
fn leaf(
    map: &Map<String, Value>,
    actual: Option<&Value>,
    path: String,
    out: &mut Vec<Mismatch>,
) -> Result<(), EngineError> {
    let class: ToleranceClass = map
        .get("class")
        .cloned()
        .and_then(|c| serde_json::from_value(c).ok())
        .ok_or_else(|| corpus_format(format!("leaf at {path:?} has no valid class")))?;
    let class_name = match class {
        ToleranceClass::ClosedForm => "closed_form",
        ToleranceClass::Iterative => "iterative",
    }
    .to_string();

    match map.get("value") {
        Some(Value::Null) => {
            let ok = actual
                .and_then(|a| a.get("undefined"))
                .is_some_and(Value::is_string);
            if !ok {
                out.push(Mismatch {
                    path,
                    expected: None,
                    actual: describe(actual),
                    class: class_name,
                });
            }
        }
        Some(v @ Value::Number(_)) => {
            let expected = v
                .as_f64()
                .ok_or_else(|| corpus_format(format!("leaf at {path:?} is not finite")))?;
            let ok = actual
                .and_then(Value::as_f64)
                .is_some_and(|a| ToleranceClass::passes(a, expected, class.tolerance()));
            if !ok {
                out.push(Mismatch {
                    path,
                    expected: Some(expected),
                    actual: describe(actual),
                    class: class_name,
                });
            }
        }
        _ => {
            return Err(corpus_format(format!(
                "leaf at {path:?} has a non-numeric, non-null \"value\""
            )))
        }
    }
    Ok(())
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

fn describe(actual: Option<&Value>) -> String {
    match actual {
        Some(v) => v.to_string(),
        None => "missing".to_string(),
    }
}
