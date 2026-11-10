// Tauri store commands (v0.3 plan Task 14, spec §7.3): the app opens
// the native store once at startup with the keychain key; no command
// takes or returns a key. Bodies cross the IPC as raw bytes — a small
// length-prefixed binary frame — so blob params and result cells are
// never JSON arrays.
//
// Request frame (store_exec / store_query; the tx commands take an
// empty body): u32 sql_len, sql UTF-8, u32 param_count, then per
// param a u8 tag (0 null, 1 i64 LE, 2 f64 LE, 3 text u32+utf8,
// 4 blob u32+bytes).
// Response frame: u8 status (0 ok, 1 error). Error: u16 code_len +
// code utf8 + u32 msg_len + msg utf8. Ok: u32 row_count, then per row
// u32 cell_count and per cell u16 name_len + name utf8 + typed value.

use epiora_store_native::{NativeStore, SqlValue, StoreErr};
use std::sync::Mutex;
use tauri::ipc::{InvokeBody, Request, Response};

/// Managed state: the store connection opened at startup, or the
/// startup failure every command reports.
pub enum StoreState {
    Ready(Mutex<NativeStore>),
    Failed { code: &'static str, message: String },
}

impl StoreState {
    pub fn ready(store: NativeStore) -> Self {
        StoreState::Ready(Mutex::new(store))
    }

    pub fn failed(code: &'static str, message: String) -> Self {
        StoreState::Failed { code, message }
    }
}

fn wire_error(code: &'static str, message: &str) -> Response {
    let mut out = Vec::with_capacity(1 + 2 + code.len() + 4 + message.len());
    out.push(1u8);
    out.extend_from_slice(&(code.len() as u16).to_le_bytes());
    out.extend_from_slice(code.as_bytes());
    out.extend_from_slice(&(message.len() as u32).to_le_bytes());
    out.extend_from_slice(message.as_bytes());
    Response::new(out)
}

fn wire_rows(rows: &[Vec<(String, SqlValue)>]) -> Response {
    let mut out = Vec::new();
    out.push(0u8);
    out.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    for row in rows {
        out.extend_from_slice(&(row.len() as u32).to_le_bytes());
        for (name, value) in row {
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            match value {
                SqlValue::Null => out.push(0u8),
                SqlValue::Integer(n) => {
                    out.push(1u8);
                    out.extend_from_slice(&n.to_le_bytes());
                }
                SqlValue::Real(x) => {
                    out.push(2u8);
                    out.extend_from_slice(&x.to_le_bytes());
                }
                SqlValue::Text(s) => {
                    out.push(3u8);
                    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
                    out.extend_from_slice(s.as_bytes());
                }
                SqlValue::Blob(b) => {
                    out.push(4u8);
                    out.extend_from_slice(&(b.len() as u32).to_le_bytes());
                    out.extend_from_slice(b);
                }
            }
        }
    }
    Response::new(out)
}

fn err_frame(e: &StoreErr) -> Response {
    wire_error(e.code().as_str(), e.message())
}

fn take_u32(buf: &[u8], pos: &mut usize) -> Result<u32, StoreErr> {
    let end = *pos + 4;
    if end > buf.len() {
        return Err(bad_frame("truncated u32"));
    }
    let v = u32::from_le_bytes(buf[*pos..end].try_into().unwrap_or([0; 4]));
    *pos = end;
    Ok(v)
}

fn take_bytes<'a>(buf: &'a [u8], pos: &mut usize, len: usize) -> Result<&'a [u8], StoreErr> {
    let end = *pos + len;
    if end > buf.len() {
        return Err(bad_frame("truncated bytes"));
    }
    let v = &buf[*pos..end];
    *pos = end;
    Ok(v)
}

fn bad_frame(detail: &str) -> StoreErr {
    StoreErr::bad_request(detail)
}

