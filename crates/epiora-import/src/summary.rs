//! The `importFile().summary()` shape and the canonical parity line
//! (plan Task 6): one `Imported` reduces to detection, effective
//! settings, per-column coercions, row count, source SHA-256 and the
//! `[name, type]` schema; [`parity_line`] adds the snapshot hashes and
//! is the exact line `import-runner` and the wasm parity run print for
//! each `fixtures/import` case.

use crate::error::ImportError;
use crate::sha256_hex;
use crate::{Detection, ImportSettings, Imported};
use epiora_core::dataset::{Dataset, ValueType};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeMap;

/// What `ImportedHandle.summary()` reports (plan Task 6): the audit
/// trail plus the `[name, type]` schema for `datasetFromRecords`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct ImportSummary {
    /// The effective reading of the file's format.
    pub detection: Detection,
    /// Effective settings (detected values fill unset overrides).
    pub settings: ImportSettings,
    /// Per-column counts of values coerced to missing.
    pub coercions: BTreeMap<String, u64>,
    /// Data row count.
    pub row_count: u64,
    /// Lowercase hex SHA-256 of the input bytes.
    pub source_sha256: String,
    /// Column names and types in file order.
    pub schema: Vec<(String, ValueType)>,
}

/// Reduces an [`Imported`] to its audit summary (plan Task 6).
pub fn summary(imported: &Imported) -> ImportSummary {
    ImportSummary {
        detection: imported.detection.clone(),
        settings: imported.settings.clone(),
        coercions: imported.coercions.clone(),
        row_count: imported.row_count,
        source_sha256: imported.source_sha256.clone(),
        schema: imported
            .columns
            .iter()
            .map(|c| (c.name.clone(), c.data.value_type()))
            .collect(),
    }
}

/// One page of a dataset's records (`recordBatch`/`addedBatch`): the
/// record ids and their canonical record JSON strings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
pub struct RecordBatch {
    /// Record ids in dataset row order.
    pub ids: Vec<String>,
    /// Canonical record JSON, one per id.
    pub json: Vec<String>,
}

/// The canonical parity line for one `fixtures/import` case (spec
/// §6.5): a single-line JSON document `import-runner` and the wasm
/// `runImportCase` both print — case id, the [`summary`] on success or
/// the stable error code and message on failure, and on success the
/// dataset content hash plus the SHA-256 of the snapshot bytes, so the
/// native/wasm diff proves bit-identical snapshots.
pub fn parity_line(case: &str, outcome: &Result<Imported, ImportError>) -> String {
    let line = match outcome {
        Ok(imported) => {
            let mut object = json!({
                "case": case,
                "summary": summary(imported),
            });
            match Dataset::new(case, 1, imported.columns.clone()) {
                Ok(dataset) => {
                    let snapshot = epiora_core::snapshot::encode_snapshot(&dataset);
                    object["snapshot_hash"] = json!(dataset.content_hash_hex());
                    object["snapshot_sha256"] = json!(sha256_hex(&snapshot));
                }
                Err(e) => {
                    object["snapshot_error"] = json!(e.code());
                }
            }
            object
        }
        Err(e) => json!({
            "case": case,
            "error": { "code": e.code(), "message": e.to_string() },
        }),
    };
    // `serde_json::Map` orders keys by UTF-8 bytes; serialising one is
    // infallible.
    serde_json::to_string(&line).unwrap_or_else(|_| "{}".to_string())
}
