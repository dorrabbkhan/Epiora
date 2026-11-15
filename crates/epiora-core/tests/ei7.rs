#![cfg(feature = "corpus")]

use epiora_core::corpus::ei7::compare_ei7;
use epiora_core::corpus::load_dataset;
use epiora_core::dataset::Dataset;
use epiora_core::procedures::{
    freq, means, tables, FreqOptions, MeansOptions, ProportionCi, RunContext, TablesOptions,
    TablesResult, VarSpec,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".to_string(),
        project: None,
    }
}

fn var(name: &str) -> VarSpec {
    VarSpec {
        name: name.to_string(),
        levels: None,
    }
}

/// `MEANS AGE SEX` on oswego, serialized for the `means` family of the
/// ei7 map: its `overall` covers the `MEANS AGE` keys as well (the two
/// runs' overalls are the same 75 records).
fn means_age_sex_json(ds: &Dataset) -> Value {
    let out = means(
        ds,
        &MeansOptions {
            variable: "AGE".to_string(),
            group_by: Some("SEX".to_string()),
            include_missing: false,
        },
        &ctx(),
    )
    .expect("means AGE SEX");
    serde_json::to_value(&out.result).expect("serialize means")
}

#[test]
fn oswego_matches_epi_info_7() {
    let ds = load_dataset(
        &std::fs::read_to_string(fixtures_root().join("data/oswego.dataset.json"))
            .expect("read oswego dataset"),
    )
    .expect("load oswego dataset");
    let ei7: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures_root().join("ei7/oswego.json")).expect("read ei7"),
    )
    .expect("parse ei7");

    let freq_out = freq(
        &ds,
        &FreqOptions {
            variables: vec!["SEX".to_string()],
            stratify_by: None,
            include_missing: false,
            ci_method: ProportionCi::ClopperPearson,
        },
        &ctx(),
    )
    .expect("freq SEX");

    let tables_out = tables(
        &ds,
        &TablesOptions {
            exposure: var("VANILLA"),
            outcome: var("ILL"),
            stratify_by: vec![],
            include_missing: false,
        },
        &ctx(),
    )
    .expect("tables VANILLA ILL");
    let TablesResult::TwoByTwo { crude, .. } = &tables_out.result else {
        panic!("VANILLA × ILL is not a 2×2")
    };

    let strat_out = tables(
        &ds,
        &TablesOptions {
            exposure: var("VANILLA"),
            outcome: var("ILL"),
            stratify_by: vec!["SEX".to_string()],
            include_missing: false,
        },
        &ctx(),
    )
    .expect("tables VANILLA ILL by SEX");
    let TablesResult::TwoByTwo {
        stratified: Some(s),
        ..
    } = &strat_out.result
    else {
        panic!("stratified run produced no strata")
    };

    let mut actual: BTreeMap<String, Value> = BTreeMap::new();
    actual.insert(
        "freq".to_string(),
        serde_json::to_value(&freq_out.result.tables[0]).expect("serialize freq"),
    );
    actual.insert(
        "two_by_two".to_string(),
        serde_json::to_value(crude).expect("serialize 2x2"),
    );
    actual.insert(
        "stratified_two_by_two".to_string(),
        serde_json::to_value(&s.stats).expect("serialize stratified"),
    );
    actual.insert("means".to_string(), means_age_sex_json(&ds));

    let report = compare_ei7(&ei7, &actual).expect("compare_ei7");
    assert!(report.checked >= 20, "checked {}", report.checked);
    assert!(
        report.mismatches.is_empty(),
        "mismatches:\n{}",
        report
            .mismatches
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The `means.*` keys `labels.json` maps from the two MEANS commands,
/// compared at `ei7_display` (v1.0 Task 6): the grouped run's
/// serialized result carries the overall block the `MEANS AGE` keys
/// name.
#[test]
fn ei7_means_matches_at_display() {
    let ds = load_dataset(
        &std::fs::read_to_string(fixtures_root().join("data/oswego.dataset.json"))
            .expect("read oswego dataset"),
    )
    .expect("load oswego dataset");
    let mut ei7: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures_root().join("ei7/oswego.json")).expect("read ei7"),
    )
    .expect("parse ei7");
    let outputs = ei7["outputs"].as_array_mut().expect("ei7 outputs");
    outputs.retain(|o| {
        o.get("command")
            .and_then(Value::as_str)
            .is_some_and(|c| c.starts_with("MEANS"))
    });
    assert_eq!(outputs.len(), 2, "MEANS AGE and MEANS AGE SEX");

    let mut actual: BTreeMap<String, Value> = BTreeMap::new();
    actual.insert("means".to_string(), means_age_sex_json(&ds));

    let report = compare_ei7(&ei7, &actual).expect("compare_ei7");
    assert!(report.checked >= 59, "checked {}", report.checked);
    assert!(
        report.mismatches.is_empty(),
        "mismatches:\n{}",
        report
            .mismatches
            .iter()
            .map(|m| m.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn display_rule_boundaries() {
    let ei7 = |value: f64| {
        json!({"outputs": [{"command": "TABLES", "values": [
            {"label": "OR", "text": "23.4545", "value": value, "decimals": 4,
             "key": "two_by_two.odds_ratio.estimate"}
        ]}]})
    };
    let mut actual: BTreeMap<String, Value> = BTreeMap::new();
    actual.insert(
        "two_by_two".to_string(),
        json!({"odds_ratio": {"estimate": 23.45454}}),
    );
    let report = compare_ei7(&ei7(23.4545), &actual).expect("compare_ei7");
    assert_eq!(report.checked, 1);
    assert!(report.mismatches.is_empty(), "{:?}", report.mismatches);

    actual.insert(
        "two_by_two".to_string(),
        json!({"odds_ratio": {"estimate": 23.45456}}),
    );
    let report = compare_ei7(&ei7(23.4545), &actual).expect("compare_ei7");
    assert_eq!(report.mismatches.len(), 1);
}
