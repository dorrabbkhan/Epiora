//! epiora-legacy-wasm: the browser binding for `epiora-legacy` (v1.1
//! spec §7, plan Task 13). Type conversion only — the worker pushes
//! EPTB batches in, gets the mapped project + dataset snapshots out,
//! and asks for a re-import `refresh_plan`. No mapping logic.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

#[cfg(feature = "parity")]
mod parity;

use epiora_core::dataset::{Column, ColumnData, Dataset, LinkKind};
use epiora_core::snapshot::encode_snapshot;
use epiora_legacy::{
    batch::{RawData, RawTable},
    detect, map_ei3, map_ei7, refresh_plan, BatchSource, LegacyError, LegacyProject, LegacySource,
    SourceFormat,
};
use js_sys::{Error, Reflect};
use sha2::Digest;
use wasm_bindgen::prelude::*;

/// A `LegacyError` crossing the boundary becomes a JS `Error` whose
/// `code` is the crate's stable code (`legacy_*`, `mdb_*` and the
/// pass-through engine/import codes) and whose `message` is `Display`.
fn js_error(e: &LegacyError) -> JsValue {
    let err = Error::new(&e.to_string());
    let _ = Reflect::set(
        &err,
        &JsValue::from_str("code"),
        &JsValue::from_str(e.code()),
    );
    err.into()
}

/// The `source_format` spelling the golden `project.json` files pin.
pub(crate) fn format_str(f: SourceFormat) -> &'static str {
    match f {
        SourceFormat::Ei7Access => "ei7_access",
        SourceFormat::Ei7Sqlite => "ei7_sqlite",
        SourceFormat::Ei7SqlServer => "ei7_sql_server",
        SourceFormat::Ei3Access => "ei3_access",
    }
}

/// `detect` is layout-only (`Ei7Access`/`Ei3Access`); the caller
/// supplies the true backend `SourceFormat` to `map_ei7` — the file's
/// extension knows it, the batches don't. `detect_and_map` is the one
/// path every caller (wasm `legacyMap`, parity `legacyCase`) takes.
/// `name` is the file-name fallback `map_ei3` needs.
pub(crate) fn detect_and_map(
    source: &mut dyn LegacySource,
    format: Option<SourceFormat>,
    name: &str,
) -> Result<(SourceFormat, LegacyProject), LegacyError> {
    // A supplied backend wins — some valid projects (EI3 files with
    // no `view*` table) map fine yet fool `detect`'s layout-only
    // heuristic. Without it, `detect` guesses from the table names.
    let backend = match format {
        Some(f) => f,
        None => detect(source)?,
    };
    let project = match backend {
        SourceFormat::Ei7Access | SourceFormat::Ei7Sqlite | SourceFormat::Ei7SqlServer => {
            map_ei7(source, backend)?
        }
        SourceFormat::Ei3Access => map_ei3(source, name)?,
    };
    Ok((project.format, project))
}

/// `ei7_access`/`ei7_sqlite`/`ei7_sql_server`/`ei3_access` →
/// `SourceFormat` — the same vocabulary `project_json` emits.
pub(crate) fn parse_format(s: &str) -> Option<SourceFormat> {
    match s {
        "ei7_access" => Some(SourceFormat::Ei7Access),
        "ei7_sqlite" => Some(SourceFormat::Ei7Sqlite),
        "ei7_sql_server" => Some(SourceFormat::Ei7SqlServer),
        "ei3_access" => Some(SourceFormat::Ei3Access),
        _ => None,
    }
}

