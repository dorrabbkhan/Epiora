//! Dataset model v2 procedure tests (v1.1 plan Task 6, spec §4.3):
//! how FREQ, TABLES and MEANS behave on the `Time`/`DateTime` column
//! types and on columns carrying label sets. Split from
//! `dataset_v2.rs` to keep both files under the 400-line cap.

use epiora_core::dataset::{Column, ColumnData, ColumnMeta, Dataset, LabelSet, Value};
use epiora_core::procedures::{freq, tables, FreqOptions, RunContext, TablesOptions, VarSpec};
use epiora_core::EngineError;
use std::collections::BTreeMap;

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

fn meta(
    prompt: Option<&str>,
    legacy_type: Option<&str>,
    legacy_name: Option<&str>,
    label_set: Option<&str>,
) -> ColumnMeta {
    ColumnMeta {
        prompt: prompt.map(str::to_string),
        legacy_type: legacy_type.map(str::to_string),
        legacy_name: legacy_name.map(str::to_string),
        label_set: label_set.map(str::to_string),
    }
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-11-23T00:00:00Z".into(),
        project: None,
    }
}

#[test]
fn freq_shows_labels_in_category_order() {
    let ds = Dataset::new(
        "d",
        1,
        vec![col(
            "ILL",
            ColumnData::Integer(vec![Some(1), Some(0), Some(1), Some(2), None]),
        )],
    )
    .expect("dataset")
    .with_meta(
        vec![meta(Some("Were you ill?"), None, None, Some("ill"))],
        BTreeMap::from([(
            "ill".to_string(),
            LabelSet {
                id: "ill".to_string(),
                entries: vec![
                    (Value::Integer(0), "well".to_string()),
                    (Value::Integer(1), "ill".to_string()),
                ],
            },
        )]),
    )
    .expect("with_meta");
    let out = freq(
        &ds,
        &FreqOptions {
            variables: vec!["ILL".to_string()],
            stratify_by: None,
            include_missing: true,
            ci_method: Default::default(),
        },
        &ctx(),
    )
    .expect("freq");
    let table = &out.result.tables[0];
    // Category order is the stored values' order (0, 1, 2, missing);
    // the label rides beside it — not a replacement.
    assert_eq!(table.labels, vec!["0", "1", "2", "(missing)"]);
    assert_eq!(table.value_labels, vec!["well", "ill", "", ""]);
    assert_eq!(
        table.categories.iter().map(|c| c.count).collect::<Vec<_>>(),
        vec![1, 2, 1, 1]
    );
}

#[test]
fn tables_on_datetime_column_errors_typed() {
    let ds = Dataset::new(
        "d",
        1,
        vec![
            col(
                "STAMP",
                ColumnData::DateTime(vec![Some(1_700_000_000), Some(1_700_000_100)]),
            ),
            col("ILL", ColumnData::Boolean(vec![Some(true), Some(false)])),
        ],
    )
    .expect("dataset");
    let err = tables(
        &ds,
        &TablesOptions {
            exposure: VarSpec {
                name: "STAMP".to_string(),
                levels: None,
            },
            outcome: VarSpec {
                name: "ILL".to_string(),
                levels: None,
            },
            stratify_by: vec![],
            include_missing: false,
        },
        &ctx(),
    )
    .unwrap_err();
    assert!(matches!(err, EngineError::TypeMismatch { .. }), "{err}");
    assert_eq!(err.code(), "type_mismatch");
}

#[test]
fn means_accepts_time_and_datetime() {
    for (name, data, expected_sum) in [
        (
            "T",
            ColumnData::Time(vec![Some(3_600), Some(7_200), None]),
            10_800.0,
        ),
        (
            "DT",
            ColumnData::DateTime(vec![Some(86_400), Some(172_800), None]),
            259_200.0,
        ),
    ] {
        let ds = Dataset::new("d", 1, vec![col(name, data)]).expect("dataset");
        let out = epiora_core::procedures::means(
            &ds,
            &epiora_core::procedures::MeansOptions {
                variable: name.to_string(),
                group_by: None,
                include_missing: false,
            },
            &ctx(),
        )
        .expect("means");
        match out.result.overall.mean {
            epiora_core::stats::Estimate::Value(value) => {
                assert_eq!(value, expected_sum / 2.0);
            }
            other => panic!("expected a defined mean, got {other:?}"),
        }
    }
}
