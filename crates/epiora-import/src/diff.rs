//! The re-import diff (v0.3 spec §4.2/§4.4/§6.4): `previous` (the
//! committed dataset) against `next` (a fresh import), keyed by
//! [`RecordKey`]. Records are matched by id; fields compare by `Value`
//! equality with `-0.0` already normalised by `record_json`'s cell
//! extraction. Columns only in `next` report their populated values as
//! `old: None` field changes — the store needs them to write `update`
//! entries — while removed columns are reported once in
//! [`SchemaChanges`]. An identical file diffs to `unchanged`.

use crate::error::ImportError;
use crate::keys::record_ids;
use crate::records::cell_value;
use crate::settings::RecordKey;
use crate::Imported;
use epiora_core::dataset::{Dataset, Value, ValueType};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/// The record-level and schema-level differences between two imports.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Diff {
    /// Row indices in `next` whose record id is new, in `next` order.
    pub added: Vec<u32>,
    /// Record ids present in `previous` but not `next`, in `previous`
    /// order.
    pub removed: Vec<String>,
    /// Field-level changes on records present in both, in `next` row
    /// order then `next` column order.
    pub changed: Vec<FieldChange>,
    /// Column-level differences.
    pub schema: SchemaChanges,
    /// `true` when re-importing created no record or schema change
    /// (spec §4.2: an unchanged file creates no version).
    pub unchanged: bool,
}

/// One field that differs for a record present on both sides. `old` is
/// `None` when the field was missing (or the column is new); `new` is
/// `None` when it became missing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct FieldChange {
    /// The record's key id.
    pub record_id: String,
    /// The record's row index in `next`.
    pub row: u32,
    /// The changed column.
    pub field: String,
    /// The previous value.
    pub old: Option<Value>,
    /// The new value.
    pub new: Option<Value>,
}

/// Column additions, removals and type changes between the versions.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct SchemaChanges {
    /// Columns in `next` that `previous` does not have, in `next`
    /// order.
    pub added_columns: Vec<String>,
    /// Columns in `previous` that `next` does not have, in `previous`
    /// order.
    pub removed_columns: Vec<String>,
    /// Columns present in both whose `ValueType` changed.
    pub type_changes: Vec<TypeChange>,
}

/// One column whose `ValueType` changed between the versions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct TypeChange {
    /// The column's name.
    pub column: String,
    /// The previous type.
    pub from: ValueType,
    /// The new type.
    pub to: ValueType,
}

/// The diff preview (spec §6.4): counts, schema changes and the first
/// 20 changes per field.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct DiffSummary {
    /// New record count.
    pub added: u64,
    /// Removed record count.
    pub removed: u64,
    /// Changed field count (across all records).
    pub changed: u64,
    /// The schema changes, which must be accepted explicitly.
    pub schema: SchemaChanges,
    /// `true` when nothing changed.
    pub unchanged: bool,
    /// Field name → its first 20 [`FieldChange`]s.
    pub samples: BTreeMap<String, Vec<FieldChange>>,
}

