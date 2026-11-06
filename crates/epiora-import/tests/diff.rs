//! Re-import diff tests (v0.3 spec §4.2/§4.4/§6.4, plan Task 5): keyed
//! and positional diffs, field-level changes, schema changes, duplicate
//! or missing keys refused and summary sampling. The apply property —
//! applying the diff to `previous`'s records yields `next`'s records —
//! lives in `diff_property.rs`.

mod common;

use common::{col, dataset, imported, key};
use epiora_core::dataset::{ColumnData, Value, ValueType};
use epiora_import::{diff, summarize, FieldChange, RecordKey};

#[test]
fn identical_is_unchanged() {
    let columns = vec![
        col("id", ColumnData::Integer(vec![Some(1), Some(2)])),
        col(
            "x",
            ColumnData::Text(vec![Some("a".to_string()), Some("b".to_string())]),
        ),
    ];
    let d = diff(&dataset(columns.clone()), &imported(columns), &key("id")).expect("diff");
    assert!(d.unchanged);
    assert!(d.added.is_empty() && d.removed.is_empty() && d.changed.is_empty());
    assert!(
        d.schema.added_columns.is_empty()
            && d.schema.removed_columns.is_empty()
            && d.schema.type_changes.is_empty()
    );
}

#[test]
fn added_changed_removed_by_key() {
    let prev = dataset(vec![
        col("id", ColumnData::Integer(vec![Some(1), Some(2), Some(3)])),
        col(
            "x",
            ColumnData::Text(vec![
                Some("a".to_string()),
                Some("b".to_string()),
                Some("c".to_string()),
            ]),
        ),
    ]);
    let next = imported(vec![
        col("id", ColumnData::Integer(vec![Some(1), Some(2), Some(4)])),
        col(
            "x",
            ColumnData::Text(vec![
                Some("a".to_string()),
                Some("B".to_string()),
                Some("d".to_string()),
            ]),
        ),
    ]);
    let d = diff(&prev, &next, &key("id")).expect("diff");
    assert!(!d.unchanged);
    assert_eq!(d.added, vec![2]);
    assert_eq!(d.removed, vec!["3".to_string()]);
    assert_eq!(
        d.changed,
        vec![FieldChange {
            record_id: "2".to_string(),
            row: 1,
            field: "x".to_string(),
            old: Some(Value::Text("b".to_string())),
            new: Some(Value::Text("B".to_string())),
        }]
    );
}

#[test]
fn change_to_missing_and_back() {
    let prev = dataset(vec![
        col("id", ColumnData::Integer(vec![Some(1), Some(2)])),
        col("x", ColumnData::Integer(vec![Some(5), None])),
    ]);
    let next = imported(vec![
        col("id", ColumnData::Integer(vec![Some(1), Some(2)])),
        col("x", ColumnData::Integer(vec![None, Some(9)])),
    ]);
    let d = diff(&prev, &next, &key("id")).expect("diff");
    assert_eq!(
        d.changed,
        vec![
            FieldChange {
                record_id: "1".to_string(),
                row: 0,
                field: "x".to_string(),
                old: Some(Value::Integer(5)),
                new: None,
            },
            FieldChange {
                record_id: "2".to_string(),
                row: 1,
                field: "x".to_string(),
                old: None,
                new: Some(Value::Integer(9)),
            },
        ]
    );
}

#[test]
fn positional_diff() {
    let prev = dataset(vec![col(
        "x",
        ColumnData::Text(vec![
            Some("a".to_string()),
            Some("b".to_string()),
            Some("c".to_string()),
        ]),
    )]);
    let next = imported(vec![
        col(
            "x",
            ColumnData::Text(vec![
                Some("a".to_string()),
                Some("B".to_string()),
                Some("c".to_string()),
                Some("d".to_string()),
            ]),
        ),
        col("y", ColumnData::Integer(vec![Some(7), None, None, None])),
    ]);
    let d = diff(&prev, &next, &RecordKey::RowPosition).expect("diff");
    assert_eq!(d.added, vec![3]);
    assert!(d.removed.is_empty());
    assert_eq!(d.schema.added_columns, vec!["y".to_string()]);
    assert_eq!(
        d.changed,
        vec![
            FieldChange {
                record_id: "row:1".to_string(),
                row: 0,
                field: "y".to_string(),
                old: None,
                new: Some(Value::Integer(7)),
            },
            FieldChange {
                record_id: "row:2".to_string(),
                row: 1,
                field: "x".to_string(),
                old: Some(Value::Text("b".to_string())),
                new: Some(Value::Text("B".to_string())),
            },
        ]
    );
}

