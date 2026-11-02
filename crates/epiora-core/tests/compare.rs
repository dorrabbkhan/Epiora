#![cfg(feature = "corpus")]

use epiora_core::corpus::compare::compare;
use epiora_core::EngineError;
use serde_json::{json, Value};

fn fixture(expected: Value) -> Value {
    json!({
        "id": "test_fixture",
        "tolerance_classes": { "closed_form": 1e-9, "iterative": 1e-6 },
        "expected": expected,
    })
}

fn closed(v: f64) -> Value {
    json!({ "value": v, "class": "closed_form" })
}

fn iterative(v: f64) -> Value {
    json!({ "value": v, "class": "iterative" })
}

#[test]
fn compare_relative_pass_and_fail() {
    let f = fixture(json!({ "p": closed(1.8e-7) }));
    let pass = json!({ "p": 1.8e-7 * (1.0 + 5e-10) });
    assert!(compare(&f, &pass).expect("compare").is_empty());

    let fail = json!({ "p": 1.8e-7 + 1e-9 });
    assert_eq!(compare(&f, &fail).expect("compare").len(), 1);
}

#[test]
fn compare_zero_expected_uses_absolute() {
    let f = fixture(json!({ "v": closed(0.0) }));
    assert!(compare(&f, &json!({ "v": 5e-10 }))
        .expect("compare")
        .is_empty());
    assert_eq!(
        compare(&f, &json!({ "v": 2e-9 })).expect("compare").len(),
        1
    );
}

#[test]
fn compare_iterative_class() {
    let f = fixture(json!({ "or": iterative(138.393) }));
    assert!(compare(&f, &json!({ "or": 138.393 * (1.0 + 9e-7) }))
        .expect("compare")
        .is_empty());
    assert_eq!(
        compare(&f, &json!({ "or": 138.865 }))
            .expect("compare")
            .len(),
        1
    );
}

#[test]
fn compare_null_needs_undefined() {
    let f = fixture(json!({ "v": { "value": null, "class": "closed_form" } }));
    assert_eq!(compare(&f, &json!({ "v": 3.0 })).expect("compare").len(), 1);
    assert!(compare(&f, &json!({ "v": { "undefined": "zero_cell" } }))
        .expect("compare")
        .is_empty());
}

#[test]
fn compare_missing_path_is_mismatch() {
    let f = fixture(json!({ "a": { "b": closed(1.0) } }));
    let mismatches = compare(&f, &json!({ "a": {} })).expect("compare");
    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].path, "a.b");
}

#[test]
fn compare_walks_arrays() {
    let f = fixture(json!({ "values": [closed(1.0), closed(2.0)] }));
    assert!(compare(&f, &json!({ "values": [1.0, 2.0] }))
        .expect("compare")
        .is_empty());

    let mismatches = compare(&f, &json!({ "values": [1.0, 2.5] })).expect("compare");
    assert_eq!(mismatches.len(), 1);
    assert_eq!(mismatches[0].path, "values[1]");
}

#[test]
fn compare_rejects_loosened_classes() {
    let f = json!({
        "id": "test_fixture",
        "tolerance_classes": { "closed_form": 1e-6, "iterative": 1e-6 },
        "expected": { "v": closed(1.0) },
    });
    assert!(matches!(
        compare(&f, &json!({ "v": 1.0 })),
        Err(EngineError::CorpusFormat { .. })
    ));
}

#[test]
fn mismatch_message_names_path_expected_actual_class() {
    let f = fixture(json!({ "odds_ratio": { "estimate": closed(23.45) } }));
    let mismatches = compare(&f, &json!({ "odds_ratio": { "estimate": 22.0 } })).expect("compare");
    assert_eq!(mismatches.len(), 1);
    let msg = mismatches[0].to_string();
    assert!(msg.contains("odds_ratio.estimate"), "{msg}");
    assert!(msg.contains("23.45"), "{msg}");
    assert!(msg.contains("22"), "{msg}");
    assert!(msg.contains("closed_form"), "{msg}");
}
