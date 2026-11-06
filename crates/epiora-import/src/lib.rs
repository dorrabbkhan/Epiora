//! epiora-import: dataset import for Epiora (v0.3 spec §6).
//!
//! Text containers: magic/encoding/delimiter/decimal detection over the
//! first 64 KiB, `encoding_rs` decoding, quote-aware `csv` reading with
//! `RaggedRow` errors. Spreadsheets: `calamine` reads the used range of
//! the chosen sheet; date cells become ISO text via [`serial_to_days`],
//! error cells and bad serials force coercions, then §6.2 inference
//! and coercion run as for text. Deterministic: no clock, environment
//! or locale reads; every failure is an [`ImportError`] with a stable
//! `code()`; no input may panic.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod coerce;
mod decode;
mod detect;
mod error;
mod excel_date;
mod infer;
mod settings;
mod sheet;
mod table;
mod text;

pub use error::ImportError;
pub use excel_date::serial_to_days;
pub use settings::{
    Container, DateFormat, DecimalSep, Detection, Encoding, ImportSettings, RecordKey,
};

use epiora_core::dataset::{Column, ValueType};
use sha2::Digest;
use std::collections::BTreeMap;

/// Rows surfaced in an [`Inspection`] preview.
pub const PREVIEW_ROWS: usize = 100;
/// Maximum fields per record (spec §6.3).
pub const MAX_COLUMNS: usize = 16_384;
/// Maximum data rows (spec §6.3).
pub const MAX_ROWS: u64 = (1 << 31) - 1;

/// One column in an [`Inspection`] preview.
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewColumn {
    /// Final (uniquified) name.
    pub name: String,
    /// Type the data suggests.
    pub inferred: ValueType,
    /// Type that will be used (`column_types` override or `inferred`).
    pub chosen: ValueType,
    /// Cells that were missing codes.
    pub missing: u64,
    /// Non-missing cells failing `chosen` (→ missing on import).
    pub coerced_to_missing: u64,
}

/// `inspect`/`inspect_with` result: effective detection and settings,
/// per-column preview and the first [`PREVIEW_ROWS`] rows.
#[derive(Clone, Debug)]
pub struct Inspection {
    /// The effective reading of the file's format.
    pub detection: Detection,
    /// Effective settings (detected values fill unset overrides).
    pub settings: ImportSettings,
    /// Per-column preview.
    pub columns: Vec<PreviewColumn>,
    /// First [`PREVIEW_ROWS`] rows; missing codes are `None`, other
    /// cells keep their raw text.
    pub preview_rows: Vec<Vec<Option<String>>>,
    /// Total data rows.
    pub row_count: u64,
}

/// `import` result: typed columns plus the audit trail.
#[derive(Clone, Debug)]
pub struct Imported {
    /// Typed columns in file order.
    pub columns: Vec<Column>,
    /// The effective reading of the file's format.
    pub detection: Detection,
    /// Effective settings (detected values fill unset overrides).
    pub settings: ImportSettings,
    /// Per-column count of non-missing values coerced to missing by a
    /// `column_types` override (every column listed, zero included).
    pub coercions: BTreeMap<String, u64>,
    /// Data row count.
    pub row_count: u64,
    /// Lowercase hex SHA-256 of the input bytes (spec §6.3).
    pub source_sha256: String,
}

/// Inspects `bytes` with default settings (spec §6.1–6.2). `file_name`
/// is reserved for extension hints; detection is content-based.
pub fn inspect(bytes: &[u8], file_name: &str) -> Result<Inspection, ImportError> {
    inspect_with(bytes, file_name, &ImportSettings::default())
}

