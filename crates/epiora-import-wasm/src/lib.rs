//! epiora-import-wasm: the browser binding for `epiora-import` (v0.3
//! spec §5.3, §6). Type conversion only — no parsing logic.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

#[cfg(feature = "parity")]
mod parity;

use epiora_core::dataset::{Dataset, ValueType};
use epiora_import::{
    dataset_from_records, diff as diff_records, key_candidates, record_ids, record_json, summarize,
    Diff, ImportError, ImportSettings, Imported, RecordBatch,
};
use js_sys::{Error, Reflect};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::cell::OnceCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// An `ImportError` crossing the boundary becomes a JS `Error` whose
/// `code` is the importer's stable code and whose `message` is its
/// `Display`; variants that carry a 1-based data `line` surface it too
/// (spec §11).
fn js_error(e: &ImportError) -> JsValue {
    let err = Error::new(&e.to_string());
    let _ = Reflect::set(
        &err,
        &JsValue::from_str("code"),
        &JsValue::from_str(e.code()),
    );
    let line = match e {
        ImportError::RaggedRow { line, .. }
        | ImportError::MissingKey { line }
        | ImportError::DuplicateKey { line, .. } => Some(*line),
        _ => None,
    };
    if let Some(line) = line {
        let _ = Reflect::set(
            &err,
            &JsValue::from_str("line"),
            &JsValue::from_f64(line as f64),
        );
    }
    err.into()
}

fn invalid_settings(reason: impl Into<String>) -> JsValue {
    js_error(&ImportError::InvalidSettings {
        reason: reason.into(),
    })
}

fn options<T: DeserializeOwned>(value: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(value).map_err(|e| invalid_settings(e.to_string()))
}

fn output<T: Serialize + ?Sized>(value: &T) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| invalid_settings(e.to_string()))
}

fn settings(value: JsValue) -> Result<ImportSettings, JsValue> {
    if value.is_undefined() || value.is_null() {
        return Ok(ImportSettings::default());
    }
    options(value)
}

fn check_version(version: f64) -> Result<u64, JsValue> {
    if version.is_finite() && version.fract() == 0.0 && version >= 0.0 && version <= u64::MAX as f64
    {
        Ok(version as u64)
    } else {
        Err(invalid_settings("version must be a non-negative integer"))
    }
}

/// A `[start, len)` page request as slice bounds `[start, end)` into a
/// vector of `total` rows; a `start` past the end pages to empty.
fn page(start: f64, len: f64, total: usize) -> Result<(usize, usize), JsValue> {
    for (name, v) in [("start", start), ("len", len)] {
        if !v.is_finite() || v.fract() != 0.0 || v < 0.0 || v > usize::MAX as f64 {
            return Err(invalid_settings(format!(
                "{name} must be a non-negative integer"
            )));
        }
    }
    let start = (start as usize).min(total);
    let end = start.saturating_add(len as usize).min(total);
    Ok((start, end))
}

/// The imported columns as a validated `Dataset`; the caller chooses
/// the id and version stamped on it.
fn dataset(
    imported: &Imported,
    id: impl Into<String>,
    version: u64,
) -> Result<Dataset, ImportError> {
    Ok(Dataset::new(id, version, imported.columns.clone())?)
}

/// `inspect(bytes, fileName)` — detection, effective settings and a
/// typed preview under default settings (spec §6.1–6.2).
#[wasm_bindgen(js_name = inspect)]
pub fn wasm_inspect(bytes: &[u8], file_name: &str) -> Result<JsValue, JsValue> {
    output(&epiora_import::inspect(bytes, file_name).map_err(|e| js_error(&e))?)
}

/// `inspectWith(bytes, fileName, settings)` — `inspect` with explicit
/// settings; detection fills unset overrides.
#[wasm_bindgen(js_name = inspectWith)]
pub fn inspect_with(
    bytes: &[u8],
    file_name: &str,
    settings_value: JsValue,
) -> Result<JsValue, JsValue> {
    let s = settings(settings_value)?;
    output(&epiora_import::inspect_with(bytes, file_name, &s).map_err(|e| js_error(&e))?)
}

