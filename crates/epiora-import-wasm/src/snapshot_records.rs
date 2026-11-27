// Snapshot-backed canonical-record paging and the legacy-aware
// rebuild export (v1.1 Task 15 review): the worker reuses the v0.3
// records/changes write path for legacy datasets without mapping the
// source file twice.

use epiora_core::dataset::{ColumnMeta, Dataset, LabelSet, ValueType};
use epiora_import::{
    dataset_from_records_with_meta, record_ids, record_json, ImportError, RecordBatch,
};
use serde::Serialize;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

use crate::{check_version, invalid_settings, js_error, options, output, page};

/// The schema plus snapshot-v2 metadata `datasetFromRecordsWithMeta`
/// needs to rebuild a legacy dataset byte-for-byte. Label sets key the
/// map by id; `meta` stays in column order.
#[derive(Serialize)]
struct SnapshotSchema<'a> {
    schema: Vec<(String, ValueType)>,
    meta: &'a [ColumnMeta],
    label_sets: &'a BTreeMap<String, LabelSet>,
}

fn decode_records_snapshot(snapshot: &[u8]) -> Result<Dataset, JsValue> {
    epiora_core::snapshot::decode_snapshot("records", 0, snapshot)
        .map_err(|e| js_error(&ImportError::Engine(e)))
}

/// `snapshotSchema(snapshot)` — decode only the schema/meta carried in
/// a verified snapshot so the worker can persist it in the opaque
/// `datasets.settings` composite for a later rebuild.
#[wasm_bindgen(js_name = snapshotSchema)]
pub fn snapshot_schema(snapshot: &[u8]) -> Result<JsValue, JsValue> {
    let ds = decode_records_snapshot(snapshot)?;
    output(&SnapshotSchema {
        schema: ds
            .columns()
            .iter()
            .map(|c| (c.name.clone(), c.data.value_type()))
            .collect(),
        meta: ds.meta(),
        label_sets: ds.label_sets(),
    })
}

/// A decoded snapshot plus the record ids of every row. Paging record
/// JSON out of the verified bytes lets the legacy import reuse the
/// v0.3 records/changes write path without a second mapper pass.
#[wasm_bindgen]
pub struct SnapshotRecordsHandle {
    dataset: Dataset,
    ids: Vec<String>,
}

fn record_batch_output(
    dataset: &Dataset,
    ids: &[String],
    rows: impl Iterator<Item = usize>,
) -> Result<JsValue, JsValue> {
    let rows: Vec<usize> = rows.collect();
    output(&RecordBatch {
        ids: rows.iter().map(|&r| ids[r].clone()).collect(),
        json: rows
            .iter()
            .map(|&r| record_json(dataset.columns(), r))
            .collect(),
    })
}

/// `snapshotRecords(snapshot, keyColumn)` — decode a verified snapshot
/// once for canonical-record paging. An empty `keyColumn` uses row
/// position (a dataset with no legacy record key); otherwise the same
/// `RecordKey::Column` rules as the v0.3 diff apply.
#[wasm_bindgen(js_name = snapshotRecords)]
pub fn snapshot_records(
    snapshot: &[u8],
    key_column: &str,
) -> Result<SnapshotRecordsHandle, JsValue> {
    let dataset = decode_records_snapshot(snapshot)?;
    let key = if key_column.is_empty() {
        epiora_import::RecordKey::RowPosition
    } else {
        epiora_import::RecordKey::Column {
            name: key_column.to_string(),
        }
    };
    let ids = record_ids(dataset.columns(), &key).map_err(|e| js_error(&e))?;
    Ok(SnapshotRecordsHandle { dataset, ids })
}

#[wasm_bindgen]
impl SnapshotRecordsHandle {
    /// `schema()` — `[name, type]` schema, column meta and label sets
    /// of the decoded snapshot (the rebuild metadata the store persists).
    #[wasm_bindgen(js_name = schema)]
    pub fn schema(&self) -> Result<JsValue, JsValue> {
        output(&SnapshotSchema {
            schema: self
                .dataset
                .columns()
                .iter()
                .map(|c| (c.name.clone(), c.data.value_type()))
                .collect(),
            meta: self.dataset.meta(),
            label_sets: self.dataset.label_sets(),
        })
    }

    /// `recordBatch(start, len)` — `{ ids, json }` in snapshot row order.
    #[wasm_bindgen(js_name = recordBatch)]
    pub fn record_batch(&self, start: f64, len: f64) -> Result<JsValue, JsValue> {
        let (lo, hi) = page(start, len, self.ids.len())?;
        record_batch_output(&self.dataset, &self.ids, lo..hi)
    }

    /// `recordRows(rows)` — `{ ids, json }` for arbitrary snapshot row
    /// indices (the re-import diff's `added` rows).
    #[wasm_bindgen(js_name = recordRows)]
    pub fn record_rows(&self, rows_value: JsValue) -> Result<JsValue, JsValue> {
        let rows: Vec<u32> = serde_wasm_bindgen::from_value(rows_value)
            .map_err(|e| invalid_settings(e.to_string()))?;
        for &row in &rows {
            if row as usize >= self.ids.len() {
                return Err(invalid_settings(format!(
                    "record row {} is outside {} rows",
                    row + 1,
                    self.ids.len()
                )));
            }
        }
        record_batch_output(
            &self.dataset,
            &self.ids,
            rows.into_iter().map(|r| r as usize),
        )
    }
}

/// `datasetFromRecordsWithMeta(id, version, schema, rows, meta,
/// labelSets)` — the legacy-aware rebuild path: record JSON plus the
/// column meta and label sets persisted from the original snapshot.
#[wasm_bindgen(js_name = datasetFromRecordsWithMeta)]
pub fn dataset_from_records_with_meta_wasm(
    id: &str,
    version: f64,
    schema: JsValue,
    rows: Vec<String>,
    meta: JsValue,
    label_sets: JsValue,
) -> Result<Vec<u8>, JsValue> {
    let version = check_version(version)?;
    let schema: Vec<(String, ValueType)> = options(schema)?;
    let row_refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let meta: Vec<ColumnMeta> = options(meta)?;
    let label_sets: BTreeMap<String, LabelSet> = options(label_sets)?;
    dataset_from_records_with_meta(id, version, &schema, &row_refs, meta, label_sets)
        .map(|ds| epiora_core::snapshot::encode_snapshot(&ds))
        .map_err(|e| js_error(&e))
}
