//! Fixture + raw-table loading for the v1.1 EI7 corpus runner.

use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_legacy::batch::{decode_batch, RawTable};
use epiora_mdb::Mdb;
use serde_json::Value as Json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[derive(serde::Deserialize)]
pub struct Fixture {
    pub steps: Vec<Step>,
}
#[derive(serde::Deserialize)]
pub struct Step {
    #[serde(default)]
    pub line: u32,
    pub command: String,
    #[serde(default)]
    pub rows: Option<Vec<Vec<Json>>>,
    #[serde(default)]
    pub results: BTreeMap<String, ResultPin>,
}
#[derive(serde::Deserialize)]
pub struct ResultPin {
    pub value: f64,
    pub decimals: i32,
}

/// EI7's raw-read column typing: Access byte/int → Integer, float →
/// Real, text/memo/guid → Text, datetime serial → DateTime seconds
/// since 1970-01-01, YesNo → 0/1 Integer (EI7's LIST shows 0/1 for
/// them, never True/False).
pub fn raw_table(name: &str, raw: &RawTable) -> Dataset {
    let mut cols = Vec::new();
    for c in &raw.columns {
        let data = match &c.data {
            epiora_legacy::batch::RawData::Int(v) => ColumnData::Integer(v.clone()),
            epiora_legacy::batch::RawData::Float(v) => ColumnData::Real(v.clone()),
            epiora_legacy::batch::RawData::Text(v) => ColumnData::Text(v.clone()),
            epiora_legacy::batch::RawData::Bool(v) => {
                ColumnData::Integer(v.iter().map(|x| x.map(i64::from)).collect())
            }
            epiora_legacy::batch::RawData::DateTime(v) => ColumnData::DateTime(
                v.iter()
                    .map(|x| x.map(|s| ((s - 25569.0) * 86400.0).round() as i64))
                    .collect(),
            ),
            epiora_legacy::batch::RawData::BinarySkipped => continue,
        };
        cols.push(Column {
            name: c.name.clone(),
            data,
        });
    }
    Dataset::new(name, 1, cols).expect("raw table as dataset")
}

pub fn load_table(mdbs: &BTreeMap<String, Mdb>, file: &str, table: &str) -> Dataset {
    let mdb = mdbs
        .get(&file.to_ascii_lowercase())
        .unwrap_or_else(|| panic!("mdb {file}"));
    let bytes = mdb.batch(table, 0, u32::MAX).expect("batch");
    raw_table(table, &decode_batch(&bytes).expect("decode"))
}