/// Diffs `next` against `previous` by record key (spec §4.4).
/// Key-column problems on either side refuse the diff:
/// `DuplicateKey`, `MissingKey`, `InvalidKeyType` or `UnknownColumn`.
pub fn diff(previous: &Dataset, next: &Imported, key: &RecordKey) -> Result<Diff, ImportError> {
    let prev_ids = record_ids(previous.columns(), key)?;
    let next_ids = record_ids(&next.columns, key)?;

    let prev_cols: HashMap<&str, &epiora_core::dataset::Column> = previous
        .columns()
        .iter()
        .map(|c| (c.name.as_str(), c))
        .collect();
    let next_cols: HashMap<&str, &epiora_core::dataset::Column> =
        next.columns.iter().map(|c| (c.name.as_str(), c)).collect();

    let mut schema = SchemaChanges::default();
    for c in previous.columns() {
        match next_cols.get(c.name.as_str()) {
            None => schema.removed_columns.push(c.name.clone()),
            Some(n) if n.data.value_type() != c.data.value_type() => {
                schema.type_changes.push(TypeChange {
                    column: c.name.clone(),
                    from: c.data.value_type(),
                    to: n.data.value_type(),
                });
            }
            Some(_) => {}
        }
    }
    for c in &next.columns {
        if !prev_cols.contains_key(c.name.as_str()) {
            schema.added_columns.push(c.name.clone());
        }
    }

    let prev_pos: HashMap<&str, usize> = prev_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let next_pos: HashMap<&str, usize> = next_ids
        .iter()
        .enumerate()
        .map(|(j, id)| (id.as_str(), j))
        .collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    for (j, id) in next_ids.iter().enumerate() {
        let Some(&i) = prev_pos.get(id.as_str()) else {
            added.push(j as u32);
            continue;
        };
        for nc in &next.columns {
            let new = cell_value(&nc.data, j);
            match prev_cols.get(nc.name.as_str()) {
                Some(pc) => {
                    let old = cell_value(&pc.data, i);
                    if old != new {
                        changed.push(FieldChange {
                            record_id: id.clone(),
                            row: j as u32,
                            field: nc.name.clone(),
                            old,
                            new,
                        });
                    }
                }
                // A new column's populated value is a field change the
                // store must write; a missing one defaults to null.
                None => {
                    if new.is_some() {
                        changed.push(FieldChange {
                            record_id: id.clone(),
                            row: j as u32,
                            field: nc.name.clone(),
                            old: None,
                            new,
                        });
                    }
                }
            }
        }
    }
    for id in &prev_ids {
        if !next_pos.contains_key(id.as_str()) {
            removed.push(id.clone());
        }
    }

    let unchanged = added.is_empty()
        && removed.is_empty()
        && changed.is_empty()
        && schema.added_columns.is_empty()
        && schema.removed_columns.is_empty()
        && schema.type_changes.is_empty();
    Ok(Diff {
        added,
        removed,
        changed,
        schema,
        unchanged,
    })
}

/// The preview summary of `d` (spec §6.4).
pub fn summarize(d: &Diff) -> DiffSummary {
    let mut samples: BTreeMap<String, Vec<FieldChange>> = BTreeMap::new();
    for fc in &d.changed {
        let entry = samples.entry(fc.field.clone()).or_default();
        if entry.len() < 20 {
            entry.push(fc.clone());
        }
    }
    DiffSummary {
        added: d.added.len() as u64,
        removed: d.removed.len() as u64,
        changed: d.changed.len() as u64,
        schema: d.schema.clone(),
        unchanged: d.unchanged,
        samples,
    }
}

/// Fuzz entry point (plan Task 5): two small arbitrary datasets and a
/// key choice → [`diff`]; must return, never panic, and on `Ok` the
/// apply property holds (diff applied to `previous`'s records equals
/// `next`'s records).
#[cfg(feature = "fuzzing")]
pub fn diff_any(data: &[u8]) {
    let Some((prev_cols, next_cols, key)) = fuzz_decode(data) else {
        return;
    };
    let Ok(previous) = Dataset::new("p", 1, prev_cols) else {
        return;
    };
    let next = Imported {
        row_count: next_cols.first().map(|c| c.data.len() as u64).unwrap_or(0),
        columns: next_cols,
        detection: crate::Detection {
            container: crate::Container::Text,
            encoding: Some(crate::Encoding::Utf8),
            delimiter: Some(','),
            decimal: crate::DecimalSep::Dot,
            header_renames: Vec::new(),
            sheets: Vec::new(),
        },
        settings: crate::ImportSettings::default(),
        coercions: BTreeMap::new(),
        source_sha256: String::new(),
    };
    if let Ok(d) = diff(&previous, &next, &key) {
        check_apply(&previous, &next, &key, &d);
    }
}