/// Inspects `bytes` with explicit settings; detection fills unset
/// overrides.
pub fn inspect_with(
    bytes: &[u8],
    file_name: &str,
    settings: &ImportSettings,
) -> Result<Inspection, ImportError> {
    let p = prepare(bytes, file_name, settings)?;
    let ctx = infer::InferCtx {
        decimal: p.settings.decimal.unwrap_or(DecimalSep::Dot),
        date_format: p.settings.date_format,
    };
    let mut columns = Vec::with_capacity(p.table.names.len());
    for (i, name) in p.table.names.iter().enumerate() {
        let non_missing = non_missing(&p.table, i, &p.settings.missing_codes);
        let inferred = infer::infer(&non_missing, &ctx);
        let chosen = p
            .settings
            .column_types
            .get(name)
            .copied()
            .unwrap_or(inferred);
        let c = coerce::coerce_column(&p.table, i, chosen, &p.settings.missing_codes, &ctx);
        columns.push(PreviewColumn {
            name: name.clone(),
            inferred,
            chosen,
            missing: c.missing,
            coerced_to_missing: c.coerced,
        });
    }
    let preview_rows = p
        .table
        .rows
        .iter()
        .enumerate()
        .take(PREVIEW_ROWS)
        .map(|(r, row)| {
            row.iter()
                .enumerate()
                .map(|(c, cell)| {
                    if p.table.forced_coerced.contains(&(r, c))
                        || infer::is_missing(cell, &p.settings.missing_codes)
                    {
                        None
                    } else {
                        Some(cell.clone())
                    }
                })
                .collect()
        })
        .collect();
    Ok(Inspection {
        detection: p.detection,
        settings: p.settings,
        columns,
        preview_rows,
        row_count: p.table.rows.len() as u64,
    })
}

/// Imports `bytes` into typed columns (spec §6.1–6.3).
pub fn import(
    bytes: &[u8],
    file_name: &str,
    settings: &ImportSettings,
) -> Result<Imported, ImportError> {
    let p = prepare(bytes, file_name, settings)?;
    let ctx = infer::InferCtx {
        decimal: p.settings.decimal.unwrap_or(DecimalSep::Dot),
        date_format: p.settings.date_format,
    };
    let mut columns = Vec::with_capacity(p.table.names.len());
    let mut coercions = BTreeMap::new();
    for (i, name) in p.table.names.iter().enumerate() {
        let non_missing = non_missing(&p.table, i, &p.settings.missing_codes);
        let inferred = infer::infer(&non_missing, &ctx);
        let chosen = p
            .settings
            .column_types
            .get(name)
            .copied()
            .unwrap_or(inferred);
        let c = coerce::coerce_column(&p.table, i, chosen, &p.settings.missing_codes, &ctx);
        coercions.insert(name.clone(), c.coerced);
        columns.push(Column {
            name: name.clone(),
            data: c.data,
        });
    }
    Ok(Imported {
        columns,
        detection: p.detection,
        settings: p.settings,
        coercions,
        row_count: p.table.rows.len() as u64,
        source_sha256: sha256_hex(bytes),
    })
}

/// The shared front half of `inspect_with`/`import`: container and
/// encoding detection, decoding, delimiter/decimal detection, csv
/// parsing and settings validation; produces the effective settings.
struct Prepared {
    detection: Detection,
    settings: ImportSettings,
    table: table::Table,
}

fn prepare(bytes: &[u8], file_name: &str, s: &ImportSettings) -> Result<Prepared, ImportError> {
    let _ = file_name; // extension is a hint only (spec §6.1.3)
    match detect::container(bytes) {
        Container::Text => prepare_text(bytes, s),
        container => prepare_sheet(bytes, s, container),
    }
}

/// The text path of `prepare`: encoding detection, decoding,
/// delimiter/decimal detection, csv parsing and settings validation.
fn prepare_text(bytes: &[u8], s: &ImportSettings) -> Result<Prepared, ImportError> {
    let encoding = s.encoding.unwrap_or_else(|| detect::encoding(bytes));
    let text = decode::decode(bytes, encoding, s.encoding.is_some())?;
    if text.is_empty() {
        return Err(ImportError::EmptyFile);
    }
    let sample = detect::sample(&text);
    let delimiter = s.delimiter.unwrap_or_else(|| detect::delimiter(sample));
    let delim_u8 = u8::try_from(delimiter).map_err(|_| ImportError::InvalidSettings {
        reason: format!("delimiter {delimiter:?} is not a byte"),
    })?;
    let decimal = s
        .decimal
        .unwrap_or_else(|| detect::decimal(sample, delimiter));
    let table = text::parse(&text, delim_u8, s.header)?;
    if let Some(name) = &s.sheet {
        return Err(ImportError::SheetNotFound { name: name.clone() });
    }
    check_columns(&table, s)?;
    Ok(Prepared {
        detection: Detection {
            container: Container::Text,
            encoding: Some(encoding),
            delimiter: Some(delimiter),
            decimal,
            header_renames: table.renames.clone(),
            sheets: Vec::new(),
        },
        settings: ImportSettings {
            encoding: Some(encoding),
            delimiter: Some(delimiter),
            decimal: Some(decimal),
            ..s.clone()
        },
        table,
    })
}

