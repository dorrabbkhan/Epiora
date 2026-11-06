//! The single error type of the importer (v0.3 spec §6, §11). No input
//! can cause a panic; every failure is an `ImportError` whose `code()`
//! is stable — callers match on `code()`, never on the message.

use crate::settings::Encoding;
use epiora_core::dataset::ValueType;
use epiora_core::EngineError;

/// An import failure with a stable machine-readable `code()`.
#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    /// The file is empty (0 bytes or nothing after BOM removal).
    #[error("empty file")]
    EmptyFile,
    /// The file has content but no columns could be read.
    #[error("no columns")]
    NoColumns,
    /// An explicit `settings.encoding` does not decode the input.
    #[error("invalid {encoding} input")]
    Encoding {
        /// The encoding that failed.
        encoding: Encoding,
    },
    /// A record's field count differs from the first record's.
    #[error("line {line}: expected {expected} fields, found {found}: {sample}")]
    RaggedRow {
        /// 1-based physical line where the ragged record starts.
        line: u64,
        /// The first record's field count.
        expected: usize,
        /// This record's field count.
        found: usize,
        /// The offending line's text (truncated).
        sample: String,
    },
    /// More than `MAX_COLUMNS` fields.
    #[error("too many columns")]
    TooManyColumns,
    /// More than `MAX_ROWS` rows.
    #[error("too many rows")]
    TooManyRows,
    /// `column_types`, `sheet` or `key` names a column that is not present.
    #[error("unknown column: {name}")]
    UnknownColumn {
        /// The offending name.
        name: String,
    },
    /// `settings.sheet` names a sheet the workbook does not have.
    #[error("sheet not found: {name}")]
    SheetNotFound {
        /// The missing sheet.
        name: String,
    },
    /// A spreadsheet container could not be read.
    #[error("spreadsheet: {reason}")]
    Spreadsheet {
        /// What went wrong.
        reason: String,
    },
    /// The settings are inconsistent (e.g. a non-ASCII delimiter).
    #[error("invalid settings: {reason}")]
    InvalidSettings {
        /// What is wrong.
        reason: String,
    },
    /// A record id appears twice in a re-import diff (spec §4.4).
    #[error("duplicate key {key}: lines {first_line} and {line}")]
    DuplicateKey {
        /// The duplicated record id.
        key: String,
        /// 1-based data line of the first occurrence.
        first_line: u64,
        /// 1-based data line of the second occurrence.
        line: u64,
    },
    /// A record id is missing in a re-import diff.
    #[error("missing key at line {line}")]
    MissingKey {
        /// 1-based data line of the missing id.
        line: u64,
    },
    /// A key column's type cannot be a record id.
    #[error("invalid key type for {column}: {found:?}")]
    InvalidKeyType {
        /// The key column.
        column: String,
        /// Its type.
        found: ValueType,
    },
    /// An engine error surfaced by dataset construction.
    #[error(transparent)]
    Engine(#[from] EngineError),
}

impl ImportError {
    /// The stable machine-readable code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Self::EmptyFile => "empty_file",
            Self::NoColumns => "no_columns",
            Self::Encoding { .. } => "encoding_invalid",
            Self::RaggedRow { .. } => "ragged_row",
            Self::TooManyColumns => "too_many_columns",
            Self::TooManyRows => "too_many_rows",
            Self::UnknownColumn { .. } => "unknown_column",
            Self::SheetNotFound { .. } => "sheet_not_found",
            Self::Spreadsheet { .. } => "spreadsheet",
            Self::InvalidSettings { .. } => "invalid_settings",
            Self::DuplicateKey { .. } => "duplicate_key",
            Self::MissingKey { .. } => "missing_key",
            Self::InvalidKeyType { .. } => "invalid_key_type",
            Self::Engine(e) => e.code(),
        }
    }
}
