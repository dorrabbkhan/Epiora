//! `notes` — engine-computed assumption notes (v1.0 Task 22,
//! spec §6.1). A [`Note`] is a pure function of values a run already
//! computed; it rides beside `result` and `receipt` on the output
//! wrappers, never inside either, so receipts and corpus parity are
//! bit-for-bit unchanged.

mod attach;
mod rules;

pub(crate) use attach::{calc, means, tables_rxc, tables_two_by_two};

use serde::Serialize;
use std::collections::BTreeMap;

/// One assumption note (spec §6.1): the rule's stable code, the
/// emitted statistics it applies to (validation-registry ids, `[]`
/// marking array positions), the finite engine-computed values the
/// rule fired on, and an optional suggested alternative — a
/// calculator id or a statistic id.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Note {
    /// The rule's stable code — one of [`NOTE_CODES`].
    pub code: String,
    /// The statistics the note applies to, sorted.
    pub statistics: Vec<String>,
    /// The trigger values, keyed by name — finite numbers the engine
    /// itself computed.
    pub values: BTreeMap<String, f64>,
    /// The suggested alternative, or `None` when the output has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggests: Option<String>,
}

/// Every note code the engine can emit (spec §6.1), sorted — note
/// order within an output is by code, then the sorted statistics.
pub const NOTE_CODES: &[&str] = &[
    "expected_count_below_5",
    "heterogeneous_strata",
    "normal_approx_small",
    "unequal_variances",
    "zero_cell",
];
