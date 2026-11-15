#![cfg(feature = "corpus")]

//! Calculator registry and input-validation tests (v1.0 Task 7).

use epiora_core::calc::{definition, input::validate, REGISTRY};
use epiora_core::error::EngineError;
use epiora_core::validation::REGISTRY as COVERAGE;
use serde_json::json;
use std::collections::BTreeSet;
use std::path::Path;

/// The calculator ids of `docs/v1.0-calculators.md` `## ` headings, in
/// inventory order. Non-calculator sections are filtered out because
/// their first word is not all-lowercase snake_case.
fn inventory_ids() -> Vec<String> {
    let doc = std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/v1.0-calculators.md"),
    )
    .expect("inventory doc");
    doc.lines()
        .filter_map(|line| {
            let heading = line.strip_prefix("## ")?;
            let id = heading.split_whitespace().next()?;
            (id.chars().all(|c| c.is_ascii_lowercase() || c == '_') && !id.is_empty())
                .then(|| id.to_string())
        })
        .collect()
}

#[test]
fn registry_ids_match_inventory() {
    let ids: Vec<&str> = REGISTRY.iter().map(|d| d.id).collect();
    let inventory = inventory_ids();
    assert_eq!(ids.len(), 24, "REGISTRY must list all 24 calculators");
    assert_eq!(ids, inventory);
}

#[test]
fn every_output_has_registry_coverage() {
    let registered: BTreeSet<&str> = COVERAGE.iter().map(|c| c.statistic).collect();
    let mut missing = Vec::new();
    // Non-counts families are registered but cannot emit yet — Tasks
    // 8/9 add their coverage with their implementations.
    for def in REGISTRY
        .iter()
        .filter(|d| d.family == epiora_core::calc::Family::Counts)
    {
        for out in def.outputs {
            if !registered.contains(out.statistic) {
                missing.push(format!("{}: {}", def.id, out.statistic));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "registry outputs missing from validation REGISTRY:\n{}",
        missing.join("\n")
    );
}

#[test]
fn defaults_fill_canonically() {
    let def = definition("proportion").expect("proportion def");
    let canonical =
        validate(def, &json!({"numerator": 3, "denominator": 5})).expect("valid inputs");
    assert_eq!(canonical["conf_level"], 95);
    assert_eq!(canonical["multiplier"], 100);
    assert_eq!(canonical["population_size"], 1_000_000);
    // `compare_with` is optional and stays absent when not supplied.
    assert!(canonical.get("compare_with").is_none());
}

#[test]
fn unknown_input_rejected() {
    let def = definition("proportion").expect("proportion def");
    let err = validate(def, &json!({"bogus": 1})).expect_err("unknown key");
    assert_eq!(err.code(), "calc_input_out_of_range");
    match err {
        EngineError::CalcInputOutOfRange { input, reason } => {
            assert_eq!(input, "bogus");
            assert_eq!(reason, "unknown_input");
        }
        other => panic!("expected CalcInputOutOfRange, got {other:?}"),
    }
}

#[test]
fn counts_above_2_pow_53_rejected() {
    let def = definition("proportion").expect("proportion def");
    let err = validate(
        def,
        &json!({"numerator": 9007199254740992.0, "denominator": 100}),
    )
    .expect_err("2^53 must be rejected");
    assert_eq!(err.code(), "calc_input_out_of_range");
}

#[test]
fn conf_level_only_90_95_99() {
    let def = definition("proportion").expect("proportion def");
    for ok in [90, 95, 99] {
        validate(
            def,
            &json!({"numerator": 3, "denominator": 5, "conf_level": ok}),
        )
        .unwrap_or_else(|e| panic!("conf_level {ok}: {e}"));
    }
    for bad in [80, 97, 0, 100] {
        let err = validate(
            def,
            &json!({"numerator": 3, "denominator": 5, "conf_level": bad}),
        )
        .expect_err("conf_level must be rejected");
        assert_eq!(err.code(), "calc_input_out_of_range");
    }
}

#[test]
fn table_shape_enforced() {
    let def = definition("two_by_two").expect("two_by_two def");
    // A 3-column stratum is rejected.
    let err = validate(def, &json!({"strata": [[1, 2, 3]]})).expect_err("3 cols must be rejected");
    assert_eq!(err.code(), "calc_input_out_of_range");
    // Zero strata is below min_rows.
    let err = validate(def, &json!({"strata": []})).expect_err("empty strata");
    assert_eq!(err.code(), "calc_input_out_of_range");
    // A non-integer cell is rejected.
    let err = validate(def, &json!({"strata": [[1, 2], [3, 4.5]]})).expect_err("fractional cell");
    assert_eq!(err.code(), "calc_input_out_of_range");
    // Ten strata exceeds the 9-stratum cap.
    let err = validate(
        def,
        &json!({"strata": [[1,2],[3,4],[1,2],[3,4],[1,2],[3,4],[1,2],[3,4],[1,2],[3,4]]}),
    )
    .expect_err("10 strata must be rejected");
    assert_eq!(err.code(), "calc_input_out_of_range");
    // A negative cell is rejected.
    let err = validate(def, &json!({"strata": [[1, 2], [3, -1]]})).expect_err("negative cell");
    assert_eq!(err.code(), "calc_input_out_of_range");
}
