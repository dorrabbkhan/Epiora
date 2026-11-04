//! The in-memory dataset model (spec §3.3): typed columns, `None` for
//! missing (Epi Info's "(.)"), validation in `Dataset::new`, canonical
//! bytes and the SHA-256 content hash (`hash`).

mod hash;

use crate::EngineError;
use serde::{Deserialize, Serialize};
use sha2::Digest;

/// Typed column storage. `None` is a missing value.
#[derive(Clone, Debug, PartialEq)]
pub enum ColumnData {
    /// Whole numbers.
    Integer(Vec<Option<i64>>),
    /// Floating point; non-finite values are rejected by `Dataset::new`.
    Real(Vec<Option<f64>>),
    /// UTF-8 text.
    Text(Vec<Option<String>>),
    /// Yes/No.
    Boolean(Vec<Option<bool>>),
    /// Days since 1970-01-01.
    Date(Vec<Option<i32>>),
}

/// The value type of a column; also the hash tag (spec §3.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValueType {
    /// Tag 1.
    Integer,
    /// Tag 2.
    Real,
    /// Tag 3.
    Text,
    /// Tag 4.
    Boolean,
    /// Tag 5.
    Date,
}

/// One cell value; adjacently tagged for JSON
/// (`{"type": "integer", "value": 5}`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub enum Value {
    /// Whole number.
    Integer(i64),
    /// Floating point.
    Real(f64),
    /// UTF-8 text.
    Text(String),
    /// Yes/No.
    Boolean(bool),
    /// Days since 1970-01-01.
    Date(i32),
}

/// A named, typed column.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    /// Column name: non-empty and unique within the dataset.
    pub name: String,
    /// The column's values.
    pub data: ColumnData,
}

/// A validated in-memory dataset (spec §3.3).
#[derive(Clone, Debug)]
pub struct Dataset {
    id: String,
    version: u64,
    columns: Vec<Column>,
}

impl ColumnData {
    /// The number of rows in this column.
    pub fn len(&self) -> usize {
        match self {
            Self::Integer(v) => v.len(),
            Self::Real(v) => v.len(),
            Self::Text(v) => v.len(),
            Self::Boolean(v) => v.len(),
            Self::Date(v) => v.len(),
        }
    }

    /// Whether this column has no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The column's value type (hash tag source).
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Integer(_) => ValueType::Integer,
            Self::Real(_) => ValueType::Real,
            Self::Text(_) => ValueType::Text,
            Self::Boolean(_) => ValueType::Boolean,
            Self::Date(_) => ValueType::Date,
        }
    }
}

impl ValueType {
    /// The canonical-bytes tag (spec §3.3).
    pub(crate) fn tag(self) -> u8 {
        match self {
            Self::Integer => 1,
            Self::Real => 2,
            Self::Text => 3,
            Self::Boolean => 4,
            Self::Date => 5,
        }
    }
}

impl Dataset {
    /// Builds a dataset, rejecting (spec §3.3): no columns
    /// (`EmptyDataset`), unequal column lengths, duplicate or empty
    /// column names, and non-finite reals (`InvalidColumn`). Zero rows
    /// is valid.
    pub fn new(
        id: impl Into<String>,
        version: u64,
        columns: Vec<Column>,
    ) -> Result<Dataset, EngineError> {
        if columns.is_empty() {
            return Err(EngineError::EmptyDataset);
        }
        let rows = columns[0].data.len();
        for column in &columns {
            if column.name.is_empty() {
                return Err(invalid(&column.name, "column name is empty"));
            }
            if column.data.len() != rows {
                return Err(invalid(
                    &column.name,
                    format!("column has {} rows, expected {rows}", column.data.len()),
                ));
            }
            if let ColumnData::Real(values) = &column.data {
                for (i, v) in values.iter().enumerate() {
                    if let Some(x) = v {
                        if !x.is_finite() {
                            return Err(invalid(
                                &column.name,
                                format!("non-finite real at row {i}"),
                            ));
                        }
                    }
                }
            }
        }
        for (i, column) in columns.iter().enumerate() {
            if columns[..i].iter().any(|c| c.name == column.name) {
                return Err(invalid(&column.name, "duplicate column name"));
            }
        }
        Ok(Dataset {
            id: id.into(),
            version,
            columns,
        })
    }

    /// The dataset's id.
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The dataset's version.
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Row count (every column has the same length after `new`).
    pub fn row_count(&self) -> usize {
        self.columns[0].data.len()
    }

    /// The columns, in order.
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// The column named `name`, or `UnknownVariable`.
    pub fn column(&self, name: &str) -> Result<&Column, EngineError> {
        self.columns
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| EngineError::UnknownVariable {
                name: name.to_string(),
            })
    }

    /// The canonical byte encoding of spec §3.3; `id` and `version` are
    /// not included.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        hash::canonical_bytes(self)
    }

    /// Lowercase hex SHA-256 of `canonical_bytes` (spec §3.3).
    pub fn content_hash_hex(&self) -> String {
        let digest = sha2::Sha256::digest(self.canonical_bytes());
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            hex.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
            hex.push(char::from_digit((byte & 0xf) as u32, 16).unwrap_or('0'));
        }
        hex
    }
}

fn invalid(column: &str, reason: impl Into<String>) -> EngineError {
    EngineError::InvalidColumn {
        column: column.to_string(),
        reason: reason.into(),
    }
}
