//! Receipt project stamp (v0.3 spec §8.2): a run without a project
//! emits byte-identical receipt v1; naming a project bumps to
//! `receipt_version` 2 with a `"project"` key that `receipt_sha256`
//! covers.

#![cfg(feature = "corpus")]

use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::procedures::{tables, RunContext, TablesOptions, VarSpec};
use epiora_core::receipt::{canonical_json, EngineStamp, ProjectStamp, Receipt};
use serde_json::Value;
use sha2::Digest;
use std::path::Path;

fn spec(name: &str) -> VarSpec {
    VarSpec {
        name: name.into(),
        levels: None,
    }
}

/// The same dataset, options and timestamp as `tests/receipt.rs`'s
/// golden receipt.
fn golden_inputs() -> (Dataset, TablesOptions) {
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
    (ds, opts)
}

fn ctx(project: Option<ProjectStamp>) -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project,
    }
}

/// Re-pins the engine stamp to `git_sha = "0000000"` and re-seals, as
/// the v0.2 golden receipt test does.
fn pin_engine(mut receipt: Receipt) -> Receipt {
    receipt.engine = EngineStamp {
        name: "epiora-core".into(),
        version: "0.2.0".into(),
        git_sha: "0000000".into(),
    };
    receipt.receipt_sha256 = receipt.sha256_hex();
    receipt
}

#[test]
fn no_project_is_exact_v1() {
    let (ds, opts) = golden_inputs();
    let receipt = pin_engine(tables(&ds, &opts, &ctx(None)).expect("tables").receipt);
    assert_eq!(receipt.receipt_version, 1);
    let canon = canonical_json(&serde_json::to_value(&receipt).expect("serialize receipt"));
    assert!(!canon.contains("\"project\""));
    let golden = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/receipt_v1.json"),
    )
    .expect("golden receipt file");
    assert_eq!(canon, golden.trim_end());
}

#[test]
fn project_gives_v2_with_project_key() {
    let (ds, opts) = golden_inputs();
    let stamp = ProjectStamp {
        id: "p1".into(),
        name: "Oswego".into(),
    };
    let receipt = pin_engine(
        tables(&ds, &opts, &ctx(Some(stamp)))
            .expect("tables")
            .receipt,
    );
    assert_eq!(receipt.receipt_version, 2);

    let value: Value = serde_json::to_value(&receipt).expect("serialize receipt");
    assert_eq!(
        value["project"],
        serde_json::json!({"id": "p1", "name": "Oswego"})
    );

    // `receipt_sha256` covers the project key: dropping it from the
    // canonical document changes the hash.
    assert_eq!(receipt.receipt_sha256, receipt.sha256_hex());
    let mut without = value.clone();
    without.as_object_mut().expect("object").remove("project");
    let digest = sha2::Sha256::digest(canonical_json(&without).as_bytes());
    let hash: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    assert_ne!(hash, receipt.receipt_sha256);
}