/// `LegacyProjectJson`: the golden `project.json` vocabulary plus the
/// fields the store needs (`text` for programs and check code, each
/// view's `key_column`, `datasets` for the non-view link children).
/// `snapshots[i]` for `i < views.len` encodes `views[i].dataset`; the
/// rest encode `datasets` in order.
pub(crate) fn project_json(p: &LegacyProject) -> Result<String, LegacyError> {
    let views: Vec<serde_json::Value> = p
        .views
        .iter()
        .map(|v| {
            serde_json::json!({
                "name": v.name,
                "records": v.dataset.row_count(),
                "deleted_records": v.deleted_records,
                "hash": v.dataset.content_hash_hex(),
                "key_column": v.key_column,
            })
        })
        .collect();
    let datasets: Vec<serde_json::Value> = p
        .datasets
        .iter()
        .map(|d| {
            serde_json::json!({
                "name": d.id(),
                "records": d.row_count(),
                "hash": d.content_hash_hex(),
            })
        })
        .collect();
    let links: Vec<serde_json::Value> = p
        .links
        .iter()
        .map(|l| {
            serde_json::json!({
                "parent": l.parent,
                "child": l.child,
                "child_key": l.child_key,
                "parent_key": l.parent_key,
                "kind": match l.kind {
                    LinkKind::Relate => "relate",
                    LinkKind::Grid => "grid",
                },
            })
        })
        .collect();
    let programs: Vec<serde_json::Value> = p
        .programs
        .iter()
        .map(|prog| {
            let digest = sha2::Sha256::digest(prog.text.as_bytes());
            let text_sha256 = digest
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();
            serde_json::json!({
                "name": prog.name,
                "text": prog.text,
                "text_sha256": text_sha256,
            })
        })
        .collect();
    let check_code: Vec<serde_json::Value> = p
        .check_code
        .iter()
        .map(|c| {
            serde_json::json!({
                "view": c.view,
                "blocks": c.blocks,
                "text": c.text,
            })
        })
        .collect();
    let report: Vec<serde_json::Value> = p
        .report
        .iter()
        .map(|r| {
            serde_json::json!({
                "kind": r.kind.as_str(),
                "item": r.item,
                "reason": r.reason,
            })
        })
        .collect();
    let out = serde_json::json!({
        "name": p.name,
        "legacy_id": p.legacy_id,
        "source_format": format_str(p.format),
        "views": views,
        "datasets": datasets,
        "links": links,
        "programs": programs,
        "check_code": check_code,
        "report": report,
    });
    serde_json::to_string(&out).map_err(|e| LegacyError::Source {
        code: "legacy_view_broken",
        reason: format!("project JSON failed: {e}"),
    })
}

/// `views` datasets then `datasets`, in order — the layout
/// `snapshots[]` in `legacyMap`'s return follows.
pub(crate) fn project_snapshots(p: &LegacyProject) -> Result<Vec<Vec<u8>>, LegacyError> {
    let mut out = Vec::with_capacity(p.views.len() + p.datasets.len());
    for v in &p.views {
        out.push(encode_snapshot(&v.dataset));
    }
    for d in &p.datasets {
        out.push(encode_snapshot(d));
    }
    Ok(out)
}

/// One `previous` packet in `legacyRefreshPlan`: the worker cannot
/// pass `(name, version, snapshot)` triples through a `Uint8Array[]`,
/// so each element self-describes —
/// `u32le name_len | name utf8 | u32le version | snapshot bytes`.
pub(crate) fn decode_previous(packets: &[Vec<u8>]) -> Result<Vec<(String, Dataset)>, LegacyError> {
    let mut out = Vec::with_capacity(packets.len());
    for p in packets {
        let bad = |why: &str| js_error_reason(format!("previous packet: {why}"));
        if p.len() < 4 {
            return Err(bad("truncated name length"));
        }
        let nlen = u32::from_le_bytes([p[0], p[1], p[2], p[3]]) as usize;
        if p.len() < 4 + nlen + 4 {
            return Err(bad("truncated name or version"));
        }
        let name = std::str::from_utf8(&p[4..4 + nlen])
            .map_err(|_| bad("name is not utf8"))?
            .to_string();
        let vb = &p[4 + nlen..4 + nlen + 4];
        let version = u64::from(u32::from_le_bytes([vb[0], vb[1], vb[2], vb[3]]));
        let snap = &p[4 + nlen + 4..];
        let ds = epiora_core::snapshot::decode_snapshot(&name, version, snap)
            .map_err(LegacyError::Engine)?;
        out.push((name, ds));
    }
    Ok(out)
}

fn js_error_reason(reason: String) -> LegacyError {
    LegacyError::BatchInvalid { reason }
}

