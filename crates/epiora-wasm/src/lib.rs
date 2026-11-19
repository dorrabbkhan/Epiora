//! epiora-wasm: the browser binding for `epiora-core` (spec §5.1).
//! Type conversion only — no statistics, no branching on data.

mod columns;
#[cfg(feature = "parity")]
mod parity;

use epiora_core::dataset::Dataset;
use epiora_core::procedures::{
    freq as freq_core, means as means_core, tables as tables_core, FreqOptions, MeansOptions,
    ProcOutput, RunContext, TablesOptions,
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

/// `means` (v1.0 spec §4.2); returns `{ result, receipt, notes }` as
/// a plain object.
#[wasm_bindgen(js_name = means)]
pub fn means(
    handle: &DatasetHandle,
    options_value: JsValue,
    ctx_value: JsValue,
) -> Result<JsValue, JsValue> {
    let opts: MeansOptions = options(options_value)?;
    let ctx: RunContext = options(ctx_value)?;
    let out = means_core(&handle.dataset, &opts, &ctx).map_err(|e| js_error(&e))?;
    output(&out)
}

/// The calculator registry (`calc::REGISTRY`) as plain
/// `CalculatorDef[]` data (v1.0 spec §5.1, §5.3). Serialized through
/// JSON rather than serde_wasm_bindgen: `Integer` bounds use
/// `i64::MAX` for "unbounded", which has no JS `number` value —
/// `JSON.parse` maps it to the nearest f64 the way every engine
/// output crossing this boundary already does.
#[wasm_bindgen(js_name = calcRegistry)]
pub fn calc_registry() -> Result<JsValue, JsValue> {
    let text = serde_json::to_string(epiora_core::calc::REGISTRY)
        .map_err(|e| invalid_options(e.to_string()))?;
    js_sys::JSON::parse(&text).map_err(|e| invalid_options(format!("{e:?}")))
}

/// The validation registry (`validation::REGISTRY`) as plain
/// `statistics[]` entries in the `validation-report --format json`
/// shape (v1.0 spec §7.4): `id`, `method`, `class` and `cases` (the
/// covering fixture count). The `/validation` page uses it as the
/// fallback when no `validation.json` was bundled — `references` and
/// `worst_error` are absent there because nothing measured the build.
#[wasm_bindgen(js_name = validationRegistry)]
pub fn validation_registry() -> Result<JsValue, JsValue> {
    let entries: Vec<serde_json::Value> = epiora_core::validation::REGISTRY
        .iter()
        .map(|c| {
            serde_json::json!({
                "id": c.statistic,
                "method": epiora_core::validation::method(c.statistic),
                "class": c.class,
                "cases": c.fixtures.len(),
            })
        })
        .collect();
    let text = serde_json::to_string(&entries).map_err(|e| invalid_options(e.to_string()))?;
    js_sys::JSON::parse(&text).map_err(|e| invalid_options(format!("{e:?}")))
}

/// Runs calculator `id` on `inputs`; returns
/// `{ result, receipt, notes }` as a plain object (v1.0 spec §5.3,
/// receipt v3 of §5.4).
#[wasm_bindgen(js_name = calcRun)]
pub fn calc_run(id: &str, inputs_value: JsValue, ctx_value: JsValue) -> Result<JsValue, JsValue> {
    let inputs: serde_json::Value = options(inputs_value)?;
    let ctx: RunContext = options(ctx_value)?;
    let out = epiora_core::calc::run(id, &inputs, &ctx).map_err(|e| js_error(&e))?;
    serde_json::json!({
        "result": out.result,
        "receipt": out.receipt,
        "notes": out.notes,
    })
    .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
    .map_err(|e| invalid_options(e.to_string()))
}

/// The explain trace of one procedure statistic (v1.0 spec §6):
/// `procedure` is `"freq"`, `"tables"` or `"means"`, `options` its
/// options object, `statistic` the registry id with concrete
/// indices.
#[wasm_bindgen(js_name = explainProcedure)]
pub fn explain_procedure(
    handle: &DatasetHandle,
    procedure: &str,
    options_value: JsValue,
    statistic: &str,
) -> Result<JsValue, JsValue> {
    let opts: serde_json::Value = options(options_value)?;
    let out = epiora_core::explain::explain_procedure(&handle.dataset, procedure, &opts, statistic)
        .map_err(|e| js_error(&e))?;
    out.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| invalid_options(e.to_string()))
}

