//! The `LegacySource` trait and the in-memory `BatchSource` that
//! consumes EPTB batches from the worker (Task 5). `epiora-mdb` and
//! the sqlite-wasm worker encode batches; this crate decodes them via
//! [`crate::batch::decode_batch`] and merges same-table chunks.

use std::collections::HashMap;

use crate::batch::{decode_batch, RawData, RawTable};
use crate::error::LegacyError;

/// A stream of raw Access/SQLite tables the mapper reads through. The
/// worker implements this over `epiora-mdb` or the sqlite-wasm bridge
/// (spec §4.4); tests implement it over those same backends.
pub trait LegacySource {
    /// Every table the file carries, in file order.
    fn tables(&self) -> Vec<String>;
    /// Read one table fully. `name` comes from [`Self::tables`];
    /// implementations return a typed [`LegacyError`] — `mdb_*` codes
    /// pass through as [`LegacyError::Source`].
    fn table(&mut self, name: &str) -> Result<RawTable, LegacyError>;
}

/// One decoded cell value. Access date-times arrive as
/// [`Cell::Serial`] (the raw OLE Automation serial); SQLite projects
/// deliver date-times as `YYYY-MM-DD HH:MM:SS` [`Cell::Text`].
#[derive(Clone, Debug, PartialEq)]
pub enum Cell {
    /// An integer value.
    Int(i64),
    /// A floating-point value.
    Real(f64),
    /// A UTF-8 text value.
    Text(String),
    /// A Boolean value.
    Bool(bool),
    /// A raw Access datetime serial (days since 1899-12-30 + fraction).
    Serial(f64),
}

impl Cell {
    /// JS `Number(v)`: Int/Real → the number; Bool → 1/0; Text → a
    /// float parse of the whole string (NaN when it fails); Serial →
    /// `f64::NAN` (the golden route never sees serials as text).
    pub(crate) fn as_number(&self) -> f64 {
        match self {
            Self::Int(i) => *i as f64,
            Self::Real(f) => *f,
            Self::Text(s) => s.trim().parse::<f64>().unwrap_or(f64::NAN),
            Self::Bool(b) => i32::from(*b) as f64,
            Self::Serial(_) => f64::NAN,
        }
    }

    /// JS `String(v)`: numbers use `format!("{}")` which matches JS for
    /// integral floats (`33.0` → `"33"`).
    pub(crate) fn as_string(&self) -> String {
        match self {
            Self::Int(i) => i.to_string(),
            Self::Real(f) => format!("{f}"),
            Self::Text(s) => s.clone(),
            Self::Bool(b) => b.to_string(),
            Self::Serial(f) => crate::ei7::datetime::serial_to_text(*f),
        }
    }
}

/// One decoded column as row-major access sees it.
pub(crate) struct TableCol {
    /// The column name.
    pub name: String,
    /// True when the column carried binary data the encoder skipped
    /// (kind 5): it exists for the unclaimed-column report but has no
    /// row values. Read by the EI3 mapper (Task 8).
    #[allow(dead_code)]
    pub binary: bool,
}

/// A decoded table transposed to row-major `rows × columns`.
pub(crate) struct TableData {
    /// The table name.
    pub name: String,
    /// Every column, in file order (binary columns included).
    pub cols: Vec<TableCol>,
    /// Row-major cells; `rows[r][c]` aligns with `cols[c]`.
    pub rows: Vec<Vec<Option<Cell>>>,
    /// The code page the encoder recorded (JET3 files only).
    pub code_page: Option<String>,
    /// Replacement-character substitutions the decoder made.
    pub decode_errors: u32,
}

impl TableData {
    /// An empty table shell (the golden's `|| []` for missing tables).
    pub fn empty(name: &str) -> Self {
        Self {
            name: name.to_string(),
            cols: Vec::new(),
            rows: Vec::new(),
            code_page: None,
            decode_errors: 0,
        }
    }

