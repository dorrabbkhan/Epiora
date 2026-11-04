//! `rxc` — the Pearson chi-square of an R×C table, matching
//! `chisq.test(table, correct = FALSE)` (spec §3.5). `low_expected`
//! flags any expected cell below 5; the flag is advisory and never a
//! correction.

use super::estimate::{Estimate, UndefinedReason};
use crate::special::chisq_sf;
use serde::Serialize;

/// Serialized R×C statistics — the `expected` key layout of the `rxc`
/// fixtures plus the `low_expected` flag.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RxcStats {
    /// `chi_square.*`.
    pub chi_square: RxcChiSquare,
    /// True when any expected count `row_total × col_total / n` is < 5.
    pub low_expected: bool,
}

/// The Pearson chi-square triple of an R×C table.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct RxcChiSquare {
    /// `sum((o − e)² / e)`.
    pub statistic: Estimate,
    /// `(rows − 1)(cols − 1)`.
    pub df: Estimate,
    /// `pchisq(statistic, df, lower.tail = FALSE)`.
    pub p_value: Estimate,
}

fn undefined(reason: UndefinedReason) -> RxcStats {
    let u = || Estimate::Undefined { reason };
    RxcStats {
        chi_square: RxcChiSquare {
            statistic: u(),
            df: u(),
            p_value: u(),
        },
        low_expected: false,
    }
}

/// Pearson chi-square of `table` (spec §3.5). Rows may not all have the
/// same width; out-of-range cells count as zero. An empty table (no
/// cells or total 0) gives `Undefined { EmptyTable }`, a zero margin
/// `Undefined { ZeroMargin }`, a table with df 0 keeps its (zero)
/// statistic and df but the p-value is `Undefined`.
pub fn rxc(table: &[Vec<u64>]) -> RxcStats {
    let rows = table.len();
    let cols = table.iter().map(Vec::len).max().unwrap_or(0);
    if rows == 0 || cols == 0 {
        return undefined(UndefinedReason::EmptyTable);
    }
    let cell = |r: usize, c: usize| table[r].get(c).copied().unwrap_or(0) as f64;
    let row_totals: Vec<f64> = (0..rows)
        .map(|r| (0..cols).map(|c| cell(r, c)).sum())
        .collect();
    let col_totals: Vec<f64> = (0..cols)
        .map(|c| (0..rows).map(|r| cell(r, c)).sum())
        .collect();
    let n: f64 = row_totals.iter().sum();
    if n == 0.0 {
        return undefined(UndefinedReason::EmptyTable);
    }

    let zm = UndefinedReason::ZeroMargin;
    let df = ((rows - 1) * (cols - 1)) as f64;
    let mut statistic = 0.0;
    let mut low_expected = false;
    let mut zero_margin = false;
    for (r, &rt) in row_totals.iter().enumerate() {
        for (c, &ct) in col_totals.iter().enumerate() {
            let e = rt * ct / n;
            if e == 0.0 {
                zero_margin = true;
            } else {
                let diff = cell(r, c) - e;
                statistic += diff * diff / e;
            }
            low_expected |= e < 5.0;
        }
    }

    if zero_margin {
        // R reports NaN statistics on a zero margin; the degrees of
        // freedom still follow the table shape.
        return RxcStats {
            chi_square: RxcChiSquare {
                statistic: Estimate::Undefined { reason: zm },
                df: Estimate::Value(df),
                p_value: Estimate::Undefined { reason: zm },
            },
            low_expected,
        };
    }

    RxcStats {
        chi_square: RxcChiSquare {
            statistic: Estimate::from_f64(statistic, zm),
            df: Estimate::Value(df),
            p_value: if df > 0.0 {
                Estimate::from_f64(chisq_sf(statistic, df).clamp(0.0, 1.0), zm)
            } else {
                Estimate::Undefined { reason: zm }
            },
        },
        low_expected,
    }
}