/// `importFile(bytes, fileName, settings)` — a full import; the handle
/// pages out records, emits a snapshot and diffs against a previous
/// snapshot.
#[wasm_bindgen(js_name = importFile)]
pub fn import_file(
    bytes: &[u8],
    file_name: &str,
    settings_value: JsValue,
) -> Result<ImportedHandle, JsValue> {
    let s = settings(settings_value)?;
    epiora_import::import(bytes, file_name, &s)
        .map(|imported| ImportedHandle {
            imported: Rc::new(imported),
            record_ids: OnceCell::new(),
        })
        .map_err(|e| js_error(&e))
}

/// A completed import living inside the wasm instance; the handle the
/// other exports take (plan Task 6). The import sits behind an `Rc` so
/// `diffAgainst` shares it instead of copying every column, and record
/// ids are computed once — `recordBatch` pages over them without an
/// O(n) re-derivation per call.
#[wasm_bindgen]
pub struct ImportedHandle {
    imported: Rc<Imported>,
    record_ids: OnceCell<Result<Vec<String>, ImportError>>,
}

#[wasm_bindgen]
impl ImportedHandle {
    /// `rowCount()` — the data row count.
    #[wasm_bindgen(js_name = rowCount)]
    pub fn row_count(&self) -> f64 {
        self.imported.row_count as f64
    }

    /// `summary()` — `{ detection, settings, coercions, rowCount,
    /// sourceSha256, schema: [name, type][] }` (plan Task 6).
    #[wasm_bindgen(js_name = summary)]
    pub fn summary(&self) -> Result<JsValue, JsValue> {
        output(&epiora_import::summary(&self.imported))
    }

    /// `toSnapshot(datasetId, version)` — the columnar snapshot bytes
    /// (`epiora_core::encode_snapshot`).
    #[wasm_bindgen(js_name = toSnapshot)]
    pub fn to_snapshot(&self, dataset_id: &str, version: f64) -> Result<Vec<u8>, JsValue> {
        let version = check_version(version)?;
        let ds = dataset(&self.imported, dataset_id, version).map_err(|e| js_error(&e))?;
        Ok(epiora_core::snapshot::encode_snapshot(&ds))
    }

    /// `contentHash()` — the dataset content hash; id and version are
    /// not hashed (spec §3.3), so `""`/`0` stand in.
    #[wasm_bindgen(js_name = contentHash)]
    pub fn content_hash(&self) -> Result<String, JsValue> {
        dataset(&self.imported, "", 0)
            .map(|ds| ds.content_hash_hex())
            .map_err(|e| js_error(&e))
    }

    /// `recordBatch(start, len)` — `{ ids, json }` for the page; record
    /// ids come from the import's key setting (spec §4.4) and are
    /// derived once per handle.
    #[wasm_bindgen(js_name = recordBatch)]
    pub fn record_batch(&self, start: f64, len: f64) -> Result<JsValue, JsValue> {
        let columns = &self.imported.columns;
        let ids = match self
            .record_ids
            .get_or_init(|| record_ids(columns, &self.imported.settings.key))
        {
            Ok(ids) => ids,
            Err(e) => return Err(js_error(e)),
        };
        let (lo, hi) = page(start, len, ids.len())?;
        output(&RecordBatch {
            ids: ids[lo..hi].to_vec(),
            json: (lo..hi).map(|r| record_json(columns, r)).collect(),
        })
    }

    /// `keyCandidates()` — per-column key usability for the wizard.
    #[wasm_bindgen(js_name = keyCandidates)]
    pub fn key_candidates(&self) -> Result<JsValue, JsValue> {
        output(&key_candidates(&self.imported.columns))
    }
}

/// `diffAgainst(previousSnapshot, imported)` — the re-import diff
/// preview (spec §6.4); the key is the import's `settings.key`.
#[wasm_bindgen(js_name = diffAgainst)]
pub fn diff_against(
    previous_snapshot: &[u8],
    imported: &ImportedHandle,
) -> Result<DiffHandle, JsValue> {
    let previous = epiora_core::snapshot::decode_snapshot("previous", 0, previous_snapshot)
        .map_err(|e| js_error(&ImportError::Engine(e)))?;
    let key = &imported.imported.settings.key;
    let next_ids = record_ids(&imported.imported.columns, key).map_err(|e| js_error(&e))?;
    let diff = diff_records(&previous, &imported.imported, key).map_err(|e| js_error(&e))?;
    Ok(DiffHandle {
        diff,
        next: Rc::clone(&imported.imported),
        next_ids,
    })
}

