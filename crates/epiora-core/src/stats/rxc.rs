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

/// The closed-form intermediates of `rxc` — the expected cells, the
/// Pearson sum and the expected-cell diagnostics (v1.0 Task 10,
/// spec §6). Shared by the `rxc` and `r_by_c` runs and the explain
/// traces.
pub(crate) struct RxcParts {
    /// Row count.
    pub rows: usize,
    /// Column count (widest row).
    pub cols: usize,
    /// Row totals.
    pub row_totals: Vec<f64>,
    /// Column totals.
    pub col_totals: Vec<f64>,
    /// Grand total.
    pub n: f64,
    /// `rt·ct/n` per cell, row-major over `cols`.
    pub expected: Vec<f64>,
    /// `Σ (o−e)²/e` over cells with `e > 0`.
    pub statistic: f64,
    /// `(rows − 1)(cols − 1)`.
    pub df: f64,
    /// Expected cells `< 5` (Cochran's criterion).
    pub lt5: u64,
    /// Expected cells `< 1`.
    pub lt1: u64,
    /// Any expected cell exactly 0 — the zero-margin flag.
    pub zero_margin: bool,
}

/// The intermediates of `rxc(table)` — shared by the result and the
/// explain trace. `cell(r, c)` reads `table[r][c]` as `rxc` does:
/// ragged or missing cells count as 0.
pub(crate) fn rxc_parts(table: &[Vec<u64>]) -> RxcParts {
    let rows = table.len();
    let cols = table.iter().map(Vec::len).max().unwrap_or(0);
    let cell = |r: usize, c: usize| table[r].get(c).copied().unwrap_or(0) as f64;
    let row_totals: Vec<f64> = (0..rows)
        .map(|r| (0..cols).map(|c| cell(r, c)).sum())
        .collect();
    let col_totals: Vec<f64> = (0..cols)
        .map(|c| (0..rows).map(|r| cell(r, c)).sum())
        .collect();
    let n: f64 = row_totals.iter().sum();
    let df = ((rows.max(1) - 1) * (cols.max(1) - 1)) as f64;

    let mut statistic = 0.0;
    let mut lt5 = 0u64;
    let mut lt1 = 0u64;
    let mut zero_margin = false;
    let mut expected = Vec::with_capacity(rows * cols);
    for (r, &rt) in row_totals.iter().enumerate() {
        for (c, &ct) in col_totals.iter().enumerate() {
            let e = rt * ct / n;
            expected.push(e);
            if e == 0.0 {
                zero_margin = true;
            } else if e > 0.0 {
                let diff = cell(r, c) - e;
                statistic += diff * diff / e;
            }
            lt5 += u64::from(e < 5.0);
            lt1 += u64::from(e < 1.0);
        }
    }
    RxcParts {
        rows,
        cols,
        row_totals,
        col_totals,
        n,
        expected,
        statistic,
        df,
        lt5,
        lt1,
        zero_margin,
    }
}

/// Pearson chi-square of `table` (spec §3.5). Rows may not all have the
/// same width; out-of-range cells count as zero. An empty table (no
/// cells or total 0) gives `Undefined { EmptyTable }`, a zero margin
/// `Undefined { ZeroMargin }`, a table with df 0 keeps its (zero)
/// statistic and df but the p-value is `Undefined`.
pub fn rxc(table: &[Vec<u64>]) -> RxcStats {
    if table.is_empty() || table.iter().all(Vec::is_empty) {
        return undefined(UndefinedReason::EmptyTable);
    }
    let p = rxc_parts(table);
    if p.n == 0.0 {
        return undefined(UndefinedReason::EmptyTable);
    }

    let zm = UndefinedReason::ZeroMargin;
    let df = p.df;
    let statistic = p.statistic;
    let low_expected = p.lt5 > 0;

    if p.zero_margin {
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
