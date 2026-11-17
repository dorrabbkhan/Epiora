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
    /// Too few observations for the statistic (variance needs ≥ 2,
    /// a mean needs ≥ 1).
    TooFew,
    /// The values carry no variance (a zero denominator).
    ZeroVariance,
    /// The statistic is not representable — a finite input whose
    /// computed value overflowed or cancelled outside f64's range.
    OutOfRange,
    /// The exact path would enumerate a support above the owner-set
    /// size limits (v1.0 Task 7 follow-up, PR #74): the work is
    /// refused before any allocation.
    TooLarge,
    /// The required sample size is infinite — the effect being
    /// powered for is zero (equal proportions, a unit effect
    /// measure, a zero mean difference; v1.0 Task 9, Review Focus 3).
    InfiniteSampleSize,
    /// A table margin is zero, so an expected cell is zero and the
    /// chi-square is not computable (the `r_by_c` calculator's
    /// `zero_expected` leaf).
    ZeroExpected,
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
            UndefinedReason::TooFew => "too_few",
            UndefinedReason::ZeroVariance => "zero_variance",
            UndefinedReason::OutOfRange => "out_of_range",
            UndefinedReason::TooLarge => "too_large",
            UndefinedReason::InfiniteSampleSize => "infinite_sample_size",
            UndefinedReason::ZeroExpected => "zero_expected",
        }
    }

    /// The reason a serialized `{"undefined": "<code>"}` leaf names —
    /// the inverse of `code` (explain relays the leaf's reason; it
    /// never invents one).
    pub(crate) fn from_code(code: &str) -> Option<UndefinedReason> {
        Some(match code {
            "zero_cell" => UndefinedReason::ZeroCell,
            "zero_margin" => UndefinedReason::ZeroMargin,
            "empty_table" => UndefinedReason::EmptyTable,
            "not_bracketed" => UndefinedReason::NotBracketed,
            "no_convergence" => UndefinedReason::NoConvergence,
            "unbounded" => UndefinedReason::Unbounded,
            "too_few" => UndefinedReason::TooFew,
            "zero_variance" => UndefinedReason::ZeroVariance,
            "out_of_range" => UndefinedReason::OutOfRange,
            "too_large" => UndefinedReason::TooLarge,
            "infinite_sample_size" => UndefinedReason::InfiniteSampleSize,
            "zero_expected" => UndefinedReason::ZeroExpected,
            _ => return None,
        })
    }
}

/// A statistic: a finite number, or `Undefined` with a reason.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(
        export,
        type = "number | { \"undefined\": \"zero_cell\" | \"zero_margin\" | \"empty_table\" | \"not_bracketed\" | \"no_convergence\" | \"unbounded\" | \"too_few\" | \"zero_variance\" | \"out_of_range\" | \"too_large\" | \"infinite_sample_size\" | \"zero_expected\" }"
    )
)]
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
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Ci {
    /// Lower bound.
    pub lower: Estimate,
    /// Upper bound.
    pub upper: Estimate,
}
