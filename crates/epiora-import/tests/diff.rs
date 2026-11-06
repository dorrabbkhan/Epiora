//! Re-import diff tests (v0.3 spec §4.2/§4.4/§6.4, plan Task 5): keyed
//! and positional diffs, field-level changes, schema changes, duplicate
//! or missing keys refused, summary sampling, and the apply property —
//! applying the diff to `previous`'s records yields `next`'s records.

use epiora_core::dataset::{Column, ColumnData, Dataset, Value, ValueType};
use epiora_import::{
    diff, record_ids, record_json, summarize, Container, DecimalSep, Detection, Diff, Encoding,
    FieldChange, ImportSettings, Imported, RecordKey,
};
use proptest::prelude::*;
use std::collections::{BTreeMap, HashMap};

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

fn dataset(columns: Vec<Column>) -> Dataset {
    Dataset::new("d", 1, columns).expect("dataset")
}

/// Wraps columns in an `Imported`; only `columns` and `row_count` are
/// read by `diff`.
fn imported(columns: Vec<Column>) -> Imported {
    let row_count = columns.first().map(|c| c.data.len() as u64).unwrap_or(0);
    Imported {
        columns,
        detection: Detection {
            container: Container::Text,
            encoding: Some(Encoding::Utf8),
            delimiter: Some(','),
            decimal: DecimalSep::Dot,
            header_renames: Vec::new(),
            sheets: Vec::new(),
        },
        settings: ImportSettings::default(),
        coercions: BTreeMap::new(),
        row_count,
        source_sha256: String::new(),
    }
}

fn key(name: &str) -> RecordKey {
    RecordKey::Column {
        name: name.to_string(),
    }
}

/// The records table's view of one side: `record_id → canonical JSON`.
fn records(columns: &[Column], key: &RecordKey) -> BTreeMap<String, String> {
    let ids = record_ids(columns, key).expect("record ids");
    ids.into_iter()
        .enumerate()
        .map(|(r, id)| (id, record_json(columns, r)))
        .collect()
}

/// Applies `d` to `previous`'s records; `next` supplies the added rows'
/// records (the diff carries only their row indices). Used by the
/// proptest and the same check the `diff_any` fuzz harness makes.
fn apply_diff(
    previous: &Dataset,
    next: &Imported,
    key: &RecordKey,
    d: &Diff,
) -> BTreeMap<String, String> {
    let prev_ids = record_ids(previous.columns(), key).expect("prev ids");
    let next_ids = record_ids(&next.columns, key).expect("next ids");
    let prev_pos: HashMap<&str, usize> = prev_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let mut by_id: HashMap<&str, Vec<&FieldChange>> = HashMap::new();
    for fc in &d.changed {
        by_id.entry(fc.record_id.as_str()).or_default().push(fc);
    }
    let mut out = BTreeMap::new();
    for (j, id) in next_ids.iter().enumerate() {
        let json = if d.added.contains(&(j as u32)) {
            record_json(&next.columns, j)
        } else {
            let i = *prev_pos
                .get(id.as_str())
                .unwrap_or_else(|| panic!("shared id {id} not in previous"));
            let raw = record_json(previous.columns(), i);
            let serde_json::Value::Object(mut map) =
                serde_json::from_str::<serde_json::Value>(&raw).expect("record is an object")
            else {
                panic!("record is not an object");
            };
            for name in &d.schema.removed_columns {
                map.remove(name);
            }
            if let Some(changes) = by_id.get(id.as_str()) {
                for fc in changes {
                    let cell = match &fc.new {
                        Some(v) => serde_json::to_value(v).expect("value serializes"),
                        None => serde_json::Value::Null,
                    };
                    map.insert(fc.field.clone(), cell);
                }
            }
            // A column added by the schema with no recorded value is
            // missing in this record.
            for c in &next.columns {
                map.entry(c.name.clone()).or_insert(serde_json::Value::Null);
            }
            serde_json::to_string(&map).expect("map serializes")
        };
        out.insert(id.clone(), json);
    }
    out
}

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

