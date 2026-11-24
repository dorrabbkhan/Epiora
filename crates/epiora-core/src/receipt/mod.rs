//! `receipt` — receipt v1 (spec §4.3): every result carries the
//! dataset stamp, the canonical options, the engine stamp, the
//! validation coverage of its emitted statistics, the per-variable
//! exclusion counts, the caller's timestamp and the receipt's own
//! SHA-256 over the canonical JSON.

mod canonical;

pub use canonical::canonical_json;

use crate::validation::ToleranceClass;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::Digest;
use std::collections::BTreeMap;

/// Receipt schema version.
pub const RECEIPT_VERSION: u64 = 1;
/// Receipt schema version when a project stamp is present (v0.3 spec
/// §8.2).
pub const RECEIPT_VERSION_WITH_PROJECT: u64 = 2;
/// Receipt schema version for calculator runs (v1.0 spec §5.4): no
/// dataset, a `calculator` stamp instead.
pub const RECEIPT_VERSION_CALCULATOR: u64 = 3;
/// Receipt schema version when a working-view stamp is present (v1.1
/// spec §5.2): the dataset the procedure ran on came out of a recipe
/// replay, and the stamp records exactly which recipe made it.
pub const RECEIPT_VERSION_WORKING_VIEW: u64 = 4;

/// `project` — which project the run belongs to (v0.3 spec §8.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ProjectStamp {
    /// Stable project id.
    pub id: String,
    /// Display name.
    pub name: String,
}

/// `dataset` — which input produced the result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DatasetStamp {
    /// Dataset id.
    pub id: String,
    /// Dataset version.
    pub version: u64,
    /// `Dataset::content_hash_hex`.
    pub sha256: String,
}

/// `working_view` — the recipe replay that produced the procedure's
/// input (v1.1 spec §5.2). `source` stamps the raw table the recipe
/// started from; `recipe_text` is the recipe's canonical text,
/// `recipe_sha256` its hash and `view_sha256` the materialized view's
/// content hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct WorkingViewStamp {
    /// Stamp of the recipe's source dataset.
    pub source: DatasetStamp,
    /// `Recipe::canonical_text`.
    pub recipe_text: String,
    /// `Recipe::sha256_hex`.
    pub recipe_sha256: String,
    /// `WorkingView::materialize().content_hash_hex()`.
    pub view_sha256: String,
}

/// `calculator` — which calculator produced the result (v1.0 spec
/// §5.4): its registry id and canonical inputs.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct CalculatorStamp {
    /// Registry id of the calculator (e.g. `proportion`).
    pub id: String,
    /// Canonical inputs, every default filled in.
    pub inputs: Value,
}

/// `engine` — fixed at build time; the engine reads no environment at
/// run time (spec §4.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct EngineStamp {
    /// `"epiora-core"`.
    pub name: String,
    /// `CARGO_PKG_VERSION`.
    pub version: String,
    /// `GIT_SHA` captured by `build.rs`, or `"unknown"`.
    pub git_sha: String,
}

impl EngineStamp {
    /// The stamp of this build.
    pub fn current() -> EngineStamp {
        EngineStamp {
            name: "epiora-core".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            git_sha: env!("EPIORA_GIT_SHA").to_string(),
        }
    }
}

/// One `validation` entry: the registry coverage of an emitted
/// statistic.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct ValidationEntry {
    /// Family-prefixed statistic id, e.g.
    /// `two_by_two.odds_ratio.ci_95_fisher_exact.lower`.
    pub statistic: String,
    /// Corpus fixture ids covering it.
    pub fixtures: Vec<String>,
    /// Its tolerance class.
    pub tolerance_class: ToleranceClass,
    /// The class's tolerance.
    pub tolerance: f64,
}

