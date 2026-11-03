#![cfg(feature = "corpus")]

use epiora_core::corpus::compare::compare;
use epiora_core::stats::{two_by_two, Table2x2};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/2x2")
}

fn load_dir(dir: &Path) -> Vec<(String, Value)> {
    let mut out: Vec<(String, Value)> = std::fs::read_dir(dir)
        .expect("read fixture dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            let id = p
                .file_stem()
                .expect("file stem")
                .to_string_lossy()
                .to_string();
            let fixture = serde_json::from_str(&std::fs::read_to_string(&p).expect("read fixture"))
                .expect("parse fixture");
            (id, fixture)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn table_of(fixture: &Value) -> Table2x2 {
    let t = &fixture["inputs"]["table"];
    let cell = |r: usize, c: usize| t[r][c].as_u64().expect("table cell is u64");
    Table2x2 {
        a: cell(0, 0),
        b: cell(0, 1),
        c: cell(1, 0),
        d: cell(1, 1),
    }
}

/// Every expected leaf's actual value is a finite JSON number, or the
/// leaf is `{"undefined": "<non-empty string>"}`. Anything else — a
/// bare null, a string, a bool — is a violation: NaN can never reach
/// the serialized output (spec §3.5 "Undefined, never NaN").
fn check_leaves(v: &Value, path: &str, problems: &mut Vec<String>) {
    match v {
        Value::Object(m) => {
            if let Some(u) = m.get("undefined") {
                if m.len() != 1 || !u.as_str().is_some_and(|s| !s.is_empty()) {
                    problems.push(format!("{path}: malformed undefined leaf {v}"));
                }
            } else {
                for (k, c) in m {
                    check_leaves(c, &format!("{path}.{k}"), problems);
                }
            }
        }
        Value::Array(items) => {
            for (i, c) in items.iter().enumerate() {
                check_leaves(c, &format!("{path}[{i}]"), problems);
            }
        }
        // serde_json::Number can only hold finite values; a NaN would
        // serialize as null and land in `other`.
        Value::Number(_) => {}
        other => problems.push(format!(
            "{path}: not a finite number or undefined leaf: {other}"
        )),
    }
}

/// Every `{value: null}` leaf in `expected` must have an `undefined`
/// leaf at the same path in `actual`.
fn check_null_is_undefined(
    expected: &Value,
    actual: Option<&Value>,
    path: &str,
    out: &mut Vec<String>,
) {
    match expected {
        Value::Object(m) => {
            if m.contains_key("value") {
                if m.get("value").is_some_and(Value::is_null) {
                    let ok = actual
                        .and_then(|a| a.get("undefined"))
                        .is_some_and(Value::is_string);
                    if !ok {
                        out.push(format!(
                            "{path}: expected undefined, actual {}",
                            describe(actual)
                        ));
                    }
                }
            } else {
                for (k, c) in m {
                    check_null_is_undefined(
                        c,
                        actual.and_then(|a| a.get(k)),
                        join(path, k).as_str(),
                        out,
                    );
                }
            }
        }
        Value::Array(items) => {
            for (i, c) in items.iter().enumerate() {
                check_null_is_undefined(
                    c,
                    actual.and_then(|a| a.get(i)),
                    &format!("{path}[{i}]"),
                    out,
                );
            }
        }
        _ => {}
    }
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

#[test]
fn two_by_two_fixtures() {
    let mut failures = Vec::new();
    for (id, fixture) in load_dir(&fixture_dir()) {
        let actual =
            serde_json::to_value(two_by_two(&table_of(&fixture))).expect("serialize TwoByTwoStats");
        match compare(&fixture, &actual) {
            Ok(mismatches) => {
                for m in mismatches {
                    failures.push(format!("{id}: {m}"));
                }
            }
            Err(e) => failures.push(format!("{id}: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn zero_cell_fixtures() {
    let mut problems = Vec::new();
    for name in ["zero_a", "zero_b", "zero_c", "zero_d", "all_zero"] {
        let (_, fixture) = load_dir(&fixture_dir())
            .into_iter()
            .find(|(id, _)| id == name)
            .expect("zero fixture exists");
        let actual =
            serde_json::to_value(two_by_two(&table_of(&fixture))).expect("serialize TwoByTwoStats");
        check_leaves(&actual, name, &mut problems);
        check_null_is_undefined(&fixture["expected"], Some(&actual), name, &mut problems);
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
