//! epiora-wasm: the browser binding for `epiora-core` (spec §5.1).
//! Type conversion only — no statistics, no branching on data.

mod columns;
#[cfg(feature = "parity")]
mod parity;

use epiora_core::dataset::Dataset;
use epiora_core::procedures::{
    freq as freq_core, tables as tables_core, FreqOptions, ProcOutput, RunContext, TablesOptions,
};
use epiora_core::receipt::EngineStamp;
use epiora_core::EngineError;
use js_sys::{Error, Reflect};
use serde::de::DeserializeOwned;
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// A validated dataset living inside the wasm instance; the handle the
/// other exports take (spec §5.1).
#[wasm_bindgen]
pub struct DatasetHandle {
    dataset: Dataset,
}

/// An `EngineError` crossing the boundary becomes a JS `Error` whose
/// `code` is the engine's stable code and whose `message` is its
/// `Display` (spec §5.1).
pub(crate) fn js_error(e: &EngineError) -> JsValue {
    let err = Error::new(&e.to_string());
    let _ = Reflect::set(
        &err,
        &JsValue::from_str("code"),
        &JsValue::from_str(e.code()),
    );
    err.into()
}

fn invalid_options(reason: impl Into<String>) -> JsValue {
    js_error(&EngineError::InvalidOptions {
        reason: reason.into(),
    })
}

fn options<T: DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value).map_err(|e| invalid_options(e.to_string()))
}

fn output<T: Serialize>(out: &ProcOutput<T>) -> Result<JsValue, JsValue> {
    out.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| invalid_options(e.to_string()))
}

fn check_version(version: f64) -> Result<u64, JsValue> {
    if version.is_finite() && version.fract() == 0.0 && version >= 0.0 && version <= u64::MAX as f64
    {
        Ok(version as u64)
    } else {
        Err(invalid_options("version must be a non-negative integer"))
    }
}

/// Builds a dataset from typed-array columns (spec §5.1 encoding,
/// decoded in `columns.rs`).
#[wasm_bindgen(js_name = datasetFromColumns)]
pub fn dataset_from_columns(
    id: &str,
    version: f64,
    columns: &js_sys::Array,
) -> Result<DatasetHandle, JsValue> {
    let version = check_version(version)?;
    let columns = columns::convert(columns).map_err(|e| js_error(&e))?;
    Dataset::new(id, version, columns)
        .map(|dataset| DatasetHandle { dataset })
        .map_err(|e| js_error(&e))
}

/// Decodes columnar snapshot bytes into a dataset handle (v0.3 spec
/// §5.1).
#[wasm_bindgen(js_name = datasetFromSnapshot)]
pub fn dataset_from_snapshot(
    id: &str,
    version: f64,
    bytes: &[u8],
) -> Result<DatasetHandle, JsValue> {
    let version = check_version(version)?;
    epiora_core::snapshot::decode_snapshot(id, version, bytes)
        .map(|dataset| DatasetHandle { dataset })
        .map_err(|e| js_error(&e))
}

/// Encodes the dataset into columnar snapshot bytes (v0.3 spec §5.1).
#[wasm_bindgen(js_name = datasetToSnapshot)]
pub fn dataset_to_snapshot(handle: &DatasetHandle) -> Vec<u8> {
    epiora_core::snapshot::encode_snapshot(&handle.dataset)
}

/// The dataset's SHA-256 content hash (`Dataset::content_hash_hex`).
#[wasm_bindgen(js_name = datasetHash)]
pub fn dataset_hash(handle: &DatasetHandle) -> String {
    handle.dataset.content_hash_hex()
}

/// `freq` (spec §4.1); returns `{ result, receipt }` as a plain object.
#[wasm_bindgen(js_name = freq)]
pub fn freq(
    handle: &DatasetHandle,
    options_value: JsValue,
    ctx_value: JsValue,
) -> Result<JsValue, JsValue> {
    let opts: FreqOptions = options(options_value)?;
    let ctx: RunContext = options(ctx_value)?;
    let out = freq_core(&handle.dataset, &opts, &ctx).map_err(|e| js_error(&e))?;
    output(&out)
}

/// `tables` (spec §4.1); returns `{ result, receipt }` as a plain
/// object.
#[wasm_bindgen(js_name = tables)]
pub fn tables(
    handle: &DatasetHandle,
    options_value: JsValue,
    ctx_value: JsValue,
) -> Result<JsValue, JsValue> {
    let opts: TablesOptions = options(options_value)?;
    let ctx: RunContext = options(ctx_value)?;
    let out = tables_core(&handle.dataset, &opts, &ctx).map_err(|e| js_error(&e))?;
    output(&out)
}

/// The engine version stamped on receipts (spec §5.1).
#[wasm_bindgen(js_name = engineVersion)]
pub fn engine_version() -> String {
    EngineStamp::current().version
}
