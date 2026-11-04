//! `procedures` — `freq` and `tables`: tabulate → stats → receipt
//! (spec §4.1). Every result carries a [`Receipt`] and every emitted
//! statistic is registered in `validation::REGISTRY`.

mod freq;
mod tables;

pub use freq::freq;
pub use tables::tables;

use crate::dataset::{Dataset, Value};
use crate::receipt::{Receipt, ValidationEntry};
use crate::stats::{CategoryStats, RxcStats, StratifiedStats, TwoByTwoStats};
use crate::validation::{statistic_ids, REGISTRY};
use crate::EngineError;
use serde::Serialize;
use serde_json::Value as Json;
use std::collections::BTreeMap;

/// Caller-supplied context (spec §4.1). The engine reads no clock, so
/// the timestamp arrives from outside; an empty one is `InvalidOptions`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RunContext {
    /// RFC 3339 timestamp, supplied by the caller; must be non-empty.
    pub timestamp: String,
}

/// A procedure's result plus its receipt.
#[derive(Debug, Clone, Serialize)]
pub struct ProcOutput<T> {
    /// The computed result.
    pub result: T,
    /// The receipt v1 of spec §4.3.
    pub receipt: Receipt,
}

/// Which proportion CI is labelled as Epi Info's (spec §3.5). The
/// default is Clopper–Pearson until Task 9 records Epi Info 7's method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProportionCi {
    /// `binom.test`'s exact beta interval.
    #[default]
    ClopperPearson,
    /// `prop.test(correct = FALSE)` score interval.
    Wilson,
}

/// Options of `freq` (spec §4.1).
#[derive(Debug, Clone, Serialize)]
pub struct FreqOptions {
    /// Variables to tabulate.
    pub variables: Vec<String>,
    /// Optional stratify variable.
    pub stratify_by: Option<String>,
    /// Adds a `"(missing)"` category instead of excluding.
    pub include_missing: bool,
    /// Which CI is reported as the procedure's method.
    pub ci_method: ProportionCi,
}

/// One variable of a `tables` call, with optional explicit levels —
/// `[first, second]` is the orientation (exposed/ill first, spec §3.3).
#[derive(Debug, Clone, Serialize)]
pub struct VarSpec {
    /// Column name.
    pub name: String,
    /// Explicit orientation pair, or `None` for the spec's rule.
    pub levels: Option<[Value; 2]>,
}

/// Options of `tables` (spec §4.1).
#[derive(Debug, Clone, Serialize)]
pub struct TablesOptions {
    /// Row variable (exposure in a 2×2).
    pub exposure: VarSpec,
    /// Column variable (outcome in a 2×2).
    pub outcome: VarSpec,
    /// Stratify variables; combined strata in category order.
    pub stratify_by: Vec<String>,
    /// Adds `"(missing)"` categories instead of excluding.
    pub include_missing: bool,
}

/// `freq` result (spec §4.1).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FreqResult {
    /// One table per variable (per stratum when stratified).
    pub tables: Vec<FreqTable>,
    /// Excluded records per used variable.
    pub exclusions: BTreeMap<String, u64>,
}

/// One frequency table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FreqTable {
    /// The tabulated variable.
    pub variable: String,
    /// The stratum this table belongs to, when stratified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stratum: Option<String>,
    /// Category labels, `"(missing)"` last when present.
    pub labels: Vec<String>,
    /// The procedure's CI method label.
    pub ci_method: ProportionCi,
    /// Records in scope.
    pub total: u64,
    /// Records with a missing value.
    pub missing: u64,
    /// Per-category statistics, aligned with `labels`.
    pub categories: Vec<CategoryStats>,
}

/// Row/column labels of a 2×2 result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Labels2x2 {
    /// `[exposed, unexposed]`.
    pub exposure: Vec<String>,
    /// `[ill, well]`.
    pub outcome: Vec<String>,
}

/// The stratified part of a `TwoByTwo` result.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StratifiedResult {
    /// Combined stratum labels in category order.
    pub strata_labels: Vec<String>,
    /// Mantel–Haenszel, crude and Breslow–Day–Tarone statistics.
    pub stats: StratifiedStats,
}

/// `tables` result (spec §4.1): `TwoByTwo` when both variables have
/// exactly two levels, otherwise `RxC`. The `TwoByTwo` variant is large
/// because it embeds two full stats structs — the shape is the plan's
/// interface, so boxing is declined here.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TablesResult {
    /// Both variables have exactly two levels.
    TwoByTwo {
        /// Orientation labels.
        labels: Labels2x2,
        /// Statistics of the pooled (unstratified) table.
        crude: TwoByTwoStats,
        /// Per-stratum pooled statistics when `stratify_by` is set.
        #[serde(skip_serializing_if = "Option::is_none")]
        stratified: Option<StratifiedResult>,
    },
    /// Any other shape — a Pearson chi-square table.
    RxC {
        /// Row categories.
        row_labels: Vec<String>,
        /// Column categories.
        col_labels: Vec<String>,
        /// The pooled count table.
        counts: Vec<Vec<u64>>,
        /// Pearson chi-square and the `low_expected` flag.
        stats: RxcStats,
    },
}

/// An empty timestamp is `InvalidOptions` (spec §4.1).
fn check_ctx(ctx: &RunContext) -> Result<(), EngineError> {
    if ctx.timestamp.is_empty() {
        return Err(EngineError::InvalidOptions {
            reason: "RunContext.timestamp must not be empty".into(),
        });
    }
    Ok(())
}

/// Receipt assembly shared by `freq` and `tables` (spec §4.3): the
/// `validation` list is the registry coverage of every statistic the
/// result emits, sorted by statistic id.
fn build_receipt(
    ds: &Dataset,
    procedure: &str,
    options: Json,
    stats: &[(&str, Json)],
    exclusions: BTreeMap<String, u64>,
    ctx: &RunContext,
) -> Receipt {
    let mut entries = BTreeMap::new();
    for (family, value) in stats {
        for path in statistic_ids(value) {
            let id = format!("{family}.{path}");
            if let Some(c) = REGISTRY.iter().find(|c| c.statistic == id) {
                entries.insert(
                    id.clone(),
                    ValidationEntry {
                        statistic: id,
                        fixtures: c.fixtures.iter().map(|f| (*f).to_string()).collect(),
                        tolerance_class: c.class,
                        tolerance: c.class.tolerance(),
                    },
                );
            }
        }
    }
    Receipt::new(
        crate::receipt::DatasetStamp {
            id: ds.id().to_string(),
            version: ds.version(),
            sha256: ds.content_hash_hex(),
        },
        procedure.to_string(),
        options,
        entries.into_values().collect(),
        exclusions,
        ctx.timestamp.clone(),
        crate::receipt::EngineStamp::current(),
    )
}

fn options_json<T: Serialize>(opts: &T) -> Result<Json, EngineError> {
    serde_json::to_value(opts).map_err(|e| EngineError::InvalidOptions {
        reason: format!("options do not serialize: {e}"),
    })
}