    /// Transpose a [`RawTable`]; binary columns keep their name but no
    /// values.
    pub fn from_raw(raw: &RawTable) -> Self {
        let n_rows = raw
            .columns
            .iter()
            .map(|c| match &c.data {
                RawData::Int(v) => v.len(),
                RawData::Float(v) => v.len(),
                RawData::Text(v) => v.len(),
                RawData::Bool(v) => v.len(),
                RawData::DateTime(v) => v.len(),
                RawData::BinarySkipped => 0,
            })
            .max()
            .unwrap_or(0);
        let mut rows: Vec<Vec<Option<Cell>>> = (0..n_rows)
            .map(|_| Vec::with_capacity(raw.columns.len()))
            .collect();
        for c in &raw.columns {
            for (ri, row) in rows.iter_mut().enumerate() {
                row.push(match &c.data {
                    RawData::Int(v) => v.get(ri).copied().flatten().map(Cell::Int),
                    RawData::Float(v) => v.get(ri).copied().flatten().map(Cell::Real),
                    RawData::Text(v) => v.get(ri).cloned().flatten().map(Cell::Text),
                    RawData::Bool(v) => v.get(ri).copied().flatten().map(Cell::Bool),
                    RawData::DateTime(v) => v.get(ri).copied().flatten().map(Cell::Serial),
                    RawData::BinarySkipped => None,
                });
            }
        }
        Self {
            name: raw.name.clone(),
            cols: raw
                .columns
                .iter()
                .map(|c| TableCol {
                    name: c.name.clone(),
                    binary: matches!(c.data, RawData::BinarySkipped),
                })
                .collect(),
            rows,
            code_page: raw.code_page.clone(),
            decode_errors: raw.decode_errors,
        }
    }

    /// Case-insensitive column index (JS `colOf`).
    pub fn col_ci(&self, name: &str) -> Option<usize> {
        self.cols
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(name))
    }

    /// Cell `(row, column-name)` or `None`.
    pub fn get(&self, row: usize, col: usize) -> Option<&Cell> {
        self.rows
            .get(row)
            .and_then(|r| r.get(col))
            .and_then(|c| c.as_ref())
    }

    /// Whole column as `Option<Cell>` values.
    pub fn column(&self, col: usize) -> Vec<Option<Cell>> {
        self.rows
            .iter()
            .map(|r| r.get(col).cloned().flatten())
            .collect()
    }
}

/// In-memory `LegacySource` fed by the worker: `push(table, batch)`
/// decodes one EPTB chunk; chunks of one table merge row-wise.
#[derive(Clone, Default)]
pub struct BatchSource {
    order: Vec<String>,
    tables: HashMap<String, RawTable>,
}

impl BatchSource {
    /// An empty source.
    pub fn new() -> Self {
        Self::default()
    }

    /// Decode one EPTB batch for `table` and merge it into the source.
    /// The batch's own name tag must match `table`; same-table chunks
    /// must agree on column count, names, kinds and source types.
    pub fn push(&mut self, table: &str, batch: &[u8]) -> Result<(), LegacyError> {
        let decoded = decode_batch(batch)?;
        if decoded.name != table {
            return Err(LegacyError::BatchInvalid {
                reason: format!("batch for table {:?} pushed as {table:?}", decoded.name),
            });
        }
        let key = table.to_lowercase();
        match self.tables.get_mut(&key) {
            None => {
                self.order.push(decoded.name.clone());
                self.tables.insert(key, decoded);
            }
            Some(acc) => merge_batch(acc, &decoded)?,
        }
        Ok(())
    }
}

fn merge_batch(acc: &mut RawTable, next: &RawTable) -> Result<(), LegacyError> {
    if acc.columns.len() != next.columns.len() {
        return Err(LegacyError::BatchInvalid {
            reason: format!(
                "table {:?}: chunk column count {} != {}",
                acc.name,
                next.columns.len(),
                acc.columns.len()
            ),
        });
    }
    for (a, n) in acc.columns.iter_mut().zip(next.columns.iter()) {
        if a.name != n.name || a.source_type != n.source_type {
            return Err(LegacyError::BatchInvalid {
                reason: format!("table {:?}: chunk column mismatch", acc.name),
            });
        }
        match (&mut a.data, &n.data) {
            (RawData::Int(x), RawData::Int(y)) => x.extend(y.iter().copied()),
            (RawData::Float(x), RawData::Float(y)) => x.extend(y.iter().copied()),
            (RawData::Text(x), RawData::Text(y)) => x.extend(y.iter().cloned()),
            (RawData::Bool(x), RawData::Bool(y)) => x.extend(y.iter().copied()),
            (RawData::DateTime(x), RawData::DateTime(y)) => x.extend(y.iter().copied()),
            (RawData::BinarySkipped, RawData::BinarySkipped) => {}
            _ => {
                return Err(LegacyError::BatchInvalid {
                    reason: format!("table {:?}: chunk kind mismatch", acc.name),
                })
            }
        }
    }
    if acc.code_page.is_none() {
        acc.code_page = next.code_page.clone();
    }
    acc.decode_errors += next.decode_errors;
    Ok(())
}

impl LegacySource for BatchSource {
    fn tables(&self) -> Vec<String> {
        self.order.clone()
    }

    fn table(&mut self, name: &str) -> Result<RawTable, LegacyError> {
        self.tables
            .get(&name.to_lowercase())
            .cloned()
            .ok_or_else(|| LegacyError::Source {
                code: "legacy_no_table",
                reason: format!("no table {name:?} in source"),
            })
    }
}
