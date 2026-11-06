#![cfg(feature = "corpus")]

use epiora_core::corpus::compare::compare;
use epiora_core::corpus::{load_dataset_dir, run_case};
use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::procedures::{
    freq, tables, FreqOptions, ProportionCi, RunContext, TablesOptions, VarSpec,
};
use proptest::prelude::*;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn datasets() -> BTreeMap<String, Dataset> {
    load_dataset_dir(&fixtures_root().join("data")).expect("corpus datasets")
}

fn load_fixtures(dir: &str) -> Vec<(String, Value)> {
    let mut out: Vec<(String, Value)> = std::fs::read_dir(fixtures_root().join(dir))
        .unwrap_or_else(|e| panic!("read fixtures/{dir}: {e}"))
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            let id = format!("{dir}/{}", p.file_stem().expect("stem").to_string_lossy());
            let fixture = serde_json::from_str(&std::fs::read_to_string(&p).expect("read fixture"))
                .expect("parse fixture");
            (id, fixture)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn assert_no_mismatches(id: &str, fixture: &Value, actual: &Value, failures: &mut Vec<String>) {
    match compare(fixture, actual) {
        Ok(mismatches) => {
            for m in mismatches {
                failures.push(format!("{id}: {m}"));
            }
        }
        Err(e) => failures.push(format!("{id}: {e}")),
    }
}

#[test]
fn every_fixture_passes_via_run_case() {
    let sets = datasets();
    let mut failures = Vec::new();
    let mut cases = 0;
    for dir in ["special", "2x2", "stratified", "rxc", "freq"] {
        for (id, fixture) in load_fixtures(dir) {
            cases += 1;
            match run_case(&fixture, &sets) {
                Ok(output) => {
                    assert_eq!(output.id, fixture["id"].as_str().expect("fixture id"));
                    assert_no_mismatches(&id, &fixture, &output.actual, &mut failures);
                }
                Err(e) => failures.push(format!("{id}: run_case failed: {e}")),
            }
        }
    }
    assert!(cases > 0, "no corpus fixtures found");
    assert!(
        failures.is_empty(),
        "{} failures in {cases} cases:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn oswego_tables_matches_two_by_two_fixture() {
    let sets = datasets();
    let fixture = load_fixtures("2x2")
        .into_iter()
        .find(|(id, _)| id == "2x2/oswego_vanilla")
        .expect("fixture")
        .1;
    let output = run_case(&fixture, &sets).expect("run_case");
    let mismatches = compare(&fixture, &output.actual).expect("compare");
    assert!(mismatches.is_empty(), "{mismatches:?}");
    assert!(output.receipt.is_some(), "dataset run carries a receipt");
}

#[test]
fn oswego_stratified_by_sex_matches_fixture() {
    let sets = datasets();
    let fixture = load_fixtures("stratified")
        .into_iter()
        .find(|(id, _)| id == "stratified/oswego_vanilla_by_sex")
        .expect("fixture")
        .1;
    let output = run_case(&fixture, &sets).expect("run_case");
    let mismatches = compare(&fixture, &output.actual).expect("compare");
    assert!(mismatches.is_empty(), "{mismatches:?}");
}

#[test]
fn freq_matches_fixtures() {
    let sets = datasets();
    let mut failures = Vec::new();
    for (id, fixture) in load_fixtures("freq") {
        let output = run_case(&fixture, &sets).expect("run_case");
        assert_no_mismatches(&id, &fixture, &output.actual, &mut failures);
        // `inputs.labels` is the fixture's record of the categories the
        // table must produce, in order — as raw JSON values. Compare
        // them against the engine's display labels (spec §4.1: Integer
        // as decimal, Text verbatim, "(missing)" last).
        let expected: Vec<String> = fixture["inputs"]["labels"]
            .as_array()
            .expect("inputs.labels")
            .iter()
            .map(|v| match v {
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(),
                other => other.to_string(),
            })
            .collect();
        let actual: Vec<String> = output.actual["labels"]
            .as_array()
            .expect("labels")
            .iter()
            .map(|v| v.as_str().expect("label is a string").to_string())
            .collect();
        assert_eq!(actual, expected, "{id}: category labels");
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

fn no_nulls(v: &Value, path: &str, problems: &mut Vec<String>) {
    match v {
        Value::Null => problems.push(format!("{path}: null in serialized result")),
        Value::Object(m) => {
            for (k, c) in m {
                no_nulls(c, &format!("{path}.{k}"), problems);
            }
        }
        Value::Array(items) => {
            for (i, c) in items.iter().enumerate() {
                no_nulls(c, &format!("{path}[{i}]"), problems);
            }
        }
        _ => {}
    }
}

fn spec(name: &str) -> VarSpec {
    VarSpec {
        name: name.into(),
        levels: None,
    }
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project: None,
    }
}

proptest! {
    /// Whatever the input data, serializing a result never produces a
    /// `null` (spec §4.1: `skip_serializing_if` on `Option`, `Estimate`
    /// never serializes a bare null).
    #[test]
    fn serialized_results_never_contain_null(
        rows in prop::collection::vec(
            (
                prop::option::of(prop::bool::ANY),
                prop::option::of(prop::bool::ANY),
                prop::option::of(prop::string::string_regex("[ab]{1,2}").expect("regex")),
            ),
            0..40,
        ),
        include_missing in prop::bool::ANY,
    ) {
        let ds = Dataset::new(
            "prop",
            1,
            vec![
                Column {
                    name: "E".into(),
                    data: ColumnData::Boolean(rows.iter().map(|r| r.0).collect()),
                },
                Column {
                    name: "I".into(),
                    data: ColumnData::Boolean(rows.iter().map(|r| r.1).collect()),
                },
                Column {
                    name: "G".into(),
                    data: ColumnData::Text(rows.iter().map(|r| r.2.clone()).collect()),
                },
            ],
        )
        .expect("dataset");

        let freq_opts = FreqOptions {
            variables: vec!["E".into(), "G".into()],
            stratify_by: None,
            include_missing,
            ci_method: ProportionCi::ClopperPearson,
        };
        let mut problems = Vec::new();
        if let Ok(out) = freq(&ds, &freq_opts, &ctx()) {
            let v = serde_json::to_value(&out.result).expect("serialize freq");
            no_nulls(&v, "freq", &mut problems);
        }

        let tables_opts = TablesOptions {
            exposure: spec("E"),
            outcome: spec("I"),
            stratify_by: vec!["G".into()],
            include_missing,
        };
        if let Ok(out) = tables(&ds, &tables_opts, &ctx()) {
            let v = serde_json::to_value(&out.result).expect("serialize tables");
            no_nulls(&v, "tables", &mut problems);
        }
        prop_assert!(problems.is_empty(), "{}", problems.join("\n"));
    }
}
