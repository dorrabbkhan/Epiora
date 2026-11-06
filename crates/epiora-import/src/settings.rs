//! Import settings and detection types (v0.3 spec §6.1). All types
//! serialize `snake_case` with the spellings of
//! `fixtures/schema/import.schema.json`.

use epiora_core::dataset::ValueType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The physical container of an input file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum Container {
    /// Delimited text (csv/tsv/…).
    Text,
    /// OOXML workbook (zip magic + `[Content_Types].xml`).
    Xlsx,
    /// Legacy OLE workbook (`D0 CF 11 E0 A1 B1 1A E1`).
    Xls,
    /// OpenDocument spreadsheet.
    Ods,
}

/// The character encoding of a text file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum Encoding {
    /// UTF-8 (with or without BOM).
    Utf8,
    /// UTF-16 little-endian (FF FE BOM).
    Utf16le,
    /// UTF-16 big-endian (FE FF BOM).
    Utf16be,
    /// Windows-1252.
    Windows1252,
}

impl std::fmt::Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Utf8 => "utf8",
            Self::Utf16le => "utf16le",
            Self::Utf16be => "utf16be",
            Self::Windows1252 => "windows1252",
        })
    }
}

/// The decimal separator in numeric literals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum DecimalSep {
    /// `1.5`.
    Dot,
    /// `1,5`.
    Comma,
}

/// The accepted grammar for date literals.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum DateFormat {
    /// `YYYY-MM-DD` (default; no guessing).
    Iso,
    /// `DD/MM/YYYY` with `/`, `-` or `.` separators.
    Dmy,
    /// `MM/DD/YYYY` with `/`, `-` or `.` separators.
    Mdy,
}

/// How records are keyed (spec §4.4); used by the re-import diff.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub enum RecordKey {
    /// A named column provides record ids.
    Column {
        /// The key column's name.
        name: String,
    },
    /// `row:<1-based>` positional ids.
    RowPosition,
}

/// Every knob the import wizard can set (spec §6.1–6.2). `None` means
/// "detect"; deserialization defaults missing keys from `Default`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct ImportSettings {
    /// Text encoding override.
    pub encoding: Option<Encoding>,
    /// Field delimiter override.
    pub delimiter: Option<char>,
    /// First row is a header row (default true).
    pub header: bool,
    /// Decimal separator override.
    pub decimal: Option<DecimalSep>,
    /// Missing-value codes beyond the always-missing empty cell
    /// (default `["", ".", "NA"]`; a custom list replaces it wholly).
    pub missing_codes: Vec<String>,
    /// Accepted date grammar (default `iso`; no guessing).
    pub date_format: DateFormat,
    /// Worksheet name for spreadsheets; ignored for text.
    pub sheet: Option<String>,
    /// Per-column type overrides; non-conforming values become missing.
    pub column_types: BTreeMap<String, ValueType>,
    /// Record id rule (default `row_position`).
    pub key: RecordKey,
}

impl Default for ImportSettings {
    fn default() -> Self {
        Self {
            encoding: None,
            delimiter: None,
            header: true,
            decimal: None,
            missing_codes: ["", ".", "NA"].iter().map(|s| (*s).to_string()).collect(),
            date_format: DateFormat::Iso,
            sheet: None,
            column_types: BTreeMap::new(),
            key: RecordKey::RowPosition,
        }
    }
}

/// What the importer concluded about one file: auto-detection results
/// plus any user overrides — the *effective* reading. `encoding` and
/// `delimiter` are `None` for spreadsheet containers; `sheets` is empty
/// for text.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct Detection {
    /// Physical container.
    pub container: Container,
    /// Text encoding (absent for spreadsheets).
    pub encoding: Option<Encoding>,
    /// Field delimiter (absent for spreadsheets).
    pub delimiter: Option<char>,
    /// Decimal separator (always `dot` for spreadsheets).
    pub decimal: DecimalSep,
    /// `(original, renamed)` for every header cell whose name changed
    /// (duplicate or empty), in column order.
    pub header_renames: Vec<(String, String)>,
    /// Sheet names in workbook order (empty for text).
    pub sheets: Vec<String>,
}
