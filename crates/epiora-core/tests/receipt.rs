#![cfg(feature = "corpus")]

use epiora_core::corpus::load_dataset_dir;
use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::procedures::{tables, RunContext, TablesOptions, VarSpec};
use epiora_core::receipt::{canonical_json, EngineStamp, Receipt};
use serde_json::{json, Value};
use std::path::Path;

fn spec(name: &str) -> VarSpec {
    VarSpec {
        name: name.into(),
        levels: None,
    }
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
    }
}

#[test]
fn canonical_json_sorts_and_escapes() {
    let v = json!({
        "z": 1,
        "a": {"s": "x\n\"\\é", "n": null, "b": true, "f": 1.5, "e": 1e-9}
    });
    assert_eq!(
        canonical_json(&v),
        r#"{"a":{"b":true,"e":1e-9,"f":1.5,"n":null,"s":"x\u000a\"\\é"},"z":1}"#
    );
}

/// Builds the receipt `tables` emits for a fixed dataset/options/context,
/// with the engine stamp pinned to `git_sha = "0000000"` (spec §4.3).
fn build_golden() -> Receipt {
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
    let mut receipt = tables(&ds, &opts, &ctx()).expect("tables").receipt;
    receipt.engine = EngineStamp {
        name: "epiora-core".into(),
        version: "0.2.0".into(),
        git_sha: "0000000".into(),
    };
    receipt.receipt_sha256 = receipt.sha256_hex();
    receipt
}

#[test]
fn golden_receipt() {
    let receipt = build_golden();
    let canon = canonical_json(&serde_json::to_value(&receipt).expect("serialize receipt"));
    let golden = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/receipt_v1.json"),
    )
    .expect("golden receipt file");
    assert_eq!(canon, golden.trim_end());

    // `receipt_sha256` is the SHA-256 of the canonical JSON without the
    // key; the file is already canonical, so deleting the key pair from
    // the text reproduces it:
    //   sed 's/,"receipt_sha256":"[0-9a-f]\{64\}"//' \
    //     crates/epiora-core/tests/golden/receipt_v1.json | tr -d '\n' | sha256sum
    let value: Value = serde_json::from_str(&canon).expect("canonical parses");
    assert_eq!(
        value["receipt_sha256"].as_str().expect("sha string"),
        receipt.receipt_sha256
    );
}

#[test]
fn receipt_lists_exclusions() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/data");
    let sets = load_dataset_dir(&dir).expect("corpus datasets");
    let ds = sets.get("missing").expect("missing dataset");
    let opts = TablesOptions {
        exposure: spec("EXPOSED"),
        outcome: spec("ILL"),
        stratify_by: vec![],
        include_missing: false,
    };
    let receipt = tables(ds, &opts, &ctx()).expect("tables").receipt;
    let mut expected = std::collections::BTreeMap::new();
    expected.insert("EXPOSED".to_string(), 3_u64);
    expected.insert("ILL".to_string(), 4_u64);
    assert_eq!(receipt.exclusions, expected);
    assert_eq!(receipt.receipt_version, 1);
    assert_eq!(receipt.procedure, "tables");
    assert_eq!(receipt.timestamp, "2026-10-30T12:00:00Z");
    assert_eq!(receipt.dataset.id, "missing");
    assert_eq!(receipt.dataset.sha256, ds.content_hash_hex());
}
