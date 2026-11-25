//! Case construction for `native_and_wasm_bit_identical`: a
//! `rusqlite`-backed `SqliteSource` (the same storage-class union the
//! crate's golden tests use), the EPTB v1 encoder the worker's TS
//! encoder mirrors, `case_tables` (`{name, batches:[hex]}` per source
//! file) and `native_case` — the direct `epiora-legacy` pipeline the
//! `legacyCase` runner must match byte-for-byte.

use super::{hex_decode, hex_encode};
use crate::{detect_and_map, format_str, parse_format, project_json, project_snapshots};
use epiora_legacy::batch::{RawColumn, RawData, RawTable};
use epiora_legacy::BatchSource;
use epiora_legacy::LegacySource;
use rusqlite::types::Value as SqlValue;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn golden_root() -> PathBuf {
    root().join("fixtures/legacy/golden")
}

/// A `rusqlite`-backed `LegacySource` — the same storage-class
/// union the crate's golden tests use.
struct SqliteSource {
    conn: Connection,
    tables: Vec<String>,
}

impl SqliteSource {
    fn open(path: &Path) -> Self {
        let conn = Connection::open(path).unwrap();
        let tables = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type IN ('table','view') \
                 AND name NOT LIKE 'sqlite_%' ORDER BY rowid",
            )
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        Self { conn, tables }
    }
}

impl LegacySource for SqliteSource {
    fn tables(&self) -> Vec<String> {
        self.tables.clone()
    }

    fn table(&mut self, name: &str) -> Result<RawTable, epiora_legacy::LegacyError> {
        let real = self
            .tables
            .iter()
            .find(|t| t.eq_ignore_ascii_case(name))
            .cloned()
            .unwrap();
        let quoted = format!("\"{}\"", real.replace('"', "\"\""));
        let decls: Vec<String> = self
            .conn
            .prepare(&format!("PRAGMA table_info({quoted})"))
            .unwrap()
            .query_map([], |r| r.get::<_, String>(2))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let mut stmt = self
            .conn
            .prepare(&format!("SELECT * FROM {quoted}"))
            .unwrap();
        let ncols = stmt.column_count();
        let col_names: Vec<String> = (0..ncols)
            .map(|i| stmt.column_name(i).unwrap().to_string())
            .collect();
        let mut rows: Vec<Vec<SqlValue>> = Vec::new();
        let mut q = stmt.query([]).unwrap();
        while let Some(r) = q.next().unwrap() {
            rows.push(
                (0..ncols)
                    .map(|i| r.get::<_, SqlValue>(i).unwrap_or(SqlValue::Null))
                    .collect(),
            );
        }
        let mut columns = Vec::with_capacity(ncols);
        for (i, cname) in col_names.iter().enumerate() {
            let vals: Vec<&SqlValue> = rows.iter().map(|r| &r[i]).collect();
            let has = |f: &dyn Fn(&&SqlValue) -> bool| {
                vals.iter().any(|v| !matches!(v, SqlValue::Null) && f(v))
            };
            let data = if has(&|v| matches!(v, SqlValue::Text(_))) {
                RawData::Text(
                    vals.iter()
                        .map(|v| match v {
                            SqlValue::Null => None,
                            SqlValue::Text(s) => Some(s.clone()),
                            SqlValue::Integer(i) => Some(i.to_string()),
                            SqlValue::Real(f) => Some(format!("{f}")),
                            SqlValue::Blob(_) => None,
                        })
                        .collect(),
                )
            } else if has(&|v| matches!(v, SqlValue::Real(_))) {
                RawData::Float(
                    vals.iter()
                        .map(|v| match v {
                            SqlValue::Integer(i) => Some(*i as f64),
                            SqlValue::Real(f) => Some(*f),
                            _ => None,
                        })
                        .collect(),
                )
            } else if has(&|v| matches!(v, SqlValue::Integer(_))) {
                RawData::Int(
                    vals.iter()
                        .map(|v| match v {
                            SqlValue::Integer(i) => Some(*i),
                            _ => None,
                        })
                        .collect(),
                )
            } else if has(&|v| matches!(v, SqlValue::Blob(_))) {
                RawData::BinarySkipped
            } else {
                let decl = decls.get(i).cloned().unwrap_or_default().to_lowercase();
                if decl.contains("blob") {
                    RawData::BinarySkipped
                } else {
                    RawData::Int(vals.iter().map(|_| None).collect())
                }
            };
            columns.push(RawColumn {
                name: cname.clone(),
                source_type: 0,
                data,
            });
        }
        Ok(RawTable {
            name: real,
            code_page: None,
            decode_errors: 0,
            columns,
        })
    }
}