/// A computed re-import diff; the handle the batch exports take. `next`
/// is shared with the `ImportedHandle`, not copied.
#[wasm_bindgen]
pub struct DiffHandle {
    diff: Diff,
    /// `next` (renders `addedBatch`'s record JSON).
    next: Rc<Imported>,
    /// `next`'s record ids in row order.
    next_ids: Vec<String>,
}

#[wasm_bindgen]
impl DiffHandle {
    /// `summary()` — counts, schema changes and ≤ 20 samples per field.
    #[wasm_bindgen(js_name = summary)]
    pub fn summary(&self) -> Result<JsValue, JsValue> {
        output(&summarize(&self.diff))
    }

    /// `addedBatch(start, len)` — `{ ids, json }` for a page of the
    /// added records (records come from `next`).
    #[wasm_bindgen(js_name = addedBatch)]
    pub fn added_batch(&self, start: f64, len: f64) -> Result<JsValue, JsValue> {
        let (lo, hi) = page(start, len, self.diff.added.len())?;
        output(&RecordBatch {
            ids: self.diff.added[lo..hi]
                .iter()
                .map(|&j| self.next_ids[j as usize].clone())
                .collect(),
            json: self.diff.added[lo..hi]
                .iter()
                .map(|&j| record_json(&self.next.columns, j as usize))
                .collect(),
        })
    }

    /// `changedBatch(start, len)` — a page of the `FieldChange`s.
    #[wasm_bindgen(js_name = changedBatch)]
    pub fn changed_batch(&self, start: f64, len: f64) -> Result<JsValue, JsValue> {
        let (lo, hi) = page(start, len, self.diff.changed.len())?;
        output(&self.diff.changed[lo..hi])
    }

    /// `removed()` — the removed record ids, in `previous` order.
    #[wasm_bindgen(js_name = removed)]
    pub fn removed(&self) -> Result<JsValue, JsValue> {
        output(&self.diff.removed)
    }
}

/// `datasetFromRecords(id, version, schema, rows)` — rebuilds a dataset
/// from stored record JSON (spec §4.3) and returns its snapshot bytes.
#[wasm_bindgen(js_name = datasetFromRecords)]
pub fn dataset_from_records_wasm(
    id: &str,
    version: f64,
    schema: JsValue,
    rows: Vec<String>,
) -> Result<Vec<u8>, JsValue> {
    let version = check_version(version)?;
    let schema: Vec<(String, ValueType)> = options(schema)?;
    let row_refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    dataset_from_records(id, version, &schema, &row_refs)
        .map(|ds| epiora_core::snapshot::encode_snapshot(&ds))
        .map_err(|e| js_error(&e))
}

#[cfg(all(test, feature = "ts"))]
mod export {
    use ts_rs::{Config, TS};

    /// Regenerates `crates/epiora-import-wasm/bindings/` (`cargo test
    /// -p epiora-import-wasm --features ts export_bindings`). Unlike
    /// `epiora-core`'s `#[ts(export)]` attributes — which would resolve
    /// the global `TS_RS_EXPORT_DIR` to the core bindings dir — this
    /// pins the output directory explicitly. `from_env` still honours
    /// `TS_RS_LARGE_INT = "number"`.
    #[test]
    fn export_bindings() {
        let cfg = Config::from_env().with_out_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/bindings"));
        macro_rules! export {
            ($($t:ty),+ $(,)?) => {
                $( <$t as TS>::export_all(&cfg).expect(concat!("export ", stringify!($t))); )+
            };
        }
        export!(
            epiora_import::Container,
            epiora_import::Encoding,
            epiora_import::DecimalSep,
            epiora_import::DateFormat,
            epiora_import::RecordKey,
            epiora_import::ImportSettings,
            epiora_import::Detection,
            epiora_import::PreviewColumn,
            epiora_import::Inspection,
            epiora_import::KeyCandidate,
            epiora_import::Diff,
            epiora_import::FieldChange,
            epiora_import::SchemaChanges,
            epiora_import::TypeChange,
            epiora_import::DiffSummary,
            epiora_import::ImportSummary,
            epiora_import::RecordBatch,
        );
    }
}
