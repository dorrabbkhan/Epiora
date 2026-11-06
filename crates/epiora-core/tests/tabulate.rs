#![cfg(feature = "corpus")]

use epiora_core::corpus::load_dataset_dir;
use epiora_core::dataset::{Column, ColumnData, Dataset, Value};
use epiora_core::procedures::{freq, tables, FreqOptions, RunContext, TablesOptions, VarSpec};
use epiora_core::procedures::{ProportionCi, TablesResult};
use epiora_core::tabulate::{cross, frequency};
use epiora_core::EngineError;
use std::collections::BTreeMap;
use std::path::Path;

fn datasets() -> BTreeMap<String, Dataset> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/data");
    load_dataset_dir(&dir).expect("corpus datasets")
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

#[test]
fn exclusions_counted_per_variable() {
    let sets = datasets();
    let ds = sets.get("missing").expect("missing dataset");

    // TABLES: a record is excluded when any used variable is missing.
    // missing.dataset.json: EXPOSED missing at rows 4, 14, 22 (3);
    // ILL missing at rows 0, 9, 19, 23 (4); the sets are disjoint, so
    // 17 records reach the table [[0, 8], [9, 0]].
    let xt = cross(ds, &spec("EXPOSED"), &spec("ILL"), &[], false).expect("cross");
    assert_eq!(xt.exclusions.get("EXPOSED"), Some(&3));
    assert_eq!(xt.exclusions.get("ILL"), Some(&4));
    assert_eq!(xt.counts.len(), 1);
    assert_eq!(xt.counts[0], vec![vec![0, 8], vec![9, 0]]);

    // FREQ: a record missing the analysed variable is reported in
    // `missing`, not excluded, so the variable's exclusion count is 0.
    let out = freq(
        ds,
        &FreqOptions {
            variables: vec!["GROUP".into()],
            stratify_by: None,
            include_missing: false,
            ci_method: ProportionCi::ClopperPearson,
        },
        &ctx(),
    )
    .expect("freq");
    assert_eq!(out.result.exclusions.get("GROUP"), Some(&0));
    assert_eq!(out.result.tables[0].missing, 4);
    assert_eq!(out.result.tables[0].total, 24);

    // FREQ with a stratum: records missing the stratum variable are
    // excluded from every table and counted under that variable.
    let out = freq(
        ds,
        &FreqOptions {
            variables: vec!["GROUP".into()],
            stratify_by: Some("EXPOSED".into()),
            include_missing: false,
            ci_method: ProportionCi::ClopperPearson,
        },
        &ctx(),
    )
    .expect("stratified freq");
    assert_eq!(out.result.exclusions.get("EXPOSED"), Some(&3));
    assert_eq!(out.result.exclusions.get("GROUP"), Some(&0));
    // EXPOSED = Yes, No → two stratum tables, in category order.
    assert_eq!(out.result.tables.len(), 2);
}

#[test]
fn include_missing_adds_missing_category() {
    let sets = datasets();
    let ds = sets.get("missing").expect("missing dataset");

    let counts = frequency(ds, "GROUP", true).expect("frequency");
    assert_eq!(counts.labels, vec!["a", "b", "c", "(missing)"]);
    assert_eq!(counts.counts, vec![7, 5, 8, 4]);
    assert_eq!(counts.missing, 4);

    let counts = frequency(ds, "GROUP", false).expect("frequency");
    assert_eq!(counts.labels, vec!["a", "b", "c"]);
    assert_eq!(counts.counts, vec![7, 5, 8]);
    assert_eq!(counts.missing, 4);
}

#[test]
fn two_level_text_without_levels_is_ambiguous() {
    let sets = datasets();
    let ds = sets.get("text_levels").expect("text_levels dataset");
    let err = cross(ds, &spec("T2"), &spec("ILL"), &[], false).unwrap_err();
    assert_eq!(
        err,
        EngineError::AmbiguousLevels {
            variable: "T2".into()
        }
    );
}

