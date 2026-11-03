//! `proportions` — per-category count, percent, cumulative percent and
//! 95% confidence intervals of a FREQ table (spec §3.5). Clopper–Pearson
//! is `binom.test`'s interval through `beta_quantile`; Wilson is
//! `prop.test(correct = FALSE)`. Both are on the 0–100 scale, like the
//! fixture leaves.

use super::estimate::{Ci, Estimate, UndefinedReason};
use crate::special::{beta_quantile, normal_quantile};
use serde::Serialize;

/// One category of a frequency table.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CategoryStats {
    /// Number of records in the category.
    pub count: u64,
    /// `100 × count / total`.
    pub percent: Estimate,
    /// `100 × running sum / total`.
    pub cumulative_percent: Estimate,
    /// Clopper–Pearson 95% CI on the percent scale.
    pub ci_95_clopper_pearson: Ci,
    /// Wilson score 95% CI on the percent scale.
    pub ci_95_wilson: Ci,
}

/// `qnorm(0.975)`, as in `two_by_two`.
fn z_975() -> f64 {
    normal_quantile(0.975).unwrap_or(1.959963984540054)
}

/// `qbeta(p, a, b)` as an estimate; a failed root is
/// `Undefined { NoConvergence }`.
fn qb(p: f64, a: f64, b: f64) -> Estimate {
    match beta_quantile(p, a, b) {
        Some(v) if v.is_finite() => Estimate::Value(v * 100.0),
        _ => Estimate::Undefined {
            reason: UndefinedReason::NoConvergence,
        },
    }
}

/// Per-category statistics of `counts`, in order (spec §3.5). The
/// denominator is the sum of the counts — i.e. the non-missing total
/// unless the caller appended a "(missing)" category. A zero
/// denominator makes the derived values `Undefined { EmptyTable }`.
pub fn proportions(counts: &[u64]) -> Vec<CategoryStats> {
    // Summed in f64 so a caller's counts can never overflow-panic.
    let nf: f64 = counts.iter().map(|&c| c as f64).sum();
    let n = nf as u64;
    let z = z_975();
    let et = UndefinedReason::EmptyTable;
    let mut cumulative = 0.0;
    counts
        .iter()
        .map(|&count| {
            cumulative += count as f64;
            let k = count as f64;
            let percent = if n == 0 {
                Estimate::Undefined { reason: et }
            } else {
                Estimate::Value(k / nf * 100.0)
            };
            let cumulative_percent = if n == 0 {
                Estimate::Undefined { reason: et }
            } else {
                Estimate::Value(cumulative / nf * 100.0)
            };
            CategoryStats {
                count,
                percent,
                cumulative_percent,
                ci_95_clopper_pearson: clopper_pearson(count, n),
                ci_95_wilson: wilson(k, nf, z),
            }
        })
        .collect()
}

/// `binom.test(k, n)$conf.int × 100`: the exact beta interval, with the
/// conventional 0 / 100 endpoints when the count is on a boundary.
fn clopper_pearson(k: u64, n: u64) -> Ci {
    if n == 0 {
        let u = || Estimate::Undefined {
            reason: UndefinedReason::EmptyTable,
        };
        return Ci {
            lower: u(),
            upper: u(),
        };
    }
    let (kf, nf) = (k as f64, n as f64);
    Ci {
        lower: if k == 0 {
            Estimate::Value(0.0)
        } else {
            qb(0.025, kf, nf - kf + 1.0)
        },
        upper: if k == n {
            Estimate::Value(100.0)
        } else {
            qb(0.975, kf + 1.0, nf - kf)
        },
    }
}

/// `prop.test(k, n, correct = FALSE)$conf.int × 100`: the Wilson score
/// interval.
fn wilson(k: f64, n: f64, z: f64) -> Ci {
    if n == 0.0 {
        let u = || Estimate::Undefined {
            reason: UndefinedReason::EmptyTable,
        };
        return Ci {
            lower: u(),
            upper: u(),
        };
    }
    let p = k / n;
    let zz = z * z;
    let denom = 1.0 + zz / n;
    let centre = p + zz / (2.0 * n);
    let half = z * libm::sqrt(p * (1.0 - p) / n + zz / (4.0 * n * n)) / denom;
    Ci {
        lower: Estimate::Value((centre / denom - half) * 100.0),
        upper: Estimate::Value((centre / denom + half) * 100.0),
    }
}
