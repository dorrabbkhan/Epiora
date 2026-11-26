// Plain-table decoding for unknown-layout legacy files (Review Focus
// 2). Kept out of `lib.rs` so the wasm binding root stays within the
// repository file-size rule; type conversion only, no mapping logic.

use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::snapshot::encode_snapshot;
use epiora_legacy::batch::{RawData, RawTable};
use epiora_legacy::{LegacyError, LegacySource};
use js_sys::Reflect;
use wasm_bindgen::prelude::*;

use crate::{js_error, js_error_reason, LegacyHandle};

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
