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
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct DatasetStamp {
    /// Dataset id.
    pub id: String,
    /// Dataset version.
    pub version: u64,
    /// `Dataset::content_hash_hex`.
    pub sha256: String,
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
    /// The input dataset stamp.
    pub dataset: DatasetStamp,
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
    ) -> Receipt {
        let mut receipt = Receipt {
            receipt_version: if project.is_some() {
                RECEIPT_VERSION_WITH_PROJECT
            } else {
                RECEIPT_VERSION
            },
            project,
            dataset,
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
        let digest = sha2::Sha256::digest(canonical_json(&value).as_bytes());
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            hex.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
            hex.push(char::from_digit((byte & 0xf) as u32, 16).unwrap_or('0'));
        }
        hex
    }
}