/// The inverse of `batch::decode_batch` — EPTB v1 from a RawTable
/// (the same bytes the worker's TS encoder writes).
fn encode_table(t: &RawTable) -> Vec<u8> {
    fn raw_len(d: &RawData) -> usize {
        match d {
            RawData::Int(v) => v.len(),
            RawData::Float(v) => v.len(),
            RawData::Bool(v) => v.len(),
            RawData::Text(v) => v.len(),
            RawData::DateTime(v) => v.len(),
            RawData::BinarySkipped => 0,
        }
    }
    let rows = t
        .columns
        .iter()
        .find_map(|c| match &c.data {
            RawData::BinarySkipped => None,
            other => Some(raw_len(other)),
        })
        .unwrap_or(0) as u32;
    let mut b = Vec::new();
    b.extend_from_slice(b"EPTB");
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&rows.to_le_bytes());
    b.extend_from_slice(&(t.columns.len() as u16).to_le_bytes());
    let mut ext = Vec::new();
    ext.push(1u8);
    ext.extend_from_slice(&(t.name.len() as u16).to_le_bytes());
    ext.extend_from_slice(t.name.as_bytes());
    if let Some(cp) = &t.code_page {
        ext.push(2u8);
        ext.extend_from_slice(&(cp.len() as u16).to_le_bytes());
        ext.extend_from_slice(cp.as_bytes());
    }
    if t.decode_errors > 0 {
        ext.push(3u8);
        ext.extend_from_slice(&4u16.to_le_bytes());
        ext.extend_from_slice(&t.decode_errors.to_le_bytes());
    }
    b.extend_from_slice(&(ext.len() as u16).to_le_bytes());
    b.extend_from_slice(&ext);
    let bitmap_len = (rows as usize).div_ceil(8);
    for c in &t.columns {
        b.extend_from_slice(&(c.name.len() as u32).to_le_bytes());
        b.extend_from_slice(c.name.as_bytes());
        b.push(c.source_type);
        let kind = match &c.data {
            RawData::Int(_) => 0u8,
            RawData::Float(_) => 1,
            RawData::Text(_) => 2,
            RawData::Bool(_) => 3,
            RawData::DateTime(_) => 4,
            RawData::BinarySkipped => 5,
        };
        b.push(kind);
        if kind == 5 {
            continue;
        }
        let missing: Vec<bool> = match &c.data {
            RawData::Int(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::Float(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::Text(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::Bool(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::DateTime(v) => v.iter().map(|x| x.is_none()).collect(),
            RawData::BinarySkipped => Vec::new(),
        };
        let mut bitmap = vec![0u8; bitmap_len];
        for (r, m) in missing.iter().enumerate() {
            if *m {
                bitmap[r / 8] |= 1 << (r % 8);
            }
        }
        b.extend_from_slice(&bitmap);
        match &c.data {
            RawData::Int(v) => {
                for x in v.iter().flatten() {
                    b.extend_from_slice(&x.to_le_bytes());
                }
            }
            RawData::Float(v) | RawData::DateTime(v) => {
                for x in v.iter().flatten() {
                    b.extend_from_slice(&x.to_le_bytes());
                }
            }
            RawData::Bool(v) => {
                for x in v.iter().flatten() {
                    b.push(u8::from(*x));
                }
            }
            RawData::Text(v) => {
                let mut blob = Vec::new();
                let mut offs = Vec::new();
                for s in v.iter().flatten() {
                    offs.push(blob.len() as u32);
                    blob.extend_from_slice(s.as_bytes());
                    blob.push(0);
                }
                b.extend_from_slice(&(blob.len() as u32).to_le_bytes());
                b.extend_from_slice(&blob);
                for o in offs {
                    b.extend_from_slice(&o.to_le_bytes());
                }
            }
            RawData::BinarySkipped => {}
        }
    }
    b
}

/// Every batch of one source file as `{name, batches:[hex]}` —
/// `.mdb`/`.accdb` through `epiora-mdb`, `.db` through the sqlite
/// source + EPTB encoder.
pub fn case_tables(path: &Path) -> Vec<Value> {
    let ext = path
        .extension()
        .unwrap_or_default()
        .to_string_lossy()
        .to_lowercase();
    if ext == "db" {
        let mut src = SqliteSource::open(path);
        return src
            .tables()
            .iter()
            .map(|t| {
                let raw = src.table(t).unwrap();
                json!({"name": t, "batches": [hex_encode(&encode_table(&raw))]})
            })
            .collect();
    }
    let mdb = epiora_mdb::Mdb::open(path).unwrap();
    mdb.tables()
        .unwrap()
        .iter()
        .map(|t| {
            let batch = mdb.batch(t, 0, u32::MAX).unwrap();
            json!({"name": t, "batches": [hex_encode(&batch)]})
        })
        .collect()
}

/// The identical batches through `epiora-legacy` directly — the
/// native reference `legacyCase` must match byte-for-byte.
pub fn native_case(input: &str) -> String {
    let v: Value = serde_json::from_str(input).unwrap();
    let name = v["name"].as_str().unwrap_or("").to_string();
    let mut src = BatchSource::new();
    for t in v["tables"].as_array().unwrap() {
        let tname = t["name"].as_str().unwrap();
        for b in t["batches"].as_array().unwrap() {
            src.push(tname, &hex_decode(b.as_str().unwrap()).unwrap())
                .unwrap();
        }
    }
    let fmt = parse_format(v["format"].as_str().unwrap_or(""));
    let (format, project) = detect_and_map(&mut src, fmt, &name).unwrap();
    let mut out = String::from("{\"detected\":");
    out.push_str(&serde_json::json!(format_str(format)).to_string());
    out.push_str(",\"project\":");
    out.push_str(&project_json(&project).unwrap());
    out.push_str(",\"snapshots\":[");
    for (i, s) in project_snapshots(&project).unwrap().iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&hex_encode(s));
        out.push('"');
    }
    out.push_str("]}");
    out
}
