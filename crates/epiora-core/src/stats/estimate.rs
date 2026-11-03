//! `Estimate` — the one value type of `stats` (spec §3.5). Every
//! statistic is either a finite `Value` or `Undefined { reason }`;
//! nothing NaN or infinite ever reaches a serialized result.

use serde::{ser::SerializeMap, Serialize};

/// Why a statistic is undefined (spec §3.5). Codes are the snake_case
/// spellings used in the serialized `{"undefined": "<code>"}` leaf.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UndefinedReason {
    /// A needed cell count is zero (0/0 or division producing ±∞).
    ZeroCell,
    /// A row, column or stratum margin is zero.
    ZeroMargin,
    /// The table has no records at all.
    EmptyTable,
    /// The root is not bracketed in the search interval.
    NotBracketed,
    /// The root-finder did not converge.
    NoConvergence,
    /// The limit lies at the boundary of the conditional support.
    Unbounded,
}

impl UndefinedReason {
    /// The stable snake_case code.
    pub fn code(self) -> &'static str {
        match self {
            UndefinedReason::ZeroCell => "zero_cell",
            UndefinedReason::ZeroMargin => "zero_margin",
            UndefinedReason::EmptyTable => "empty_table",
            UndefinedReason::NotBracketed => "not_bracketed",
            UndefinedReason::NoConvergence => "no_convergence",
            UndefinedReason::Unbounded => "unbounded",
        }
    }
}

/// A statistic: a finite number, or `Undefined` with a reason.
#[derive(Debug, Clone, PartialEq)]
pub enum Estimate {
    /// A finite computed value.
    Value(f64),
    /// Statistically undefined; serializes as `{"undefined": "<code>"}`.
    Undefined { reason: UndefinedReason },
}

impl Estimate {
    /// `Value(x)` when `x` is finite, `Undefined { reason }` otherwise —
    /// the only way a computed `f64` becomes an estimate, so NaN and
    /// infinity can never be emitted.
    pub fn from_f64(x: f64, reason: UndefinedReason) -> Estimate {
        if x.is_finite() {
            Estimate::Value(x)
        } else {
            Estimate::Undefined { reason }
        }
    }

    /// `Some(v)` for `Value(v)`, `None` when undefined.
    pub fn value(&self) -> Option<f64> {
        match self {
            Estimate::Value(v) => Some(*v),
            Estimate::Undefined { .. } => None,
        }
    }
}

impl Serialize for Estimate {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Estimate::Value(v) => s.serialize_f64(*v),
            Estimate::Undefined { reason } => {
                let mut map = s.serialize_map(Some(1))?;
                map.serialize_entry("undefined", reason.code())?;
                map.end()
            }
        }
    }
}

/// A two-sided confidence interval.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Ci {
    /// Lower bound.
    pub lower: Estimate,
    /// Upper bound.
    pub upper: Estimate,
}