/// Decodes two bounded column sets plus a key choice from fuzz bytes.
/// Ids are deduplicated keeping first occurrence so the `Ok` path is
/// exercised; extra columns are renamed deterministically when a name
/// collides.
#[cfg(feature = "fuzzing")]
fn fuzz_decode(
    data: &[u8],
) -> Option<(
    Vec<epiora_core::dataset::Column>,
    Vec<epiora_core::dataset::Column>,
    RecordKey,
)> {
    use arbitrary::Unstructured;
    use epiora_core::dataset::{Column, ColumnData};

    const MAX_ROWS: usize = 24;
    const MAX_COLS: usize = 8;
    const MAX_NAME: usize = 16;

    fn side(u: &mut Unstructured) -> arbitrary::Result<Vec<Column>> {
        let mut ids: Vec<i64> = u.arbitrary()?;
        ids.truncate(MAX_ROWS);
        let mut unique: std::collections::HashSet<i64> = std::collections::HashSet::new();
        ids.retain(|i| unique.insert(*i));
        let rows = ids.len();
        let n_extra = usize::from(u.arbitrary::<u8>()?) % MAX_COLS;
        let mut columns = Vec::with_capacity(n_extra + 1);
        columns.push(Column {
            name: "id".to_string(),
            data: ColumnData::Integer(ids.into_iter().map(Some).collect()),
        });
        for i in 0..n_extra {
            let mut name: String = u.arbitrary()?;
            name = name.chars().take(MAX_NAME).collect();
            // Names must stay unique (`Imported` is not validated like
            // `Dataset`): fall back to `c<i>` and keep suffixing until
            // the name is free.
            if name.is_empty() || columns.iter().any(|c: &Column| c.name == name) {
                let mut k = i;
                loop {
                    let cand = format!("c{k}");
                    if columns.iter().all(|c: &Column| c.name != cand) {
                        name = cand;
                        break;
                    }
                    k += 1;
                }
            }
            let raw: Vec<Option<i64>> = u.arbitrary()?;
            let mut cells: Vec<Option<i64>> = raw;
            cells.truncate(rows);
            cells.resize(rows, None);
            let data = match u.arbitrary::<u8>()? % 5 {
                0 => ColumnData::Integer(cells),
                1 => ColumnData::Real(cells.iter().map(|c| c.map(|v| v as f64)).collect()),
                2 => ColumnData::Text(
                    cells
                        .iter()
                        .map(|c| c.map(|v| format!("t{}", v.rem_euclid(11))))
                        .collect(),
                ),
                3 => ColumnData::Boolean(cells.iter().map(|c| c.map(|v| v % 2 == 0)).collect()),
                _ => ColumnData::Date(
                    cells
                        .iter()
                        .map(|c| {
                            c.map(|v| i32::try_from(v.rem_euclid(200_000)).unwrap_or(0) - 100_000)
                        })
                        .collect(),
                ),
            };
            columns.push(Column { name, data });
        }
        Ok(columns)
    }

    let mut u = Unstructured::new(data);
    let prev = side(&mut u).ok()?;
    let next = side(&mut u).ok()?;
    let key = match u.arbitrary::<u8>().ok()? % 8 {
        0..=3 => RecordKey::RowPosition,
        7 => RecordKey::Column {
            name: format!("k{}", u.arbitrary::<u8>().ok()? % 4),
        },
        _ => RecordKey::Column {
            name: "id".to_string(),
        },
    };
    Some((prev, next, key))
}

/// The apply-property checker (plan Task 5 Step 4): rebuild `next`'s
/// records from `previous`'s via the diff and compare. A mismatch
/// under cargo-fuzz is the crash signal — the crate's `clippy::panic`
/// deny is lifted for exactly this assertion.
#[cfg(feature = "fuzzing")]
#[allow(clippy::panic)]
fn check_apply(previous: &Dataset, next: &Imported, key: &RecordKey, d: &Diff) {
    use crate::records::record_json;

    let prev_ids = record_ids(previous.columns(), key).unwrap_or_default();
    let next_ids = record_ids(&next.columns, key).unwrap_or_default();
    let prev_pos: HashMap<&str, usize> = prev_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let mut by_id: HashMap<&str, Vec<&FieldChange>> = HashMap::new();
    for fc in &d.changed {
        by_id.entry(fc.record_id.as_str()).or_default().push(fc);
    }
    for (j, id) in next_ids.iter().enumerate() {
        let expect = record_json(&next.columns, j);
        let got = if d.added.contains(&(j as u32)) {
            expect.clone()
        } else {
            let i = prev_pos[id.as_str()];
            let raw = record_json(previous.columns(), i);
            let serde_json::Value::Object(mut map) =
                serde_json::from_str::<serde_json::Value>(&raw).unwrap_or(serde_json::Value::Null)
            else {
                panic!("record {id}: previous record is not an object");
            };
            for name in &d.schema.removed_columns {
                map.remove(name);
            }
            if let Some(changes) = by_id.get(id.as_str()) {
                for fc in changes {
                    let cell = match &fc.new {
                        Some(v) => serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
                        None => serde_json::Value::Null,
                    };
                    map.insert(fc.field.clone(), cell);
                }
            }
            for c in &next.columns {
                map.entry(c.name.clone()).or_insert(serde_json::Value::Null);
            }
            serde_json::to_string(&map).unwrap_or_else(|_| "{}".to_string())
        };
        assert!(
            got == expect,
            "apply property violated for record {id}: {got} != {expect}"
        );
    }
}
