//! `store-stdio` — test-only JSON-lines executor (v0.3 plan Task 14,
//! spec §7.3). Exposes `NativeStore` over stdin/stdout so the
//! TypeScript conformance suite runs against native SQLite3MC in CI.
//! Never bundled into the app (`required-features = ["stdio"]`).
//!
//! Request:  `{ "id", "op": "open"|"exec"|"query"|"begin"|"commit"|
//! "rollback"|"close"|"test_fail_next_write", "sql"?, "params"?,
//! "path"?, "key_hex"? }`
//! Reply:    `{ "id", "ok": true, "rows"? }` or
//!           `{ "id", "ok": false, "code", "message" }`
//! Values are serde-tagged `{ "t": "i"|"r"|"s"|"b"|"null", "v"? }` —
//! integers as decimal strings (i64 lossless), blobs base64.

use epiora_store_native::{NativeStore, SqlValue, StoreErr};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufWriter, Write};
use std::path::PathBuf;

#[derive(Deserialize)]
struct Req {
    id: serde_json::Value,
    op: String,
    #[serde(default)]
    sql: Option<String>,
    #[serde(default)]
    params: Option<Vec<WireValue>>,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    key_hex: Option<String>,
}

#[derive(Serialize)]
struct Ok {
    id: serde_json::Value,
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    rows: Option<Vec<Vec<(String, WireValue)>>>,
}

#[derive(Serialize)]
struct Err_ {
    id: serde_json::Value,
    ok: bool,
    code: &'static str,
    message: String,
}

/// Wire shape `{ "t", "v"? }` matching the TypeScript codec in
/// app/src/store/stdio.ts.
#[derive(Serialize, Deserialize)]
struct WireValue {
    t: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    v: Option<serde_json::Value>,
}

impl From<&SqlValue> for WireValue {
    fn from(v: &SqlValue) -> WireValue {
        let (t, v) = match v {
            SqlValue::Null => ("null", None),
            SqlValue::Integer(n) => ("i", Some(serde_json::Value::from(n.to_string()))),
            SqlValue::Real(x) => ("r", Some(serde_json::Value::from(*x))),
            SqlValue::Text(s) => ("s", Some(serde_json::Value::from(s.as_str()))),
            SqlValue::Blob(b) => ("b", Some(serde_json::Value::from(b64_encode(b)))),
        };
        WireValue {
            t: t.to_string(),
            v,
        }
    }
}

fn sql_value(w: &WireValue) -> Result<SqlValue, String> {
    match (w.t.as_str(), &w.v) {
        ("null", _) => Ok(SqlValue::Null),
        ("i", Some(serde_json::Value::String(s))) => s
            .parse::<i64>()
            .map(SqlValue::Integer)
            .map_err(|_| format!("bad i64 {s:?}")),
        ("i", Some(serde_json::Value::Number(n))) => n
            .as_i64()
            .map(SqlValue::Integer)
            .ok_or_else(|| "i64 out of range".to_string()),
        ("r", Some(serde_json::Value::Number(n))) => Ok(SqlValue::Real(
            n.as_f64().ok_or_else(|| "bad f64".to_string())?,
        )),
        ("s", Some(serde_json::Value::String(s))) => Ok(SqlValue::Text(s.clone())),
        ("b", Some(serde_json::Value::String(s))) => b64_decode(s).map(SqlValue::Blob),
        (t, _) => Err(format!("bad value tag {t:?}")),
    }
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn b64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            B64[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err(format!("bad base64 byte {c:#x}")),
        }
    }
    let bytes = s.as_bytes();
    if !bytes.len().is_multiple_of(4) {
        return Err("base64 length not a multiple of 4".to_string());
    }
    let mut out = Vec::with_capacity(bytes.len() / 4 * 3);
    for chunk in bytes.chunks(4) {
        let pad = chunk.iter().filter(|&&c| c == b'=').count();
        let n = (val(chunk[0])? << 18)
            | (val(chunk[1])? << 12)
            | (if pad < 2 { val(chunk[2])? << 6 } else { 0 })
            | (if pad < 1 { val(chunk[3])? } else { 0 });
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

fn hex_decode(s: &str) -> Result<[u8; 32], String> {
    let bytes = s.as_bytes();
    if bytes.len() != 64 {
        return Err("key_hex must be 64 lowercase hex".to_string());
    }
    let nibble = |c: u8| -> Result<u8, String> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            _ => Err("key_hex must be lowercase hex".to_string()),
        }
    };
    let mut key = [0u8; 32];
    for (i, pair) in bytes.as_chunks::<2>().0.iter().enumerate() {
        key[i] = (nibble(pair[0])? << 4) | nibble(pair[1])?;
    }
    Ok(key)
}

fn is_write(sql: &str) -> bool {
    let first = sql
        .trim_start()
        .split(|c: char| c.is_whitespace() || c == '(')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    matches!(
        first.as_str(),
        "insert"
            | "update"
            | "delete"
            | "replace"
            | "create"
            | "drop"
            | "alter"
            | "vacuum"
            | "reindex"
            | "attach"
            | "detach"
            | "truncate"
    )
}

fn ok(id: serde_json::Value, rows: Option<Vec<Vec<(String, WireValue)>>>) -> String {
    serde_json::to_string(&Ok { id, ok: true, rows })
        .unwrap_or_else(|_| "{\"ok\":true}".to_string())
}

