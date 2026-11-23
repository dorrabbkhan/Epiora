//! `LegacySource` impls over the real backends (`epiora-mdb` for
//! Access, `rusqlite` for SQLite), the verbatim `copy-tables.mjs`
//! SQLite writer, and the mdb-export datetime surface text.

use std::path::Path;

use epiora_legacy::batch::{RawColumn, RawData, RawTable};
use epiora_legacy::{BatchSource, LegacySource};
use rusqlite::types::Value as SqlValue;
use rusqlite::Connection;

use super::encode::raw_len;

/// An `epiora-mdb`-backed `BatchSource` for one Access file.
pub fn mdb_source(path: &Path) -> BatchSource {
    let mdb = epiora_mdb::Mdb::open(path).unwrap();
    let mut src = BatchSource::new();
    for t in mdb.tables().unwrap() {
        let batch = mdb.batch(&t, 0, u32::MAX).unwrap();
        src.push(&t, &batch).unwrap();
    }
    src
}

/// A `rusqlite`-backed `LegacySource`: `sqlite_master`'s `table`/`view`
/// names (the `.tables` set the golden uses) and `SELECT *` rows with
/// per-cell storage-class values, like `sqlite3 -json`.
pub struct SqliteSource {
    conn: Connection,
    tables: Vec<String>,
}

impl SqliteSource {
    /// Open a SQLite file.
    pub fn open(path: &Path) -> Self {
        let conn = Connection::open(path).unwrap();
        // `sqlite3 .tables` hides `sqlite_*` internals (the golden's
        // table list) — match it.
        let tables = conn
            .prepare(
                "SELECT name FROM sqlite_master WHERE type IN ('table','view') \
                 AND name NOT LIKE 'sqlite_%' ORDER BY rowid",
            )
            .unwrap()
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<_, _>>()
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
            .ok_or_else(|| epiora_legacy::LegacyError::Source {
                code: "legacy_no_table",
                reason: format!("no table {name:?}"),
            })?;
        let quoted = format!("\"{}\"", real.replace('"', "\"\""));
        let decls: Vec<String> = self
            .conn
            .prepare(&format!("PRAGMA table_info({quoted})"))
            .unwrap()
            .query_map([], |r| r.get::<_, String>(2))
            .unwrap()
            .collect::<Result<_, _>>()
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
        // Per column the storage-class union picks the EPTB kind —
        // text wins, then real, then integer; all-blob columns are the
        // skipped-binary kind.
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

/// A `Vec<RawTable>`-backed source for synthetic inputs.
pub struct VecSource {
    tables: Vec<RawTable>,
}

impl VecSource {
    /// Wrap decoded tables.
    pub fn new(tables: Vec<RawTable>) -> Self {
        Self { tables }
    }
}

impl LegacySource for VecSource {
    fn tables(&self) -> Vec<String> {
        self.tables.iter().map(|t| t.name.clone()).collect()
    }

    fn table(&mut self, name: &str) -> Result<RawTable, epiora_legacy::LegacyError> {
        self.tables
            .iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
            .cloned()
            .ok_or_else(|| epiora_legacy::LegacyError::Source {
                code: "legacy_no_table",
                reason: format!("no table {name:?}"),
            })
    }
}

/// Copy every EPTB table into a fresh SQLite file (verbatim, the
/// `copy-tables.mjs` semantics) and return the temp path.
pub fn copy_to_sqlite(src: &BatchSource, dst: &Path) {
    let conn = Connection::open(dst).unwrap();
    for t in src.tables() {
        let mut s = src.clone();
        let raw = s.table(&t).unwrap();
        let quoted = format!("\"{}\"", t.replace('"', "\"\""));
        let decls: Vec<String> = raw
            .columns
            .iter()
            .map(|c| {
                let ty = match &c.data {
                    RawData::Int(_) | RawData::Bool(_) => "INTEGER",
                    RawData::Float(_) => "REAL",
                    RawData::BinarySkipped => "BLOB",
                    _ => "TEXT",
                };
                format!("\"{}\" {}", c.name.replace('"', "\"\""), ty)
            })
            .collect();
        conn.execute(&format!("CREATE TABLE {quoted} ({})", decls.join(", ")), [])
            .unwrap();
        let nrows = raw
            .columns
            .iter()
            .map(|c| raw_len(&c.data))
            .max()
            .unwrap_or(0);
        if nrows == 0 {
            continue;
        }
        let placeholders = vec!["?"; raw.columns.len()].join(",");
        let mut stmt = conn
            .prepare(&format!("INSERT INTO {quoted} VALUES ({placeholders})"))
            .unwrap();
        for r in 0..nrows {
            let mut params: Vec<SqlValue> = Vec::new();
            for c in &raw.columns {
                params.push(match &c.data {
                    RawData::Int(v) => v[r].map(SqlValue::Integer).unwrap_or(SqlValue::Null),
                    RawData::Float(v) => v[r].map(SqlValue::Real).unwrap_or(SqlValue::Null),
                    RawData::Bool(v) => v[r]
                        .map(|b| SqlValue::Integer(i64::from(b)))
                        .unwrap_or(SqlValue::Null),
                    RawData::DateTime(v) => v[r]
                        .map(|s| SqlValue::Text(access_serial_to_text(s)))
                        .unwrap_or(SqlValue::Null),
                    RawData::Text(v) => v[r].clone().map(SqlValue::Text).unwrap_or(SqlValue::Null),
                    RawData::BinarySkipped => SqlValue::Null,
                });
            }
            stmt.execute(rusqlite::params_from_iter(params)).unwrap();
        }
    }
}

/// The `mdb_date_to_tm` formatting the golden's `mdb-export
/// -T '%Y-%m-%d %H:%M:%S'` uses (needed to write datetime cells into
/// the verbatim SQLite copy — the same values EI7 stores there).
fn access_serial_to_text(serial: f64) -> String {
    if !(0.0..=1e6).contains(&serial) {
        return "1900-01-00 00:00:00".to_string();
    }
    let day = serial.trunc() as i64;
    let secs = ((serial - day as f64) * 86400.0 + 0.5) as i64;
    let z = day - 25569 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}
