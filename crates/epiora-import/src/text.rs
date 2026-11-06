//! Quote-aware delimited-text reading via the `csv` crate with
//! `flexible(false)` semantics: a record whose field count differs from
//! the first record's is `RaggedRow` at its 1-based physical line —
//! no silent padding (spec §6.3).

use crate::error::ImportError;
use crate::table::{self, Table};
use crate::{MAX_COLUMNS, MAX_ROWS};
use std::collections::HashSet;

/// Parses `text` into a [`Table`]. `header` consumes the first record
/// as column names (uniquified); without it every record is data and
/// names are generated `column_1..=N`. A decoded-but-recordless input
/// (only blank lines) is `NoColumns`.
pub(crate) fn parse(text: &str, delimiter: u8, header: bool) -> Result<Table, ImportError> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(false)
        .delimiter(delimiter)
        .from_reader(text.as_bytes());
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut record = csv::StringRecord::new();
    loop {
        match reader.read_record(&mut record) {
            Ok(true) => {
                if record.len() > MAX_COLUMNS {
                    return Err(ImportError::TooManyColumns);
                }
                records.push(record.iter().map(str::to_string).collect());
            }
            Ok(false) => break,
            Err(e) => return Err(csv_error(text, e)),
        }
    }
    if records.is_empty() {
        return Err(ImportError::NoColumns);
    }
    let (header_cells, rows): (Option<&[String]>, &[Vec<String>]) = if header {
        match records.split_first() {
            Some((h, rest)) => (Some(h.as_slice()), rest),
            None => (None, &[]),
        }
    } else {
        (None, records.as_slice())
    };
    if rows.len() as u64 > MAX_ROWS {
        return Err(ImportError::TooManyRows);
    }
    let width = header_cells
        .map(|h| h.len())
        .unwrap_or_else(|| records[0].len());
    let (names, renames) = match header_cells {
        Some(h) => table::unique_names(h),
        None => (
            (1..=width).map(|i| format!("column_{i}")).collect(),
            Vec::new(),
        ),
    };
    Ok(Table {
        names,
        renames,
        rows: rows.to_vec(),
        forced_coerced: HashSet::new(),
    })
}

/// Maps a `csv` failure to `ImportError`; only `UnequalLengths` is
/// reachable on valid-UTF-8 in-memory input (`RaggedRow`).
fn csv_error(text: &str, e: csv::Error) -> ImportError {
    match e.into_kind() {
        csv::ErrorKind::UnequalLengths {
            pos,
            expected_len,
            len,
        } => {
            let line = pos.map(|p| p.line()).unwrap_or(0);
            ImportError::RaggedRow {
                line,
                expected: usize::try_from(expected_len).unwrap_or(usize::MAX),
                found: usize::try_from(len).unwrap_or(usize::MAX),
                sample: sample_line(text, line),
            }
        }
        other => ImportError::InvalidSettings {
            reason: format!("csv reader: {other:?}"),
        },
    }
}

/// The 1-based physical line's text, truncated for the error sample.
fn sample_line(text: &str, line: u64) -> String {
    let s = text
        .split('\n')
        .nth(line.saturating_sub(1) as usize)
        .unwrap_or("")
        .trim_end_matches('\r');
    s.chars().take(200).collect()
}
