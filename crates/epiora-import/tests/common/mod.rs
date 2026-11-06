//! Shared test builders for the `epiora-import` integration tests.

use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_import::{
    Container, DecimalSep, Detection, Encoding, ImportSettings, Imported, RecordKey,
};
use std::collections::BTreeMap;

pub fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

pub fn dataset(columns: Vec<Column>) -> Dataset {
    Dataset::new("d", 1, columns).expect("dataset")
}

/// Wraps columns in an `Imported`; only `columns` and `row_count` are
/// read by `diff`.
pub fn imported(columns: Vec<Column>) -> Imported {
    let row_count = columns.first().map(|c| c.data.len() as u64).unwrap_or(0);
    Imported {
        columns,
        detection: Detection {
            container: Container::Text,
            encoding: Some(Encoding::Utf8),
            delimiter: Some(','),
            decimal: DecimalSep::Dot,
            header_renames: Vec::new(),
            sheets: Vec::new(),
        },
        settings: ImportSettings::default(),
        coercions: BTreeMap::new(),
        row_count,
        source_sha256: String::new(),
    }
}

pub fn key(name: &str) -> RecordKey {
    RecordKey::Column {
        name: name.to_string(),
    }
}
