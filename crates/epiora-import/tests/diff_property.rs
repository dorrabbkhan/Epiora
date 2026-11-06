//! The re-import diff apply property (v0.3 spec §4.4, plan Task 5
//! Step 4): applying `diff`'s output to `previous`'s
//! `record_id → record JSON` map — independently of `next`'s id list —
//! reproduces `next`'s records map exactly.

mod common;

use common::{col, dataset, imported, key};
use epiora_core::dataset::{Column, ColumnData, Dataset, Value, ValueType};
use epiora_import::{diff, record_ids, record_json, Diff, FieldChange, Imported, RecordKey};
use proptest::prelude::*;
use std::collections::{BTreeMap, HashMap};

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

/// The records table's view of one side: `record_id → canonical JSON`.
fn records(columns: &[Column], key: &RecordKey) -> BTreeMap<String, String> {
    let ids = record_ids(columns, key).expect("record ids");
    ids.into_iter()
        .enumerate()
        .map(|(r, id)| (id, record_json(columns, r)))
        .collect()
}

/// Parses a `record_json` row back into its object map.
fn record_map(columns: &[Column], row: usize) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::from_str(&record_json(columns, row)).expect("record parses") {
        serde_json::Value::Object(m) => m,
        _ => panic!("record is not an object"),
    }
}

/// Applies `d` to `previous`'s `record_id → record JSON` map without
/// iterating `next`'s ids: `removed` ids must exist before deletion,
/// `removed_columns` and `changed` apply to the survivors, each
/// `added` index's id (from `next`) must be new before its record is
/// inserted, and columns `next` still lacks on a surviving record get
/// nulls. `next` supplies only the added rows' ids and records (the
/// diff carries just their indices). Used by the proptest and mirrored
/// by the `diff_any` fuzz harness's check.
fn apply_diff(
    previous: &Dataset,
    next: &Imported,
    key: &RecordKey,
    d: &Diff,
) -> BTreeMap<String, String> {
    let prev_ids = record_ids(previous.columns(), key).expect("prev ids");
    let next_ids = record_ids(&next.columns, key).expect("next ids");
    let mut map: BTreeMap<String, serde_json::Map<String, serde_json::Value>> = prev_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), record_map(previous.columns(), i)))
        .collect();
    for id in &d.removed {
        assert!(map.remove(id).is_some(), "removed id {id} not in previous");
    }
    let mut by_id: HashMap<&str, Vec<&FieldChange>> = HashMap::new();
    for fc in &d.changed {
        assert!(
            map.contains_key(fc.record_id.as_str()),
            "field change for {} names no remaining record",
            fc.record_id
        );
        by_id.entry(fc.record_id.as_str()).or_default().push(fc);
    }
    for (id, rec) in &mut map {
        for name in &d.schema.removed_columns {
            rec.remove(name);
        }
        if let Some(changes) = by_id.get(id.as_str()) {
            for fc in changes {
                let cell = match &fc.new {
                    Some(v) => serde_json::to_value(v).expect("value serializes"),
                    None => serde_json::Value::Null,
                };
                rec.insert(fc.field.clone(), cell);
            }
        }
    }
    for &j in &d.added {
        let id = &next_ids[j as usize];
        assert!(!map.contains_key(id), "added id {id} already present");
        map.insert(id.clone(), record_map(&next.columns, j as usize));
    }
    for rec in map.values_mut() {
        for c in &next.columns {
            rec.entry(c.name.clone()).or_insert(serde_json::Value::Null);
        }
    }
    map.into_iter()
        .map(|(id, m)| {
            (
                id,
                serde_json::to_string(&serde_json::Value::Object(m)).expect("map serializes"),
            )
        })
        .collect()
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
