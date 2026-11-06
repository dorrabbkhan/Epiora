//! Coercion of raw string cells into typed column data (spec §6.2):
//! missing codes become `None`; a non-missing value that fails the
//! chosen type becomes `None` and is counted in `coerced_to_missing`;
//! cells in `forced_coerced` (spreadsheet error cells, unconvertible
//! date serials) are always a coercion. Values keep their original
//! whitespace — trimming applies only to the missing-code match.

use crate::infer::{self, InferCtx};
use crate::table::Table;
use epiora_core::dataset::{ColumnData, ValueType};

/// A coerced column plus its missing/coercion counts.
pub(crate) struct Coerced {
    /// The typed values.
    pub data: ColumnData,
    /// Cells that were missing codes.
    pub missing: u64,
    /// Non-missing cells that failed the chosen type (→ `None`), plus
    /// forced cells.
    pub coerced: u64,
}

/// Coerces column `col` of `table` to `chosen`.
pub(crate) fn coerce_column(
    table: &Table,
    col: usize,
    chosen: ValueType,
    missing_codes: &[String],
    ctx: &InferCtx,
) -> Coerced {
    let mut missing = 0u64;
    let mut coerced = 0u64;
    macro_rules! coerce {
        ($variant:ident, $f:expr) => {{
            let mut values: Vec<Option<_>> = Vec::with_capacity(table.rows.len());
            for (r, row) in table.rows.iter().enumerate() {
                let cell = row.get(col).map(String::as_str).unwrap_or("");
                if table.forced_coerced.contains(&(r, col)) {
                    coerced += 1;
                    values.push(None);
                } else if infer::is_missing(cell, missing_codes) {
                    missing += 1;
                    values.push(None);
                } else {
                    match $f(cell) {
                        Some(v) => values.push(Some(v)),
                        None => {
                            coerced += 1;
                            values.push(None);
                        }
                    }
                }
            }
            ColumnData::$variant(values)
        }};
    }
    let data = match chosen {
        ValueType::Boolean => coerce!(Boolean, |s: &str| infer::parse_boolean(s)),
        ValueType::Integer => coerce!(Integer, |s: &str| s.parse::<i64>().ok()),
        ValueType::Real => coerce!(Real, |s: &str| infer::parse_real(s, ctx.decimal)),
        ValueType::Date => coerce!(Date, |s: &str| infer::parse_date(s, ctx.date_format)),
        ValueType::Text => coerce!(Text, |s: &str| Some(s.to_string())),
    };
    Coerced {
        data,
        missing,
        coerced,
    }
}