fn decode_request(body: &[u8]) -> Result<(String, Vec<SqlValue>), StoreErr> {
    let mut pos = 0usize;
    let sql_len = take_u32(body, &mut pos)? as usize;
    let sql = std::str::from_utf8(take_bytes(body, &mut pos, sql_len)?)
        .map_err(|_| bad_frame("sql is not UTF-8"))?
        .to_string();
    let n = take_u32(body, &mut pos)? as usize;
    let mut params = Vec::with_capacity(n);
    for _ in 0..n {
        let tag = *take_bytes(body, &mut pos, 1)?.first().unwrap_or(&0xff);
        params.push(match tag {
            0 => SqlValue::Null,
            1 => {
                let b = take_bytes(body, &mut pos, 8)?;
                let mut a = [0u8; 8];
                a.copy_from_slice(b);
                SqlValue::Integer(i64::from_le_bytes(a))
            }
            2 => {
                let b = take_bytes(body, &mut pos, 8)?;
                let mut a = [0u8; 8];
                a.copy_from_slice(b);
                SqlValue::Real(f64::from_le_bytes(a))
            }
            3 => {
                let len = take_u32(body, &mut pos)? as usize;
                let b = take_bytes(body, &mut pos, len)?;
                SqlValue::Text(
                    std::str::from_utf8(b)
                        .map_err(|_| bad_frame("text param is not UTF-8"))?
                        .to_string(),
                )
            }
            4 => {
                let len = take_u32(body, &mut pos)? as usize;
                SqlValue::Blob(take_bytes(body, &mut pos, len)?.to_vec())
            }
            _ => return Err(bad_frame("unknown param tag")),
        });
    }
    Ok((sql, params))
}

fn with_store<R>(
    state: &StoreState,
    request: &Request<'_>,
    f: impl Fn(&NativeStore, String, Vec<SqlValue>) -> Result<R, StoreErr>,
    respond: impl Fn(R) -> Response,
) -> Response {
    let (sql, params) = match decode_request(match request.body() {
        InvokeBody::Raw(b) => b,
        InvokeBody::Json(_) => &[],
    }) {
        Ok(v) => v,
        Err(e) => return err_frame(&e),
    };
    match state {
        StoreState::Failed { code, message } => wire_error(code, message),
        StoreState::Ready(m) => match m.lock() {
            Err(_) => wire_error("sql", "store lock poisoned"),
            Ok(store) => match f(&store, sql, params) {
                Ok(v) => respond(v),
                Err(e) => err_frame(&e),
            },
        },
    }
}

fn rows_response(_: ()) -> Response {
    wire_rows(&[])
}

#[tauri::command]
pub fn store_exec(state: tauri::State<'_, StoreState>, request: Request<'_>) -> Response {
    with_store(
        &state,
        &request,
        |store, sql, params| store.exec(&sql, &params),
        rows_response,
    )
}

#[tauri::command]
pub fn store_query(state: tauri::State<'_, StoreState>, request: Request<'_>) -> Response {
    with_store(
        &state,
        &request,
        |store, sql, params| store.query(&sql, &params),
        |rows| wire_rows(&rows),
    )
}

#[tauri::command]
pub fn store_tx_begin(state: tauri::State<'_, StoreState>, request: Request<'_>) -> Response {
    tx_op(&state, &request, NativeStore::begin)
}

#[tauri::command]
pub fn store_tx_commit(state: tauri::State<'_, StoreState>, request: Request<'_>) -> Response {
    tx_op(&state, &request, NativeStore::commit)
}

#[tauri::command]
pub fn store_tx_rollback(state: tauri::State<'_, StoreState>, request: Request<'_>) -> Response {
    tx_op(&state, &request, NativeStore::rollback)
}

fn tx_op(
    state: &StoreState,
    _request: &Request<'_>,
    op: impl Fn(&NativeStore) -> Result<(), StoreErr>,
) -> Response {
    match state {
        StoreState::Failed { code, message } => wire_error(code, message),
        StoreState::Ready(m) => match m.lock() {
            Err(_) => wire_error("sql", "store lock poisoned"),
            Ok(store) => match op(&store) {
                Ok(()) => wire_rows(&[]),
                Err(e) => err_frame(&e),
            },
        },
    }
}
