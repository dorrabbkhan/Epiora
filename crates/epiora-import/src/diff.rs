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

#[cfg(feature = "fuzzing")]
mod fuzz;
#[cfg(feature = "fuzzing")]
pub use fuzz::diff_any;

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