#[test]
fn column_added_removed_and_type_changed_reported() {
    let prev = dataset(vec![
        col("id", ColumnData::Integer(vec![Some(1), Some(2)])),
        col(
            "m",
            ColumnData::Text(vec![Some("1".to_string()), Some("2".to_string())]),
        ),
        col("z", ColumnData::Integer(vec![Some(9), Some(8)])),
    ]);
    let next = imported(vec![
        col("id", ColumnData::Integer(vec![Some(1), Some(2)])),
        col("m", ColumnData::Real(vec![Some(1.0), Some(2.0)])),
        col("y", ColumnData::Integer(vec![Some(4), Some(5)])),
    ]);
    let d = diff(&prev, &next, &key("id")).expect("diff");
    assert!(!d.unchanged);
    assert_eq!(d.schema.added_columns, vec!["y".to_string()]);
    assert_eq!(d.schema.removed_columns, vec!["z".to_string()]);
    assert_eq!(d.schema.type_changes.len(), 1);
    assert_eq!(d.schema.type_changes[0].column, "m");
    assert_eq!(d.schema.type_changes[0].from, ValueType::Text);
    assert_eq!(d.schema.type_changes[0].to, ValueType::Real);
    // The type-changed column's cells differ as typed values, so every
    // populated row reports a field change; the added column's values
    // appear as added fields (old: None).
    assert_eq!(
        d.changed,
        vec![
            FieldChange {
                record_id: "1".to_string(),
                row: 0,
                field: "m".to_string(),
                old: Some(Value::Text("1".to_string())),
                new: Some(Value::Real(1.0)),
            },
            FieldChange {
                record_id: "1".to_string(),
                row: 0,
                field: "y".to_string(),
                old: None,
                new: Some(Value::Integer(4)),
            },
            FieldChange {
                record_id: "2".to_string(),
                row: 1,
                field: "m".to_string(),
                old: Some(Value::Text("2".to_string())),
                new: Some(Value::Real(2.0)),
            },
            FieldChange {
                record_id: "2".to_string(),
                row: 1,
                field: "y".to_string(),
                old: None,
                new: Some(Value::Integer(5)),
            },
        ]
    );
}

#[test]
fn reimport_with_new_duplicate_key_is_refused() {
    let prev = dataset(vec![
        col("id", ColumnData::Integer(vec![Some(1), Some(2), Some(3)])),
        col(
            "x",
            ColumnData::Text(vec![
                Some("a".to_string()),
                Some("b".to_string()),
                Some("c".to_string()),
            ]),
        ),
    ]);
    // The re-import's key column gained a duplicate.
    let dup = imported(vec![
        col("id", ColumnData::Integer(vec![Some(1), Some(2), Some(2)])),
        col(
            "x",
            ColumnData::Text(vec![
                Some("a".to_string()),
                Some("b".to_string()),
                Some("c".to_string()),
            ]),
        ),
    ]);
    match diff(&prev, &dup, &key("id")) {
        Err(e) => {
            assert_eq!(e.code(), "duplicate_key");
            match e {
                epiora_import::ImportError::DuplicateKey {
                    key,
                    first_line,
                    line,
                } => {
                    assert_eq!(key, "2");
                    assert_eq!(first_line, 2);
                    assert_eq!(line, 3);
                }
                other => panic!("expected DuplicateKey, got {other:?}"),
            }
        }
        Ok(_) => panic!("duplicate key must refuse the diff"),
    }
    // …and one with a missing (empty) key cell.
    let missing = imported(vec![
        col("id", ColumnData::Integer(vec![Some(1), None, Some(3)])),
        col(
            "x",
            ColumnData::Text(vec![
                Some("a".to_string()),
                Some("b".to_string()),
                Some("c".to_string()),
            ]),
        ),
    ]);
    match diff(&prev, &missing, &key("id")) {
        Err(e) => {
            assert_eq!(e.code(), "missing_key");
            match e {
                epiora_import::ImportError::MissingKey { line } => assert_eq!(line, 2),
                other => panic!("expected MissingKey, got {other:?}"),
            }
        }
        Ok(_) => panic!("missing key must refuse the diff"),
    }
}

#[test]
fn summary_caps_samples_at_20() {
    let n = 30i64;
    let prev = dataset(vec![
        col("id", ColumnData::Integer((0..n).map(Some).collect())),
        col("x", ColumnData::Integer(vec![Some(0); n as usize])),
    ]);
    let next = imported(vec![
        col("id", ColumnData::Integer((0..n).map(Some).collect())),
        col("x", ColumnData::Integer(vec![Some(1); n as usize])),
    ]);
    let d = diff(&prev, &next, &key("id")).expect("diff");
    let s = summarize(&d);
    assert_eq!(s.added, 0);
    assert_eq!(s.removed, 0);
    assert_eq!(s.changed, n as u64);
    assert!(!s.unchanged);
    assert_eq!(s.samples.len(), 1);
    assert_eq!(s.samples["x"].len(), 20);
    assert_eq!(s.samples["x"][0].record_id, "0");
    assert_eq!(s.samples["x"][19].record_id, "19");
}
