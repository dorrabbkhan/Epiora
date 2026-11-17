//! Result-tree navigation for the explain tests: expanding a
//! `[]` registry pattern to concrete leaf paths inside a
//! serialized stats tree, and reading the `Estimate` a leaf
//! carries (`{"undefined": code}` and bare numbers are leaves).

use epiora_core::stats::{Estimate, UndefinedReason};
use serde_json::Value;

/// The concrete-index paths a `[]`-pattern expands to inside `tree`;
/// a container path expands to every leaf beneath it. Since v1.0
/// Task 11 the expansion lives in `corpus::explain_lines` — the same
/// code the parity explain lines use.
pub fn concretize(tree: &Value, pattern: &str) -> Vec<String> {
    epiora_core::corpus::explain_lines::concretize(tree, pattern).expect("concretize")
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
