//! Result-tree navigation for the explain tests: expanding a
//! `[]` registry pattern to concrete leaf paths inside a
//! serialized stats tree, and reading the `Estimate` a leaf
//! carries (`{"undefined": code}` and bare numbers are leaves).

use epiora_core::stats::{Estimate, UndefinedReason};
use epiora_core::validation::statistic_ids;
use serde_json::Value;

/// The concrete-index paths a `[]`-pattern expands to inside `tree`;
/// a container path expands to every leaf beneath it.
pub fn concretize(tree: &Value, pattern: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut frontier: Vec<(&Value, String)> = vec![(tree, String::new())];
    for seg in pattern.split('.') {
        let (key, is_arr) = match seg.strip_suffix("[]") {
            Some(k) => (k, true),
            None => (seg, false),
        };
        let mut next = Vec::new();
        for (node, path) in frontier {
            let child = &node[key];
            if child.is_null() {
                continue;
            }
            let prefix = if path.is_empty() {
                key.to_string()
            } else {
                format!("{path}.{key}")
            };
            if is_arr {
                for (i, item) in child.as_array().expect("array").iter().enumerate() {
                    next.push((item, format!("{prefix}[{i}]")));
                }
            } else {
                next.push((child, prefix));
            }
        }
        frontier = next;
    }
    for (node, path) in frontier {
        match node {
            // A `{"undefined": code}` object is a leaf, not a
            // container — `statistic_ids` names it with the empty
            // relative id, which would emit `path.` here.
            Value::Object(m) if m.contains_key("undefined") => out.push(path),
            Value::Object(_) | Value::Array(_) => {
                for rel in statistic_ids(node) {
                    out.push(format!("{path}.{}", rel.replace("[]", "[0]")));
                }
            }
            _ => out.push(path),
        }
    }
    out
}

/// The `Estimate` a concrete leaf path carries in `tree` —
/// `{"undefined": code}` and bare numbers are leaves.
pub fn leaf_estimate(tree: &Value, concrete: &str) -> Estimate {
    let mut node = tree;
    for seg in concrete.split('.') {
        match seg.strip_suffix(']').and_then(|h| h.split_once('[')) {
            Some((k, i)) => node = &node[k][i.parse::<usize>().expect("index")],
            None => node = &node[seg],
        }
    }
    match node {
        Value::Number(n) => Estimate::Value(n.as_f64().expect("leaf is f64")),
        Value::Object(m) if m.contains_key("undefined") => {
            let code = m["undefined"].as_str().expect("undefined code");
            Estimate::Undefined {
                reason: UNDEFINED_CODES
                    .iter()
                    .find(|(_, c)| *c == code)
                    .map(|(r, _)| *r)
                    .expect("undefined reason code"),
            }
        }
        other => panic!("not a leaf: {other}"),
    }
}

const UNDEFINED_CODES: &[(UndefinedReason, &str)] = &[
    (UndefinedReason::ZeroCell, "zero_cell"),
    (UndefinedReason::ZeroMargin, "zero_margin"),
    (UndefinedReason::EmptyTable, "empty_table"),
    (UndefinedReason::NotBracketed, "not_bracketed"),
    (UndefinedReason::NoConvergence, "no_convergence"),
    (UndefinedReason::Unbounded, "unbounded"),
    (UndefinedReason::TooFew, "too_few"),
    (UndefinedReason::ZeroVariance, "zero_variance"),
    (UndefinedReason::OutOfRange, "out_of_range"),
    (UndefinedReason::TooLarge, "too_large"),
    (UndefinedReason::InfiniteSampleSize, "infinite_sample_size"),
    (UndefinedReason::ZeroExpected, "zero_expected"),
];
