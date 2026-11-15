#![cfg(feature = "corpus")]

//! Receipt v3 tests (v1.0 Task 7): calculator receipts carry a
//! `calculator` stamp and no `dataset`; v1/v2 serialisation is
//! byte-identical.

use epiora_core::calc::run;
use epiora_core::corpus::load_dataset_dir;
use epiora_core::procedures::{tables, RunContext, TablesOptions, VarSpec};
use epiora_core::receipt::{canonical_json, EngineStamp, Receipt};
use serde_json::{json, Value};
use std::path::Path;

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project: None,
    }
}

fn spec(name: &str) -> VarSpec {
    VarSpec {
        name: name.into(),
        levels: None,
    }
}

/// The golden v1 receipt from `tests/receipt.rs`, rebuilt the same way.
#[test]
fn golden_v1_unchanged() {
    use epiora_core::dataset::{Column, ColumnData, Dataset};
    let ds = Dataset::new(
        "golden",
        1,
        vec![
            Column {
                name: "E".into(),
                data: ColumnData::Boolean(vec![
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                ]),
            },
            Column {
                name: "I".into(),
                data: ColumnData::Boolean(vec![
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(true),
                    Some(false),
                    Some(false),
                ]),
            },
            Column {
                name: "S".into(),
                data: ColumnData::Text(vec![
                    Some("x".into()),
                    Some("x".into()),
                    Some("y".into()),
                    Some("y".into()),
                    Some("x".into()),
                    Some("x".into()),
                    Some("y".into()),
                    Some("y".into()),
                ]),
            },
        ],
    )
    .expect("dataset");
    let opts = TablesOptions {
        exposure: spec("E"),
        outcome: spec("I"),
        stratify_by: vec!["S".into()],
        include_missing: false,
    };
    let mut receipt: Receipt = tables(&ds, &opts, &ctx()).expect("tables").receipt;
    receipt.engine = EngineStamp {
        name: "epiora-core".into(),
        version: "0.2.0".into(),
        git_sha: "0000000".into(),
    };
    receipt.receipt_sha256 = receipt.sha256_hex();
    let canon = canonical_json(&serde_json::to_value(&receipt).expect("serialize receipt"));
    let golden = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/receipt_v1.json"),
    )
    .expect("golden receipt file");
    assert_eq!(canon, golden.trim_end());
}

#[test]
fn calculator_receipt_is_v3_without_dataset() {
    let out = run(
        "proportion",
        &json!({"numerator": 3, "denominator": 5}),
        &ctx(),
    )
    .expect("proportion run");
    let receipt: Value = serde_json::to_value(&out.receipt).expect("serialize receipt");
    assert_eq!(receipt["receipt_version"], 3);
    assert_eq!(receipt["procedure"], "calc.proportion");
    assert_eq!(receipt["timestamp"], "2026-10-30T12:00:00Z");
    // No dataset key at all — calculators run without a project.
    assert!(receipt.get("dataset").is_none());
    assert_eq!(receipt["calculator"]["id"], "proportion");
    // Canonical inputs have the defaults filled.
    assert_eq!(receipt["calculator"]["inputs"]["conf_level"], 95);
    assert_eq!(receipt["calculator"]["inputs"]["numerator"], 3);
}

#[test]
fn v3_hash_covers_inputs() {
    let a = run(
        "proportion",
        &json!({"numerator": 3, "denominator": 5}),
        &ctx(),
    )
    .expect("run a");
    let b = run(
        "proportion",
        &json!({"numerator": 4, "denominator": 5}),
        &ctx(),
    )
    .expect("run b");
    assert_ne!(
        a.receipt.receipt_sha256, b.receipt.receipt_sha256,
        "different inputs must produce different receipt hashes"
    );
    let c = run(
        "proportion",
        &json!({"numerator": 3, "denominator": 5}),
        &ctx(),
    )
    .expect("run c");
    assert_eq!(a.receipt.receipt_sha256, c.receipt.receipt_sha256);
}

/// Dataset-backed procedure receipts keep `dataset` and no `calculator`.
#[test]
fn procedure_receipt_has_dataset_no_calculator() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/data");
    let sets = load_dataset_dir(&dir).expect("corpus datasets");
    let ds = sets.get("missing").expect("missing dataset");
    let opts = TablesOptions {
        exposure: spec("EXPOSED"),
        outcome: spec("ILL"),
        stratify_by: vec![],
        include_missing: false,
    };
    let receipt = serde_json::to_value(tables(ds, &opts, &ctx()).expect("tables").receipt)
        .expect("serialize");
    assert!(receipt.get("dataset").is_some());
    assert!(receipt.get("calculator").is_none());
    assert_eq!(receipt["receipt_version"], 1);
}