fn err(id: serde_json::Value, code: &'static str, message: String) -> String {
    serde_json::to_string(&Err_ {
        id,
        ok: false,
        code,
        message,
    })
    .unwrap_or_else(|_| "{\"ok\":false}".to_string())
}

fn sql_err(id: serde_json::Value, e: StoreErr) -> String {
    err(id, e.code().as_str(), e.message().to_string())
}

fn main() -> std::process::ExitCode {
    let stdin = std::io::stdin();
    let mut stdout = BufWriter::new(std::io::stdout());
    let mut store: Option<NativeStore> = None;
    let mut fail_next_write = false;

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let req: Req = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let _ = writeln!(
                    stdout,
                    "{}",
                    err(
                        serde_json::Value::Null,
                        "sql",
                        format!("bad request json: {e}")
                    )
                );
                let _ = stdout.flush();
                continue;
            }
        };
        let id = req.id.clone();
        let reply = match req.op.as_str() {
            "open" => {
                let parsed = (|| -> Result<NativeStore, String> {
                    let path = req.path.ok_or("open needs path")?;
                    let key_hex = req.key_hex.as_deref().ok_or("open needs key_hex")?;
                    let key = hex_decode(key_hex)?;
                    NativeStore::open(&PathBuf::from(path), &key)
                        .map_err(|e| format!("{}|{}", e.code().as_str(), e.message()))
                })();
                match parsed {
                    Ok(s) => {
                        store = Some(s);
                        ok(id, None)
                    }
                    Err(msg) => {
                        let (code, m) = match msg.split_once('|') {
                            Some((c, m)) => (
                                match c {
                                    "key_invalid" => "key_invalid",
                                    "migration_failed" => "migration_failed",
                                    "tx_aborted" => "tx_aborted",
                                    "quota_exceeded" => "quota_exceeded",
                                    _ => "sql",
                                },
                                m.to_string(),
                            ),
                            None => ("sql", msg),
                        };
                        err(id, code, m)
                    }
                }
            }
            "exec" => match (&store, req.sql) {
                (Some(s), Some(sql)) => {
                    if fail_next_write && is_write(&sql) {
                        fail_next_write = false;
                        err(
                            id,
                            "quota_exceeded",
                            "simulated quota exhaustion on write".to_string(),
                        )
                    } else {
                        let params: Vec<SqlValue> = match req
                            .params
                            .unwrap_or_default()
                            .iter()
                            .map(sql_value)
                            .collect()
                        {
                            Ok(p) => p,
                            Err(m) => {
                                let _ = writeln!(stdout, "{}", err(id, "sql", m));
                                let _ = stdout.flush();
                                continue;
                            }
                        };
                        match s.exec(&sql, &params) {
                            Ok(()) => ok(id, None),
                            Err(e) => sql_err(id, e),
                        }
                    }
                }
                (None, _) => err(id, "sql", "store is not open".to_string()),
                (_, None) => err(id, "sql", "exec needs sql".to_string()),
            },
            "query" => match (&store, req.sql) {
                (Some(s), Some(sql)) => {
                    let params: Vec<SqlValue> = match req
                        .params
                        .unwrap_or_default()
                        .iter()
                        .map(sql_value)
                        .collect()
                    {
                        Ok(p) => p,
                        Err(m) => {
                            let _ = writeln!(stdout, "{}", err(id, "sql", m));
                            let _ = stdout.flush();
                            continue;
                        }
                    };
                    match s.query(&sql, &params) {
                        Ok(rows) => ok(
                            id,
                            Some(
                                rows.iter()
                                    .map(|r| {
                                        r.iter()
                                            .map(|(n, v)| (n.clone(), WireValue::from(v)))
                                            .collect()
                                    })
                                    .collect(),
                            ),
                        ),
                        Err(e) => sql_err(id, e),
                    }
                }
                (None, _) => err(id, "sql", "store is not open".to_string()),
                (_, None) => err(id, "sql", "query needs sql".to_string()),
            },
            "begin" => match &store {
                Some(s) => match s.begin() {
                    Ok(()) => ok(id, None),
                    Err(e) => sql_err(id, e),
                },
                None => err(id, "sql", "store is not open".to_string()),
            },
            "commit" => match &store {
                Some(s) => match s.commit() {
                    Ok(()) => ok(id, None),
                    Err(e) => sql_err(id, e),
                },
                None => err(id, "sql", "store is not open".to_string()),
            },
            "rollback" => match &store {
                Some(s) => match s.rollback() {
                    Ok(()) => ok(id, None),
                    Err(e) => sql_err(id, e),
                },
                None => err(id, "sql", "store is not open".to_string()),
            },
            "test_fail_next_write" => {
                fail_next_write = true;
                ok(id, None)
            }
            "close" => {
                let _ = writeln!(stdout, "{}", ok(id, None));
                let _ = stdout.flush();
                return std::process::ExitCode::SUCCESS;
            }
            op => err(id, "sql", format!("unknown op {op:?}")),
        };
        let _ = writeln!(stdout, "{reply}");
        let _ = stdout.flush();
    }
    std::process::ExitCode::SUCCESS
}