/// The explain trace of one calculator statistic (v1.0 spec §6):
/// `id` is the calculator's registry id, `inputs` its inputs object,
/// `statistic` the `calc.<id>.<path>` id with concrete indices.
#[wasm_bindgen(js_name = explainCalculator)]
pub fn explain_calculator(
    id: &str,
    inputs_value: JsValue,
    statistic: &str,
) -> Result<JsValue, JsValue> {
    let inputs: serde_json::Value = options(inputs_value)?;
    let out = epiora_core::explain::explain_calculator(id, &inputs, statistic)
        .map_err(|e| js_error(&e))?;
    out.serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| invalid_options(e.to_string()))
}

/// The Epi Info Classic Analysis command of `procedure`
/// (`"freq"`, `"tables"` or `"means"`) under its canonical options —
/// display only (v1.0 spec §7.3/G4).
#[wasm_bindgen(js_name = epiInfoCommand)]
pub fn epi_info_command(procedure: &str, options_value: JsValue) -> Result<String, JsValue> {
    let opts: serde_json::Value = options(options_value)?;
    epiora_core::export::epi_info_command(procedure, &opts).map_err(|e| js_error(&e))
}

/// An `RExport` as `{ script, dataCsv }` — `dataCsv` a `Uint8Array`
/// for dataset runs, `null` for calculators (their inputs embed in
/// the script).
fn r_export_object(export: &epiora_core::export::RExport) -> Result<JsValue, JsValue> {
    let obj = js_sys::Object::new();
    Reflect::set(
        &obj,
        &JsValue::from_str("script"),
        &JsValue::from_str(&export.script),
    )
    .map_err(|e| invalid_options(format!("{e:?}")))?;
    let csv = match &export.data_csv {
        Some(bytes) => js_sys::Uint8Array::from(bytes.as_slice()).into(),
        None => JsValue::NULL,
    };
    Reflect::set(&obj, &JsValue::from_str("dataCsv"), &csv)
        .map_err(|e| invalid_options(format!("{e:?}")))?;
    Ok(obj.into())
}

/// The R export of one procedure run (v1.0 spec §8.2): `procedure` is
/// `"freq"`, `"tables"` or `"means"`, `options` the canonical options
/// the run took, `receiptSha` the run's sealed receipt hash.
#[wasm_bindgen(js_name = exportR)]
pub fn export_r(
    handle: &DatasetHandle,
    procedure: &str,
    options_value: JsValue,
    receipt_sha: &str,
) -> Result<JsValue, JsValue> {
    let opts: serde_json::Value = options(options_value)?;
    let export =
        epiora_core::export::export_r_procedure(&handle.dataset, procedure, &opts, receipt_sha)
            .map_err(|e| js_error(&e))?;
    r_export_object(&export)
}

/// The R export of one calculator run (v1.0 spec §8.2): `id` is the
/// calculator's registry id, `inputs` the canonical inputs — the
/// script embeds them, so `dataCsv` is always `null`.
#[wasm_bindgen(js_name = exportRCalc)]
pub fn export_r_calc(
    id: &str,
    inputs_value: JsValue,
    receipt_sha: &str,
) -> Result<JsValue, JsValue> {
    let inputs: serde_json::Value = options(inputs_value)?;
    let export = epiora_core::export::export_r_calculator(id, &inputs, receipt_sha)
        .map_err(|e| js_error(&e))?;
    r_export_object(&export)
}

/// The engine version stamped on receipts (spec §5.1).
#[wasm_bindgen(js_name = engineVersion)]
pub fn engine_version() -> String {
    EngineStamp::current().version
}