/// The spreadsheet path of `prepare` (spec §6.1.5): calamine reads the
/// chosen sheet; encoding/delimiter do not apply and decimal is `dot`
/// unless overridden for the sheet's string cells.
fn prepare_sheet(
    bytes: &[u8],
    s: &ImportSettings,
    container: Container,
) -> Result<Prepared, ImportError> {
    let st = sheet::read(bytes, container, s)?;
    check_columns(&st.table, s)?;
    Ok(Prepared {
        detection: Detection {
            container,
            encoding: None,
            delimiter: None,
            decimal: s.decimal.unwrap_or(DecimalSep::Dot),
            header_renames: st.table.renames.clone(),
            sheets: st.sheets,
        },
        settings: ImportSettings {
            decimal: Some(s.decimal.unwrap_or(DecimalSep::Dot)),
            sheet: Some(st.used),
            ..s.clone()
        },
        table: st.table,
    })
}

/// `column_types` keys and `key: {column: …}` must name a real column.
fn check_columns(table: &table::Table, s: &ImportSettings) -> Result<(), ImportError> {
    for name in s.column_types.keys() {
        if !table.names.contains(name) {
            return Err(ImportError::UnknownColumn { name: name.clone() });
        }
    }
    if let RecordKey::Column { name } = &s.key {
        if !table.names.contains(name) {
            return Err(ImportError::UnknownColumn { name: name.clone() });
        }
    }
    Ok(())
}

/// Column `col`'s non-missing, non-forced values, in row order.
fn non_missing<'a>(table: &'a table::Table, col: usize, missing_codes: &[String]) -> Vec<&'a str> {
    table
        .rows
        .iter()
        .enumerate()
        .filter(|(r, _)| !table.forced_coerced.contains(&(*r, col)))
        .map(|(_, r)| r.get(col).map(String::as_str).unwrap_or(""))
        .filter(|v| !infer::is_missing(v, missing_codes))
        .collect()
}

/// Lowercase hex SHA-256 of the raw input bytes.
fn sha256_hex(bytes: &[u8]) -> String {
    let digest = sha2::Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
        hex.push(char::from_digit((byte & 0xf) as u32, 16).unwrap_or('0'));
    }
    hex
}

/// Fuzz entry point (spec §6.5): arbitrary bytes decode into an
/// `ImportSettings` plus input bytes; `import` must return, never panic.
#[cfg(feature = "fuzzing")]
pub fn text_import(data: &[u8]) {
    let mut u = arbitrary::Unstructured::new(data);
    if let Ok((settings, bytes)) = u.arbitrary::<(ImportSettings, Vec<u8>)>() {
        let _ = import(&bytes, "fuzz.csv", &settings);
    }
}

/// Fuzz entry point (spec §6.5): arbitrary bytes behind each container
/// magic (zip, OLE) plus the raw bytes; `import` must return, never
/// panic.
#[cfg(feature = "fuzzing")]
pub fn sheet_import(data: &[u8]) {
    const MAGICS: [&[u8]; 3] = [
        b"PK\x03\x04".as_slice(),
        &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
        b"",
    ];
    let mut u = arbitrary::Unstructured::new(data);
    if let Ok((settings, bytes)) = u.arbitrary::<(ImportSettings, Vec<u8>)>() {
        for magic in MAGICS {
            let mut input = Vec::with_capacity(magic.len() + bytes.len());
            input.extend_from_slice(magic);
            input.extend_from_slice(&bytes);
            let _ = import(&input, "fuzz", &settings);
        }
    }
}
