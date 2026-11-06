//! Record keys (v0.3 spec §4.4): a key column of type Integer (decimal
//! string), Text (verbatim) or Date (ISO `YYYY-MM-DD`) provides record
//! ids; `RecordKey::RowPosition` gives `row:<1-based>`. Duplicate or
//! missing ids are refused — never a silent merge — and
//! [`key_candidates`] reports per-column usability for the preview.

use crate::error::ImportError;
use crate::excel_date::days_to_iso;
use crate::settings::RecordKey;
use epiora_core::dataset::{Column, ColumnData};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::HashSet;

/// One column's suitability as a record key, for the import preview.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct KeyCandidate {
    /// The column's name.
    pub column: String,
    /// Integer/Text/Date type with no missing or duplicated values.
    pub usable: bool,
    /// Non-missing cells whose rendered id already appeared earlier.
    pub duplicates: u64,
    /// Missing cells.
    pub missing: u64,
}

/// Renders one cell's candidate record id; `None` when the cell is
/// missing. `Real`/`Boolean` are not keyable but still render (the
/// preview counts their duplicates too).
fn cell_id(data: &ColumnData, row: usize) -> Option<String> {
    match data {
        ColumnData::Integer(v) => v.get(row).copied().flatten().map(|i| i.to_string()),
        ColumnData::Real(v) => v
            .get(row)
            .copied()
            .flatten()
            .map(|x| serde_json::to_string(&x).unwrap_or_else(|_| "0".to_string())),
        ColumnData::Text(v) => v.get(row).cloned().flatten(),
        ColumnData::Boolean(v) => v
            .get(row)
            .copied()
            .flatten()
            .map(|b| if b { "true" } else { "false" }.to_string()),
        ColumnData::Date(v) => v.get(row).copied().flatten().map(days_to_iso),
    }
}

/// Whether the column's type can provide record ids (spec §4.4).
fn keyable(data: &ColumnData) -> bool {
    matches!(
        data,
        ColumnData::Integer(_) | ColumnData::Text(_) | ColumnData::Date(_)
    )
}

/// The number of data rows `columns` describe (`0` for no columns).
fn row_count(columns: &[Column]) -> usize {
    columns.first().map(|c| c.data.len()).unwrap_or(0)
}

/// The record id of every row (spec §4.4). A key column of type `Real`
/// or `Boolean` is [`ImportError::InvalidKeyType`]; a missing cell is
/// [`ImportError::MissingKey`] and a repeated id is
/// [`ImportError::DuplicateKey`] — both naming 1-based data lines.
pub fn record_ids(columns: &[Column], key: &RecordKey) -> Result<Vec<String>, ImportError> {
    let rows = row_count(columns);
    let column = match key {
        RecordKey::RowPosition => {
            return Ok((1..=rows).map(|r| format!("row:{r}")).collect());
        }
        RecordKey::Column { name } => columns
            .iter()
            .find(|c| c.name == *name)
            .ok_or_else(|| ImportError::UnknownColumn { name: name.clone() })?,
    };
    if !keyable(&column.data) {
        return Err(ImportError::InvalidKeyType {
            column: column.name.clone(),
            found: column.data.value_type(),
        });
    }
    let mut seen: HashMap<String, u64> = HashMap::with_capacity(rows);
    let mut ids = Vec::with_capacity(rows);
    for r in 0..rows {
        let line = r as u64 + 1;
        let id = cell_id(&column.data, r).ok_or(ImportError::MissingKey { line })?;
        if let Some(&first_line) = seen.get(&id) {
            return Err(ImportError::DuplicateKey {
                key: id,
                first_line,
                line,
            });
        }
        seen.insert(id.clone(), line);
        ids.push(id);
    }
    Ok(ids)
}

/// Every column's suitability as a record key, in column order (spec
/// §4.4: duplicates and missing values are shown and block the choice).
pub fn key_candidates(columns: &[Column]) -> Vec<KeyCandidate> {
    let rows = row_count(columns);
    columns
        .iter()
        .map(|c| {
            let mut missing = 0u64;
            let mut duplicates = 0u64;
            let mut seen: HashSet<String> = HashSet::with_capacity(rows);
            for r in 0..rows {
                match cell_id(&c.data, r) {
                    None => missing += 1,
                    Some(id) => {
                        if !seen.insert(id) {
                            duplicates += 1;
                        }
                    }
                }
            }
            KeyCandidate {
                column: c.name.clone(),
                usable: keyable(&c.data) && missing == 0 && duplicates == 0,
                duplicates,
                missing,
            }
        })
        .collect()
}
