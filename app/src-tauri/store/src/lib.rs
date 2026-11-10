//! `epiora-store-native` — the desktop store backend (v0.3 plan Task 14,
//! spec §7.3). A generic SQL executor over rusqlite; the vendored,
//! patched `libsqlite3-sys` compiles SQLite3 Multiple Ciphers so the
//! database is encrypted at rest. The key is applied once at connect
//! (`PRAGMA hexkey`) and verified with a read; it never leaves this
//! process.

use rusqlite::types::Value;
use rusqlite::{params_from_iter, Connection, ErrorCode};
use std::path::Path;

/// A SQLite value on the wire. `Integer` stays `i64` end to end; the
/// TypeScript side maps values above 2^53 to `bigint` (Task 8 contract).
#[derive(Debug, Clone, PartialEq)]
pub enum SqlValue {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl SqlValue {
    fn to_rusqlite(&self) -> Value {
        match self {
            SqlValue::Null => Value::Null,
            SqlValue::Integer(n) => Value::Integer(*n),
            SqlValue::Real(x) => Value::Real(*x),
            SqlValue::Text(s) => Value::Text(s.clone()),
            SqlValue::Blob(b) => Value::Blob(b.clone()),
        }
    }

    fn from_rusqlite(v: Value) -> SqlValue {
        match v {
            Value::Null => SqlValue::Null,
            Value::Integer(n) => SqlValue::Integer(n),
            Value::Real(x) => SqlValue::Real(x),
            Value::Text(s) => SqlValue::Text(s),
            Value::Blob(b) => SqlValue::Blob(b),
        }
    }
}

/// Stable error codes — the same strings Task 8's `StoreError` uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreErrorCode {
    MigrationFailed,
    TxAborted,
    KeyInvalid,
    /// Task 8 code `quota_exceeded`; a real `SQLITE_FULL` keeps it so a
    /// full disk does not surface as a generic `sql` (review focus 3).
    QuotaExceeded,
    Sql,
}

impl StoreErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            StoreErrorCode::MigrationFailed => "migration_failed",
            StoreErrorCode::TxAborted => "tx_aborted",
            StoreErrorCode::KeyInvalid => "key_invalid",
            StoreErrorCode::QuotaExceeded => "quota_exceeded",
            StoreErrorCode::Sql => "sql",
        }
    }
}

/// The one error type this crate produces; `code()` is stable and the
/// message never contains key material.
#[derive(Debug)]
pub struct StoreErr {
    code: StoreErrorCode,
    message: String,
}

impl StoreErr {
    /// A malformed request frame (client bug): reported as `sql`.
    pub fn bad_request(detail: &str) -> StoreErr {
        StoreErr {
            code: StoreErrorCode::Sql,
            message: format!("bad request frame: {detail}"),
        }
    }

    fn at(code: StoreErrorCode, e: rusqlite::Error) -> StoreErr {
        StoreErr {
            code,
            message: e.to_string(),
        }
    }

    fn sql(e: rusqlite::Error) -> StoreErr {
        let code = match &e {
            rusqlite::Error::SqliteFailure(se, _) if se.code == ErrorCode::DiskFull => {
                StoreErrorCode::QuotaExceeded
            }
            _ => StoreErrorCode::Sql,
        };
        StoreErr::at(code, e)
    }

    pub fn code(&self) -> StoreErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for StoreErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for StoreErr {}

fn hex(key: &[u8; 32]) -> String {
    let mut s = String::with_capacity(64);
    for b in key {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// One encrypted SQLite connection. `PRAGMA hexkey` is the first
/// statement after open (sqlite3mc's default cipher, chacha20), then a
/// real read proves the key — a wrong key fails there, never later.
pub struct NativeStore {
    conn: Connection,
}

impl NativeStore {
    pub fn open(path: &Path, key: &[u8; 32]) -> Result<NativeStore, StoreErr> {
        let conn = Connection::open(path).map_err(StoreErr::sql)?;
        conn.execute_batch(&format!("PRAGMA hexkey = '{}'", hex(key)))
            .map_err(|e| StoreErr::at(StoreErrorCode::KeyInvalid, e))?;
        // The first real read proves the key: with a wrong key
        // sqlite3mc reports page 1 as "not a database"/still encrypted.
        // Either way the file can never be opened with this key.
        conn.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(()))
            .map_err(|e| StoreErr::at(StoreErrorCode::KeyInvalid, e))?;
        Ok(NativeStore { conn })
    }

    pub fn exec(&self, sql: &str, params: &[SqlValue]) -> Result<(), StoreErr> {
        if params.is_empty() {
            // DDL comes through as multi-statement strings (migrations);
            // execute() prepares only the first statement.
            self.conn.execute_batch(sql).map_err(StoreErr::sql)
        } else {
            self.conn
                .execute(
                    sql,
                    params_from_iter(params.iter().map(SqlValue::to_rusqlite)),
                )
                .map(|_| ())
                .map_err(StoreErr::sql)
        }
    }

    pub fn query(
        &self,
        sql: &str,
        params: &[SqlValue],
    ) -> Result<Vec<Vec<(String, SqlValue)>>, StoreErr> {
        let mut stmt = self.conn.prepare(sql).map_err(StoreErr::sql)?;
        let cols: Vec<String> = stmt.column_names().map(|s| s.to_string()).collect();
        let mut out = Vec::new();
        let mut rows = stmt
            .query(params_from_iter(params.iter().map(SqlValue::to_rusqlite)))
            .map_err(StoreErr::sql)?;
        while let Some(row) = rows.next().map_err(StoreErr::sql)? {
            let mut cells = Vec::with_capacity(cols.len());
            for (i, name) in cols.iter().enumerate() {
                let v = row.get::<_, Value>(i).map_err(StoreErr::sql)?;
                cells.push((name.clone(), SqlValue::from_rusqlite(v)));
            }
            out.push(cells);
        }
        Ok(out)
    }

    pub fn begin(&self) -> Result<(), StoreErr> {
        self.conn
            .execute_batch("BEGIN")
            .map_err(|e| StoreErr::at(StoreErrorCode::TxAborted, e))
    }

    pub fn commit(&self) -> Result<(), StoreErr> {
        self.conn
            .execute_batch("COMMIT")
            .map_err(|e| StoreErr::at(StoreErrorCode::TxAborted, e))
    }

    pub fn rollback(&self) -> Result<(), StoreErr> {
        self.conn
            .execute_batch("ROLLBACK")
            .map_err(|e| StoreErr::at(StoreErrorCode::TxAborted, e))
    }
}