#[test]
fn explicit_levels_orient_text() {
    let sets = datasets();
    let ds = sets.get("text_levels").expect("text_levels dataset");
    let exposure = VarSpec {
        name: "T2".into(),
        levels: Some([Value::Text("b".into()), Value::Text("a".into())]),
    };
    // T2 == "b" rows are all ILL = No; T2 == "a" rows are all ill.
    let xt = cross(ds, &exposure, &spec("ILL"), &[], false).expect("cross");
    assert_eq!(xt.row_labels, vec!["b", "a"]);
    assert_eq!(xt.col_labels, vec!["Yes", "No"]);
    assert_eq!(xt.counts[0], vec![vec![0, 12], vec![11, 0]]);
}

#[test]
fn boolean_and_01_coding_agree() {
    let sets = datasets();
    let ds = sets.get("coding").expect("coding dataset");
    let opts = |e: &str, i: &str| TablesOptions {
        exposure: spec(e),
        outcome: spec(i),
        stratify_by: vec![],
        include_missing: false,
    };
    let bool_run = tables(ds, &opts("E_BOOL", "I_BOOL"), &ctx()).expect("bool tables");
    let int_run = tables(ds, &opts("E_INT", "I_INT"), &ctx()).expect("int tables");
    match (bool_run.result, int_run.result) {
        (TablesResult::TwoByTwo { crude: b, .. }, TablesResult::TwoByTwo { crude: i, .. }) => {
            assert_eq!(b, i);
        }
        (b, i) => panic!("expected two-by-two results, got {b:?} / {i:?}"),
    }
}

#[test]
fn three_levels_gives_rxc() {
    let sets = datasets();
    let ds = sets.get("text_levels").expect("text_levels dataset");
    let xt = cross(ds, &spec("T3"), &spec("ILL"), &[], false).expect("cross");
    assert_eq!(xt.row_labels, vec!["x", "y", "z"]);
    assert_eq!(xt.col_labels, vec!["Yes", "No"]);
    assert_eq!(xt.counts[0], vec![vec![3, 4], vec![4, 4], vec![4, 4]]);
    assert_eq!(xt.exclusions.get("T3"), Some(&1));

    let out = tables(
        ds,
        &TablesOptions {
            exposure: spec("T3"),
            outcome: spec("ILL"),
            stratify_by: vec![],
            include_missing: false,
        },
        &ctx(),
    )
    .expect("tables");
    match out.result {
        TablesResult::RxC {
            row_labels,
            col_labels,
            counts,
            ..
        } => {
            assert_eq!(row_labels, vec!["x", "y", "z"]);
            assert_eq!(col_labels, vec!["Yes", "No"]);
            assert_eq!(counts, vec![vec![3, 4], vec![4, 4], vec![4, 4]]);
        }
        other => panic!("expected R×C result, got {other:?}"),
    }
}

#[test]
fn unknown_variable_errors() {
    let sets = datasets();
    let ds = sets.get("missing").expect("missing dataset");
    let err = frequency(ds, "NOPE", false).unwrap_err();
    assert_eq!(
        err,
        EngineError::UnknownVariable {
            name: "NOPE".into()
        }
    );
    let err = cross(ds, &spec("NOPE"), &spec("ILL"), &[], false).unwrap_err();
    assert_eq!(
        err,
        EngineError::UnknownVariable {
            name: "NOPE".into()
        }
    );
}

#[test]
fn zero_rows_gives_undefined_not_error() {
    let ds = Dataset::new(
        "empty",
        1,
        vec![
            Column {
                name: "E".into(),
                data: ColumnData::Boolean(vec![]),
            },
            Column {
                name: "I".into(),
                data: ColumnData::Boolean(vec![]),
            },
        ],
    )
    .expect("zero-row dataset is valid");

    let out = tables(
        &ds,
        &TablesOptions {
            exposure: spec("E"),
            outcome: spec("I"),
            stratify_by: vec![],
            include_missing: false,
        },
        &ctx(),
    )
    .expect("tables on zero rows is not an error");
    let actual = serde_json::to_value(&out.result).expect("serialize");
    assert_eq!(
        actual["rx_c"]["stats"]["chi_square"]["statistic"]["undefined"],
        "empty_table"
    );

    let out = freq(
        &ds,
        &FreqOptions {
            variables: vec!["E".into()],
            stratify_by: None,
            include_missing: false,
            ci_method: ProportionCi::ClopperPearson,
        },
        &ctx(),
    )
    .expect("freq on zero rows is not an error");
    let actual = serde_json::to_value(&out.result).expect("serialize");
    assert_eq!(actual["tables"][0]["total"], 0);
}