/// One ordinary table as a plain dataset: EPTB kinds are already the
/// column types, binary columns are skipped (the reader never exports
/// them), and Access serial datetimes become epoch seconds. This is
/// deliberately not the Epi Info field-type mapper — an unknown-layout
/// file carries no field metadata to apply.
fn plain_dataset(raw: RawTable) -> Result<Dataset, LegacyError> {
    let mut columns = Vec::new();
    for col in raw.columns {
        let data = match col.data {
            RawData::Int(v) => ColumnData::Integer(v),
            RawData::Float(v) => ColumnData::Real(v),
            RawData::Text(v) => ColumnData::Text(v),
            RawData::Bool(v) => ColumnData::Boolean(v),
            RawData::DateTime(v) => ColumnData::DateTime(
                v.into_iter()
                    .map(|x| x.map(|serial| ((serial - 25569.0) * 86_400.0).round() as i64))
                    .collect(),
            ),
            RawData::BinarySkipped => continue,
        };
        columns.push(Column {
            name: col.name,
            data,
        });
    }
    Dataset::new(raw.name, 1, columns).map_err(LegacyError::Engine)
}

/// The accumulated EPTB batches of one legacy file.
#[wasm_bindgen]
pub struct LegacyHandle {
    source: BatchSource,
}

/// `legacyNew()` — an empty batch source.
#[wasm_bindgen(js_name = legacyNew)]
pub fn legacy_new() -> LegacyHandle {
    LegacyHandle {
        source: BatchSource::new(),
    }
}

/// `legacyPush(h, table, batch)` — append one EPTB batch for `table`
/// (chunked pushes accumulate in order).
#[wasm_bindgen(js_name = legacyPush)]
pub fn legacy_push(h: &mut LegacyHandle, table: &str, batch: &[u8]) -> Result<(), JsValue> {
    h.source.push(table, batch).map_err(|e| js_error(&e))
}

/// `legacyTables(h)` — every table pushed into the source, in file
/// order. The plain-table offer uses this after `legacyMap` has
/// reported `legacy_layout_unknown`.
#[wasm_bindgen(js_name = legacyTables)]
pub fn legacy_tables(h: &LegacyHandle) -> Result<JsValue, JsValue> {
    let out = js_sys::Array::new();
    for table in h.source.tables() {
        out.push(&JsValue::from_str(&table));
    }
    Ok(out.into())
}

/// `legacyPlainTables(h, names) -> { datasets, snapshots }` — decode
/// the chosen ordinary tables as plain datasets. `datasets` carries
/// `{name, records, hash}` for the commit metadata; `snapshots[i]`
/// encodes `datasets[i]`.
#[wasm_bindgen(js_name = legacyPlainTables)]
pub fn legacy_plain_tables(h: &mut LegacyHandle, names: js_sys::Array) -> Result<JsValue, JsValue> {
    let mut wanted = Vec::new();
    for i in 0..names.length() {
        let name = names.get(i).as_string().ok_or_else(|| {
            js_error(&js_error_reason(
                "plain table names must be strings".to_string(),
            ))
        })?;
        wanted.push(name);
    }
    if wanted.is_empty() {
        return Err(js_error(&js_error_reason(
            "choose at least one table to import".to_string(),
        )));
    }
    let meta = js_sys::Array::new();
    let snapshots = js_sys::Array::new();
    for name in wanted {
        let raw = h.source.table(&name).map_err(|e| js_error(&e))?;
        let ds = plain_dataset(raw).map_err(|e| js_error(&e))?;
        let obj = js_sys::Object::new();
        let _ = Reflect::set(&obj, &JsValue::from_str("name"), &JsValue::from_str(&name));
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("records"),
            &JsValue::from_f64(ds.row_count() as f64),
        );
        let _ = Reflect::set(
            &obj,
            &JsValue::from_str("hash"),
            &JsValue::from_str(&ds.content_hash_hex()),
        );
        meta.push(&obj);
        snapshots.push(&js_sys::Uint8Array::from(&encode_snapshot(&ds)[..]));
    }
    let out = js_sys::Object::new();
    let _ = Reflect::set(&out, &JsValue::from_str("datasets"), &meta);
    let _ = Reflect::set(&out, &JsValue::from_str("snapshots"), &snapshots);
    Ok(out.into())
}

/// `legacyDetect(h) -> SourceFormat` — the golden `source_format`
/// vocabulary (`"ei7_access"`, …).
#[wasm_bindgen(js_name = legacyDetect)]
pub fn legacy_detect(h: &LegacyHandle) -> Result<String, JsValue> {
    detect(&h.source)
        .map(format_str)
        .map(str::to_string)
        .map_err(|e| js_error(&e))
}

