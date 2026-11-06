//! Canonical record JSON (v0.3 spec §4.1 `records.values`): each row is
//! `{"<col>":{"type":"integer","value":20},"<col2>":null}` with keys
//! sorted by UTF-8 bytes (a `serde_json::Map` orders them) and values
//! in the v0.2 `Value` encoding. [`dataset_from_records`] rebuilds a
//! `Dataset` from a schema plus those rows — the snapshot-rebuild path
//! of spec §4.3.

use crate::error::ImportError;
use epiora_core::dataset::{Column, ColumnData, Dataset, Value, ValueType};

/// The cell at `row` as a `Value`; `-0.0` is normalised to `0.0` so the
/// canonical bytes of a rebuilt dataset and of stored JSON agree.
pub(crate) fn cell_value(data: &ColumnData, row: usize) -> Option<Value> {
    match data {
        ColumnData::Integer(v) => v.get(row).copied().flatten().map(Value::Integer),
        ColumnData::Real(v) => v
            .get(row)
            .copied()
            .flatten()
            .map(|x| Value::Real(if x == 0.0 { 0.0 } else { x })),
        ColumnData::Text(v) => v.get(row).cloned().flatten().map(Value::Text),
        ColumnData::Boolean(v) => v.get(row).copied().flatten().map(Value::Boolean),
        ColumnData::Date(v) => v.get(row).copied().flatten().map(Value::Date),
    }
}

/// The canonical JSON of `row`: every column name mapped to its
/// `Value` encoding or `null`, keys sorted by UTF-8 bytes. A row index
/// past the end serialises as `{}`.
pub fn record_json(columns: &[Column], row: usize) -> String {
    let mut map = serde_json::Map::with_capacity(columns.len());
    for c in columns {
        let cell = match cell_value(&c.data, row) {
            Some(v) => serde_json::to_value(&v).unwrap_or(serde_json::Value::Null),
            None => serde_json::Value::Null,
        };
        map.insert(c.name.clone(), cell);
    }
    // Serialising a map of values is infallible.
    serde_json::to_string(&serde_json::Value::Object(map)).unwrap_or_else(|_| "{}".to_string())
}

/// A record row that is not a JSON object holding `Value`-shaped cells.
fn bad_record(row: usize, reason: impl std::fmt::Display) -> ImportError {
    ImportError::InvalidSettings {
        reason: format!("record {}: {reason}", row + 1),
    }
}

/// Column `name` across `parsed` rows, extracted with `extract`;
/// absent or `null` fields are missing and a `Value` of the wrong
/// variant is an error.
fn read_cells<T>(
    parsed: &[serde_json::Map<String, serde_json::Value>],
    name: &str,
    ty: ValueType,
    extract: impl Fn(Value) -> Option<T>,
) -> Result<Vec<Option<T>>, ImportError> {
    parsed
        .iter()
        .enumerate()
        .map(|(r, map)| match map.get(name) {
            None | Some(serde_json::Value::Null) => Ok(None),
            Some(raw) => {
                let value: Value = serde_json::from_value(raw.clone())
                    .map_err(|e| bad_record(r, format!("field {name:?}: {e}")))?;
                extract(value.clone()).map(Some).ok_or_else(|| {
                    bad_record(
                        r,
                        format!("field {name:?} holds {value:?}, expected {ty:?}"),
                    )
                })
            }
        })
        .collect()
}

/// Rebuilds a dataset from its `records` rows (spec §4.3). `schema`
/// gives the column names and types; each string in `rows` is one
/// canonical record JSON object. A field absent from a row is missing;
/// a value whose variant does not match the schema type is an error.
pub fn dataset_from_records(
    id: &str,
    version: u64,
    schema: &[(String, ValueType)],
    rows: &[&str],
) -> Result<Dataset, ImportError> {
    let mut parsed: Vec<serde_json::Map<String, serde_json::Value>> =
        Vec::with_capacity(rows.len());
    for (r, raw) in rows.iter().enumerate() {
        match serde_json::from_str::<serde_json::Value>(raw) {
            Ok(serde_json::Value::Object(map)) => parsed.push(map),
            Ok(_) => return Err(bad_record(r, "record is not a JSON object")),
            Err(e) => return Err(bad_record(r, e)),
        }
    }
    let mut columns = Vec::with_capacity(schema.len());
    for (name, ty) in schema {
        let data = match ty {
            ValueType::Integer => {
                ColumnData::Integer(read_cells(&parsed, name, *ty, |v| match v {
                    Value::Integer(i) => Some(i),
                    _ => None,
                })?)
            }
            ValueType::Real => ColumnData::Real(read_cells(&parsed, name, *ty, |v| match v {
                Value::Real(x) => Some(x),
                _ => None,
            })?),
            ValueType::Text => ColumnData::Text(read_cells(&parsed, name, *ty, |v| match v {
                Value::Text(s) => Some(s),
                _ => None,
            })?),
            ValueType::Boolean => {
                ColumnData::Boolean(read_cells(&parsed, name, *ty, |v| match v {
                    Value::Boolean(b) => Some(b),
                    _ => None,
                })?)
            }
            ValueType::Date => ColumnData::Date(read_cells(&parsed, name, *ty, |v| match v {
                Value::Date(d) => Some(d),
                _ => None,
            })?),
        };
        columns.push(Column {
            name: name.clone(),
            data,
        });
    }
    Ok(Dataset::new(id, version, columns)?)
}