/// Receipt v1/v2 (spec §4.3, v0.3 spec §8.2).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Receipt {
    /// 1, or 2 when `project` is `Some`.
    pub receipt_version: u64,
    /// The project the run belongs to; omitted for project-less runs
    /// (corpus, parity, goldens), which stay exact receipt v1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<ProjectStamp>,
    /// The input dataset stamp; omitted for calculator runs (v3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset: Option<DatasetStamp>,
    /// The calculator stamp; present only on v3 receipts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calculator: Option<CalculatorStamp>,
    /// The working-view stamp; present only on v4 receipts (v1.1 spec
    /// §5.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_view: Option<WorkingViewStamp>,
    /// `"freq"` or `"tables"`.
    pub procedure: String,
    /// Canonical options, every default filled in.
    pub options: Value,
    /// Engine stamp.
    pub engine: EngineStamp,
    /// Registry coverage of every emitted statistic.
    pub validation: Vec<ValidationEntry>,
    /// Excluded records per used variable.
    pub exclusions: BTreeMap<String, u64>,
    /// The caller's `RunContext.timestamp`.
    pub timestamp: String,
    /// SHA-256 hex of the canonical JSON with this key omitted.
    pub receipt_sha256: String,
}

impl Receipt {
    /// Assembles a receipt and seals it with `receipt_sha256`. The
    /// parameter list is the receipt's field list; grouping it further
    /// is declined here.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dataset: DatasetStamp,
        procedure: String,
        options: Value,
        validation: Vec<ValidationEntry>,
        exclusions: BTreeMap<String, u64>,
        timestamp: String,
        engine: EngineStamp,
        project: Option<ProjectStamp>,
        working_view: Option<WorkingViewStamp>,
    ) -> Receipt {
        let mut receipt = Receipt {
            receipt_version: if working_view.is_some() {
                RECEIPT_VERSION_WORKING_VIEW
            } else if project.is_some() {
                RECEIPT_VERSION_WITH_PROJECT
            } else {
                RECEIPT_VERSION
            },
            project,
            dataset: Some(dataset),
            calculator: None,
            working_view,
            procedure,
            options,
            engine,
            validation,
            exclusions,
            timestamp,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.sha256_hex();
        receipt
    }

    /// Assembles a calculator receipt (v3): `dataset` is absent, the
    /// `calculator` stamp carries the registry id and canonical inputs.
    /// `options` are the same canonical inputs (spec §5.4).
    pub fn for_calculator(
        stamp: CalculatorStamp,
        procedure: String,
        options: Value,
        validation: Vec<ValidationEntry>,
        timestamp: String,
        engine: EngineStamp,
    ) -> Receipt {
        let mut receipt = Receipt {
            receipt_version: RECEIPT_VERSION_CALCULATOR,
            project: None,
            dataset: None,
            calculator: Some(stamp),
            working_view: None,
            procedure,
            options,
            engine,
            validation,
            exclusions: BTreeMap::new(),
            timestamp,
            receipt_sha256: String::new(),
        };
        receipt.receipt_sha256 = receipt.sha256_hex();
        receipt
    }

    /// SHA-256 hex of the canonical JSON with `receipt_sha256` omitted
    /// (spec §4.3).
    pub fn sha256_hex(&self) -> String {
        let mut value = match serde_json::to_value(self) {
            Ok(v) => v,
            Err(_) => return String::new(),
        };
        if let Value::Object(m) = &mut value {
            m.remove("receipt_sha256");
        }
        canonical_sha256_hex(&value)
    }
}

/// SHA-256 hex of a value's canonical JSON — the `receipt_sha256`
/// primitive, shared with the corpus's build-normalized parity line,
/// which reseals a receipt exactly as `Receipt::sha256_hex` does
/// (v1.0 Task 10).
pub(crate) fn canonical_sha256_hex(v: &Value) -> String {
    let digest = sha2::Sha256::digest(canonical_json(v).as_bytes());
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        hex.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
        hex.push(char::from_digit((byte & 0xf) as u32, 16).unwrap_or('0'));
    }
    hex
}