/// `legacyMap(h, name?, format?) -> { project: LegacyProjectJson,
/// snapshots: Uint8Array[] }` — `format` is the file's true backend
/// (`"ei7_sqlite"` from a `.db`); `name` is the file-name fallback
/// EI3 needs when the file carries no project name.
#[wasm_bindgen(js_name = legacyMap)]
pub fn legacy_map(
    h: &mut LegacyHandle,
    name: Option<String>,
    format: Option<String>,
) -> Result<JsValue, JsValue> {
    let fmt = format
        .as_deref()
        .map(|f| {
            parse_format(f)
                .ok_or_else(|| js_error(&js_error_reason(format!("unknown source_format {f:?}"))))
        })
        .transpose()?;
    let (_format, project) = detect_and_map(&mut h.source, fmt, name.as_deref().unwrap_or(""))
        .map_err(|e| js_error(&e))?;
    let project_text = project_json(&project).map_err(|e| js_error(&e))?;
    let project_js = js_sys::JSON::parse(&project_text)
        .map_err(|_| js_error(&js_error_reason("project JSON parse failed".to_string())))?;
    let snapshots = js_sys::Array::new();
    for s in project_snapshots(&project).map_err(|e| js_error(&e))? {
        snapshots.push(&js_sys::Uint8Array::from(&s[..]));
    }
    let out = js_sys::Object::new();
    let _ = Reflect::set(&out, &JsValue::from_str("project"), &project_js);
    let _ = Reflect::set(&out, &JsValue::from_str("snapshots"), &snapshots);
    Ok(out.into())
}

/// `legacyRefreshPlan(h, previous: Uint8Array[]) -> RefreshPlanJson`
/// — remaps the pushed file and diffs against the committed views
/// (see [`decode_previous`] for the packet format). The return carries
/// the plan plus the remapped project + snapshots so `commitRefresh`
/// writes new versions without a second map.
#[wasm_bindgen(js_name = legacyRefreshPlan)]
pub fn legacy_refresh_plan(
    h: &mut LegacyHandle,
    previous: js_sys::Array,
    format: Option<String>,
) -> Result<JsValue, JsValue> {
    let fmt = format
        .as_deref()
        .map(|f| {
            parse_format(f)
                .ok_or_else(|| js_error(&js_error_reason(format!("unknown source_format {f:?}"))))
        })
        .transpose()?;
    let (_format, next) = detect_and_map(&mut h.source, fmt, "").map_err(|e| js_error(&e))?;
    let packets: Vec<Vec<u8>> = (0..previous.length())
        .map(|i| js_sys::Uint8Array::from(previous.get(i)).to_vec())
        .collect();
    let prev = decode_previous(&packets).map_err(|e| js_error(&e))?;
    let plan = refresh_plan(&prev, &next).map_err(|e| js_error(&e))?;
    let plan_js = js_sys::JSON::parse(&refresh_plan_json(&plan).map_err(|e| js_error(&e))?)
        .map_err(|_| {
            js_error(&js_error_reason(
                "refresh plan JSON parse failed".to_string(),
            ))
        })?;
    let project_js = js_sys::JSON::parse(&project_json(&next).map_err(|e| js_error(&e))?)
        .map_err(|_| js_error(&js_error_reason("project JSON parse failed".to_string())))?;
    let snapshots = js_sys::Array::new();
    for s in project_snapshots(&next).map_err(|e| js_error(&e))? {
        snapshots.push(&js_sys::Uint8Array::from(&s[..]));
    }
    let out = js_sys::Object::new();
    let _ = Reflect::set(&out, &JsValue::from_str("plan"), &plan_js);
    let _ = Reflect::set(&out, &JsValue::from_str("project"), &project_js);
    let _ = Reflect::set(&out, &JsValue::from_str("snapshots"), &snapshots);
    Ok(out.into())
}

/// `RefreshPlanJson`'s plan member — `{"views": [...]}`.
pub(crate) fn refresh_plan_json(plan: &epiora_legacy::RefreshPlan) -> Result<String, LegacyError> {
    serde_json::to_string(plan).map_err(|e| LegacyError::Source {
        code: "legacy_view_broken",
        reason: format!("refresh plan JSON failed: {e}"),
    })
}