/// `Vec<Option<Value>>` (all variants matching `ty`) → `ColumnData`.
fn data_of(ty: ValueType, cells: Vec<Option<Value>>) -> ColumnData {
    match ty {
        ValueType::Integer => ColumnData::Integer(
            cells
                .into_iter()
                .map(|c| {
                    c.and_then(|v| match v {
                        Value::Integer(i) => Some(i),
                        _ => None,
                    })
                })
                .collect(),
        ),
        ValueType::Real => ColumnData::Real(
            cells
                .into_iter()
                .map(|c| {
                    c.and_then(|v| match v {
                        Value::Real(x) => Some(x),
                        _ => None,
                    })
                })
                .collect(),
        ),
        ValueType::Text => ColumnData::Text(
            cells
                .into_iter()
                .map(|c| {
                    c.and_then(|v| match v {
                        Value::Text(s) => Some(s),
                        _ => None,
                    })
                })
                .collect(),
        ),
        ValueType::Boolean => ColumnData::Boolean(
            cells
                .into_iter()
                .map(|c| {
                    c.and_then(|v| match v {
                        Value::Boolean(b) => Some(b),
                        _ => None,
                    })
                })
                .collect(),
        ),
        ValueType::Date => ColumnData::Date(
            cells
                .into_iter()
                .map(|c| {
                    c.and_then(|v| match v {
                        Value::Date(d) => Some(d),
                        _ => None,
                    })
                })
                .collect(),
        ),
    }
}

fn atom(ty: ValueType) -> BoxedStrategy<Value> {
    match ty {
        ValueType::Integer => (-100i64..100).prop_map(Value::Integer).boxed(),
        ValueType::Real => prop::sample::select(vec![0.0f64, -0.0, 0.5, 1.5, -2.25])
            .prop_map(Value::Real)
            .boxed(),
        ValueType::Text => prop::sample::select(vec!["a", "b", "c", ""])
            .prop_map(|s| Value::Text(s.to_string()))
            .boxed(),
        ValueType::Boolean => any::<bool>().prop_map(Value::Boolean).boxed(),
        ValueType::Date => (-30_000i32..40_000).prop_map(Value::Date).boxed(),
    }
}

fn value_type() -> impl Strategy<Value = ValueType> {
    prop::sample::select(vec![
        ValueType::Integer,
        ValueType::Real,
        ValueType::Text,
        ValueType::Boolean,
        ValueType::Date,
    ])
}

/// One side of a diff: an `id` column of unique integers plus up to 3
/// extra columns named from a small pool (so both sides often share
/// names, sometimes with different types). Row count 0–11.
fn arb_side() -> impl Strategy<Value = Vec<Column>> {
    (
        prop::collection::hash_set(0i64..80, 0..12),
        prop::collection::hash_set(prop::sample::select(vec!["x", "y", "z", "w"]), 0..=3),
    )
        .prop_flat_map(|(ids, names)| {
            let mut ids: Vec<i64> = ids.into_iter().collect();
            ids.sort_unstable();
            let mut names: Vec<String> = names.into_iter().map(str::to_string).collect();
            names.sort();
            let n = ids.len();
            let count = names.len();
            (
                Just(ids),
                Just(names),
                prop::collection::vec(
                    value_type().prop_flat_map(move |ty| {
                        (
                            Just(ty),
                            prop::collection::vec(prop::option::weighted(0.8, atom(ty)), n..=n),
                        )
                    }),
                    count..=count,
                ),
            )
        })
        .prop_map(|(ids, names, extras)| {
            let mut columns = vec![col(
                "id",
                ColumnData::Integer(ids.into_iter().map(Some).collect()),
            )];
            for ((ty, cells), name) in extras.into_iter().zip(names) {
                columns.push(col(&name, data_of(ty, cells)));
            }
            columns
        })
}

proptest! {
    /// Applying `diff`'s output to `previous`'s records must reproduce
    /// `next`'s records exactly (plan Task 5 Step 4's fuzz property,
    /// checked here with a deterministic proptest).
    #[test]
    fn proptest_diff_applied_to_previous_gives_next(
        prev_cols in arb_side(),
        next_cols in arb_side(),
        positional in any::<bool>(),
    ) {
        let prev = dataset(prev_cols);
        let next = imported(next_cols);
        let k = if positional {
            RecordKey::RowPosition
        } else {
            key("id")
        };
        let d = diff(&prev, &next, &k).expect("ids are unique and present");
        let applied = apply_diff(&prev, &next, &k, &d);
        let next_records = records(&next.columns, &k);
        let prev_records = records(prev.columns(), &k);
        let same_records = applied.len() == prev_records.len()
            && applied.iter().all(|(id, json)| prev_records.get(id) == Some(json));
        prop_assert_eq!(applied, next_records);
        prop_assert_eq!(d.unchanged, same_records
            && d.added.is_empty() && d.removed.is_empty() && d.changed.is_empty()
            && d.schema.added_columns.is_empty() && d.schema.removed_columns.is_empty()
            && d.schema.type_changes.is_empty());
    }
}
