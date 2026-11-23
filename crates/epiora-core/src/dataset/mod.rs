//! The in-memory dataset model (spec §3.3): typed columns, `None` for
//! missing (Epi Info's "(.)"), validation in `Dataset::new`, canonical
//! bytes and the SHA-256 content hash (`hash`).

mod hash;
mod labels;
mod meta;

use crate::EngineError;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::collections::BTreeMap;

pub use labels::LabelSet;
pub use meta::{ColumnMeta, DatasetLink, LinkKind};

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
    /// Seconds since midnight, `0..=86_399` (v1.1 spec §4.3).
    Time(Vec<Option<i32>>),
    /// Seconds since 1970-01-01, no time zone, as Epi Info stores it
    /// (v1.1 spec §4.3).
    DateTime(Vec<Option<i64>>),
}

/// The value type of a column; also the hash tag (spec §3.3).
/// Serialized as the corpus spellings `integer|real|text|boolean|date`
/// plus `time|date_time` (v0.3 import settings `column_types`, spec
/// §6.2; v1.1 spec §4.3).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
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
    /// Tag 6.
    Time,
    /// Tag 7.
    DateTime,
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
    /// Seconds since midnight.
    Time(i32),
    /// Seconds since 1970-01-01, no time zone.
    DateTime(i64),
}

impl Value {
    /// The canonical bytes of the `LBLS`/`snapshot v2` encoding: the
    /// type tag (u8) then the same value bytes a present cell writes.
    pub(crate) fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        hash::write_value(&mut out, self);
        out
    }
}

/// A named, typed column.
#[derive(Clone, Debug, PartialEq)]
pub struct Column {
    /// Column name: non-empty and unique within the dataset.
    pub name: String,
    /// The column's values.
    pub data: ColumnData,
}

/// A validated in-memory dataset (spec §3.3). `meta` is empty or one
/// entry per column (set by `with_meta`); `label_sets` holds the
/// value-label sets those metas reference (v1.1 spec §4.3).
#[derive(Clone, Debug)]
pub struct Dataset {
    id: String,
    version: u64,
    columns: Vec<Column>,
    meta: Vec<ColumnMeta>,
    label_sets: BTreeMap<String, LabelSet>,
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
            Self::Time(v) => v.len(),
            Self::DateTime(v) => v.len(),
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
            Self::Time(_) => ValueType::Time,
            Self::DateTime(_) => ValueType::DateTime,
        }
    }
}

#[cfg(feature = "fuzzing")]
impl<'a> arbitrary::Arbitrary<'a> for ValueType {
    fn arbitrary(u: &mut arbitrary::Unstructured<'a>) -> arbitrary::Result<Self> {
        u.choose(&[
            Self::Integer,
            Self::Real,
            Self::Text,
            Self::Boolean,
            Self::Date,
            Self::Time,
            Self::DateTime,
        ])
        .copied()
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
            Self::Time => 6,
            Self::DateTime => 7,
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
            if let ColumnData::Time(values) = &column.data {
                for v in values.iter().flatten() {
                    if !(0..=86_399).contains(v) {
                        return Err(EngineError::InvalidTime { value: *v });
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
            meta: Vec::new(),
            label_sets: BTreeMap::new(),
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

    /// Attaches per-column metadata and value-label sets (v1.1 spec
    /// §4.3). `meta` must hold exactly one entry per column; every
    /// `label_set` id it names must be a key of `label_sets`
    /// (`LabelSetMissing`), and every `Time` value inside a label set
    /// must be in range (`InvalidTime`). Prompts, legacy types and
    /// legacy names never enter the hash; label sets do (`LBLS`).
    pub fn with_meta(
        mut self,
        meta: Vec<ColumnMeta>,
        label_sets: BTreeMap<String, LabelSet>,
    ) -> Result<Dataset, EngineError> {
        if meta.len() != self.columns.len() {
            return Err(EngineError::InvalidOptions {
                reason: format!(
                    "meta has {} entries for {} columns",
                    meta.len(),
                    self.columns.len()
                ),
            });
        }
        for m in &meta {
            if let Some(id) = &m.label_set {
                if !label_sets.contains_key(id) {
                    return Err(EngineError::LabelSetMissing { id: id.clone() });
                }
            }
        }
        for set in label_sets.values() {
            for (value, _) in &set.entries {
                match value {
                    Value::Time(t) if !(0..=86_399).contains(t) => {
                        return Err(EngineError::InvalidTime { value: *t });
                    }
                    Value::Real(x) if !x.is_finite() => {
                        return Err(EngineError::InvalidOptions {
                            reason: format!("non-finite real in label set {:?}", set.id),
                        });
                    }
                    _ => {}
                }
            }
        }
        self.meta = meta;
        self.label_sets = label_sets;
        Ok(self)
    }

    /// The per-column metadata; `[]` when the dataset has none.
    pub fn meta(&self) -> &[ColumnMeta] {
        &self.meta
    }

    /// The dataset's value-label sets, keyed by id.
    pub fn label_sets(&self) -> &BTreeMap<String, LabelSet> {
        &self.label_sets
    }

    /// The label `column`'s label set gives value `v`, or `None` when
    /// the column has no label set or no entry matches (first entry
    /// wins on a duplicated value).
    pub fn label_for(&self, column: usize, v: &Value) -> Option<&str> {
        let id = self.meta.get(column)?.label_set.as_deref()?;
        self.label_sets.get(id)?.label_for(v)
    }

    /// The label set a named column's meta references (tabulate's
    /// category-label lookup), or `None`.
    pub(crate) fn label_set_for(&self, column_name: &str) -> Option<&LabelSet> {
        let index = self.columns.iter().position(|c| c.name == column_name)?;
        let id = self.meta.get(index)?.label_set.as_deref()?;
        self.label_sets.get(id)
    }

    /// Whether the dataset carries anything the v1 encodings cannot
    /// represent: a `Time`/`DateTime` column, any column meta or any
    /// label set. Drives snapshot v2 selection.
    pub(crate) fn has_v2_features(&self) -> bool {
        !self.meta.is_empty()
            || !self.label_sets.is_empty()
            || self
                .columns
                .iter()
                .any(|c| matches!(c.data, ColumnData::Time(_) | ColumnData::DateTime(_)))
    }

    /// Whether the `LBLS` section follows the v1 canonical bytes: a
    /// `Time`/`DateTime` column or any stored label set (the plan's
    /// "any `label_set` is set"). Prompts and legacy names alone do
    /// not trigger it.
    pub(crate) fn hash_needs_lbls(&self) -> bool {
        !self.label_sets.is_empty()
            || self
                .columns
                .iter()
                .any(|c| matches!(c.data, ColumnData::Time(_) | ColumnData::DateTime(_)))
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
