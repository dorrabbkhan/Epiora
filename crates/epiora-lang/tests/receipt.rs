//! Receipt v4 (v1.1 plan Task 11): v4 exactly when a `working_view`
//! stamp is present, v1–v3 byte-unchanged. Shared builders live in
//! `tests/common/ds.rs`.

mod common;
use common::ds::*;
use epiora_core::dataset::{ColumnData, Dataset};
use epiora_core::procedures::{freq, FreqOptions, RunContext};
use epiora_core::receipt::{
    canonical_json, CalculatorStamp, EngineStamp, ProjectStamp, WorkingViewStamp,
};
use epiora_lang::{replay, working_view_stamp, Command};
use serde_json::json;
use std::path::Path;

fn ctx(working_view: Option<WorkingViewStamp>) -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project: None,
        working_view,
    }
}

fn freq_opts(variable: &str) -> FreqOptions {
    FreqOptions {
        variables: vec![variable.into()],
        stratify_by: None,
        include_missing: false,
        ci_method: Default::default(),
    }
}

/// The pinned engine stamp every receipt golden uses.
fn pin_engine(mut receipt: epiora_core::receipt::Receipt) -> epiora_core::receipt::Receipt {
    receipt.engine = EngineStamp {
        name: "epiora-core".into(),
        version: "1.0.0".into(),
        git_sha: "0000000".into(),
    };
    receipt.receipt_sha256 = receipt.sha256_hex();
    receipt
}

fn canon(receipt: &epiora_core::receipt::Receipt) -> String {
    canonical_json(&serde_json::to_value(receipt).expect("receipt serializes"))
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../epiora-core/tests/golden")
            .join(name),
    )
    .unwrap_or_else(|_| panic!("golden {name}"))
    .trim_end()
    .to_string()
}

fn golden_ds() -> Dataset {
    ds(
        "golden",
        vec![
            (
                "E",
                ColumnData::Boolean(vec![
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                ]),
            ),
            (
                "I",
                ColumnData::Boolean(vec![
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(true),
                    Some(false),
                    Some(false),
                ]),
            ),
            (
                "S",
                ColumnData::Text(vec![
                    Some("x".into()),
                    Some("x".into()),
                    Some("y".into()),
                    Some("y".into()),
                    Some("x".into()),
                    Some("x".into()),
                    Some("y".into()),
                    Some("y".into()),
                ]),
            ),
        ],
    )
}

#[test]
fn receipt_v4_only_with_steps() {
    let src = golden_ds();
    let r = recipe(vec![Command::Select(expr("E = (+)"))]);
    let view = replay(&src, &NoLinks, &r).expect("replay");
    let mat = view.materialize();

    // The stamp helper yields None for a step-less recipe — a session
    // receipt only becomes v4 once cleaning commands ran.
    assert!(working_view_stamp(&src, &mat, &recipe(vec![])).is_none());
    let st = working_view_stamp(&src, &mat, &r).expect("steps stamp the view");
    assert_eq!(st.recipe_text, r.canonical_text());
    assert_eq!(st.recipe_sha256, r.sha256_hex());
    assert_eq!(st.view_sha256, mat.content_hash_hex());

    // A run on the materialized view carries receipt version 4.
    let out = freq(&mat, &freq_opts("E"), &ctx(Some(st.clone()))).expect("freq");
    assert_eq!(out.receipt.receipt_version, 4);
    let value = serde_json::to_value(&out.receipt).expect("serialize");
    let wv = &value["working_view"];
    assert_eq!(wv["recipe_sha256"].as_str().unwrap(), st.recipe_sha256);
    assert_eq!(wv["view_sha256"].as_str().unwrap(), st.view_sha256);
    assert_eq!(
        wv["source"]["sha256"].as_str().unwrap(),
        src.content_hash_hex()
    );

    // Without a stamp the same run stays v1 — and serializes no key.
    let out = freq(&mat, &freq_opts("E"), &ctx(None)).expect("freq");
    assert_eq!(out.receipt.receipt_version, 1);
    let value = serde_json::to_value(&out.receipt).expect("serialize");
    assert!(value.get("working_view").is_none());
}

#[test]
fn receipts_v1_v2_v3_golden_unchanged() {
    use epiora_core::procedures::{tables, TablesOptions, VarSpec};

    let ds = golden_ds();
    let opts = TablesOptions {
        exposure: VarSpec {
            name: "E".into(),
            levels: None,
        },
        outcome: VarSpec {
            name: "I".into(),
            levels: None,
        },
        stratify_by: vec!["S".into()],
        include_missing: false,
    };

    // v1 — the pre-existing golden pins the canonical bytes.
    let r = pin_engine(tables(&ds, &opts, &ctx(None)).expect("tables").receipt);
    assert_eq!(r.receipt_version, 1);
    assert_eq!(canon(&r), golden("receipt_v1.json"));

    // v2 — a project stamp and no working view stays v2 and emits no
    // `working_view` key (the v4 field must not leak into old shapes).
    let mut c = ctx(None);
    c.project = Some(ProjectStamp {
        id: "p1".into(),
        name: "P".into(),
    });
    let r = pin_engine(tables(&ds, &opts, &c).expect("tables").receipt);
    assert_eq!(r.receipt_version, 2);
    let v = serde_json::to_value(&r).expect("serialize");
    assert!(v.get("working_view").is_none());
    assert_eq!(v["project"]["id"].as_str().expect("project id"), "p1");

    // v3 — a calculator receipt (no dataset, no working view).
    let inputs = json!({"numerator": 3, "denominator": 5});
    let r = pin_engine(epiora_core::receipt::Receipt::for_calculator(
        CalculatorStamp {
            id: "proportion".into(),
            inputs: inputs.clone(),
        },
        "calc.proportion".into(),
        inputs,
        vec![],
        "2026-10-30T12:00:00Z".into(),
        EngineStamp {
            name: "epiora-core".into(),
            version: "1.0.0".into(),
            git_sha: "0000000".into(),
        },
    ));
    assert_eq!(r.receipt_version, 3);
    let v = serde_json::to_value(&r).expect("serialize");
    assert!(v.get("working_view").is_none());
    assert!(v.get("dataset").is_none());
}
