//! MEANS statistics (v1.0 spec §4.2): per-group descriptives, the
//! pooled (Student) and Welch (Satterthwaite) two-sample t-tests,
//! one-way ANOVA and Bartlett's variance test. Percentiles are R
//! `quantile(type = 2)` — Epi Info 7's definition, verified in v1.0
//! Task 4 — and the mode is the smallest of tied most-frequent
//! values. Variance is the two-pass method on Neumaier sums.

use crate::stats::summation::neumaier_sum;
use crate::stats::{Ci, Estimate, UndefinedReason};
use serde::Serialize;

/// One group's descriptives (spec §4.2): `observations` is a count;
/// every other field is an [`Estimate`].
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct GroupStats {
    /// The group label — the category label or `"(missing)"`.
    pub label: String,
    /// Non-missing values in the group.
    pub observations: u64,
    /// Neumaier sum of the values in data order (`0` when empty).
    pub total: Estimate,
    /// `total / observations`.
    pub mean: Estimate,
    /// Sample variance (n − 1 denominator); `too_few` below n = 2.
    pub variance: Estimate,
    /// `sqrt(variance)`.
    pub std_dev: Estimate,
    /// Smallest value.
    pub min: Estimate,
    /// R `quantile(type = 2)` at 0.25.
    pub p25: Estimate,
    /// The median (`quantile(type = 2)` at 0.5).
    pub median: Estimate,
    /// R `quantile(type = 2)` at 0.75.
    pub p75: Estimate,
    /// Largest value.
    pub max: Estimate,
    /// The smallest of tied most-frequent values.
    pub mode: Estimate,
}

/// One two-sample t-test: statistic, df, two-sided p, the mean
/// difference (group 1 − group 2) and its 95% CI.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TTest {
    /// `diff / stderr`.
    pub t: Estimate,
    /// Student: n₁ + n₂ − 2; Welch: the Satterthwaite df.
    pub df: Estimate,
    /// `2 · pt(−|t|, df)`.
    pub p_value: Estimate,
    /// `mean₁ − mean₂` in category order.
    pub diff: Estimate,
    /// 95% CI of `diff` at `qt(0.975, df)` — the exact t quantile at
    /// the (fractional) Welch df, per the owner decision on PR #67.
    pub diff_ci_95: Ci,
}

/// Both two-group tests (spec §4.2).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct TwoGroupTests {
    /// `t.test(var.equal = TRUE)`.
    pub pooled: TTest,
    /// `t.test(var.equal = FALSE)`.
    pub welch: TTest,
}

/// One-way ANOVA over ≥ 2 groups (spec §4.2). `ss_within` and every
/// statistic downstream of it is `Undefined { too_few }` when a group
/// has too few observations for a variance, and `f`/`p_value` are
/// `Undefined { zero_variance }` when `ms_within` is 0.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Anova {
    /// `Σ nᵢ(meanᵢ − grand)²`.
    pub ss_between: Estimate,
    /// `Σ (nᵢ − 1)varᵢ`.
    pub ss_within: Estimate,
    /// `ss_between + ss_within`.
    pub ss_total: Estimate,
    /// k − 1.
    pub df_between: Estimate,
    /// N − k.
    pub df_within: Estimate,
    /// N − 1.
    pub df_total: Estimate,
    /// `ss_between / df_between`.
    pub ms_between: Estimate,
    /// `ss_within / df_within`.
    pub ms_within: Estimate,
    /// `ms_between / ms_within`.
    pub f: Estimate,
    /// `pf(F, df_between, df_within, lower.tail = FALSE)`.
    pub p_value: Estimate,
}

/// Bartlett's test for equality of variances (spec §4.2), R's
/// `bartlett.test` with the small-sample correction.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct Bartlett {
    /// The corrected χ² statistic.
    pub statistic: Estimate,
    /// k − 1.
    pub df: Estimate,
    /// `pchisq(statistic, df, lower.tail = FALSE)`.
    pub p_value: Estimate,
}

/// R `quantile(x, p, type = 2)`: `h = np + 0.5`; a half-integer `h`
/// averages the two bracketing order statistics, otherwise the
/// nearest one. `sorted` must be non-empty and ascending.
fn quantile_type2(sorted: &[f64], p: f64) -> f64 {
    let n = sorted.len();
    let h = n as f64 * p + 0.5;
    let j = libm::floor(h);
    if h - j == 0.5 {
        (sorted[j as usize - 1] + sorted[j as usize]) / 2.0
    } else {
        sorted[libm::floor(h + 0.5) as usize - 1]
    }
}

/// `means_group(x)`: the descriptives of one group's values (or the
/// overall block). An empty group reports `total = 0` and every other
/// statistic `Undefined { too_few }`; n = 1 undefines the variance
/// family only. Extreme finite inputs whose statistic falls outside
/// f64's range report `Undefined { out_of_range }` — never a NaN or
/// infinite `Value`.
pub(crate) fn group_stats(label: String, values: &[f64]) -> GroupStats {
    let n = values.len() as u64;
    let total = neumaier_sum(values.iter().copied());
    let value = |x: f64| Estimate::from_f64(x, UndefinedReason::OutOfRange);
    if n == 0 {
        return GroupStats {
            label,
            observations: 0,
            total: Estimate::Value(0.0),
            mean: Estimate::Undefined {
                reason: UndefinedReason::TooFew,
            },
            variance: Estimate::Undefined {
                reason: UndefinedReason::TooFew,
            },
            std_dev: Estimate::Undefined {
                reason: UndefinedReason::TooFew,
            },
            min: Estimate::Undefined {
                reason: UndefinedReason::TooFew,
            },
            p25: Estimate::Undefined {
                reason: UndefinedReason::TooFew,
            },
            median: Estimate::Undefined {
                reason: UndefinedReason::TooFew,
            },
            p75: Estimate::Undefined {
                reason: UndefinedReason::TooFew,
            },
            max: Estimate::Undefined {
                reason: UndefinedReason::TooFew,
            },
            mode: Estimate::Undefined {
                reason: UndefinedReason::TooFew,
            },
        };
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let s = summarize(values);
    let variance = s.variance.map(value).unwrap_or(Estimate::Undefined {
        reason: UndefinedReason::TooFew,
    });
    let std_dev = match &variance {
        Estimate::Value(v) => value(libm::sqrt(*v)),
        Estimate::Undefined { reason } => Estimate::Undefined { reason: *reason },
    };
    // Mode: the smallest value attaining the maximum frequency — the
    // sorted walk sees ties contiguously.
    let mut best = sorted[0];
    let (mut best_n, mut run) = (0_usize, 0_usize);
    for (i, &x) in sorted.iter().enumerate() {
        run = if i > 0 && x == sorted[i - 1] {
            run + 1
        } else {
            1
        };
        if run > best_n {
            best_n = run;
            best = x;
        }
    }
    GroupStats {
        label,
        observations: n,
        total: value(total),
        mean: value(s.mean),
        variance,
        std_dev,
        min: value(sorted[0]),
        p25: value(quantile_type2(&sorted, 0.25)),
        median: value(quantile_type2(&sorted, 0.5)),
        p75: value(quantile_type2(&sorted, 0.75)),
        max: value(sorted[sorted.len() - 1]),
        mode: value(best),
    }
}

/// `t.test(x1, x2)` for both variance assumptions. `None` when either
pub(crate) use crate::stats::means_summary::{
    anova_from_summary, bartlett_from_summary, summarize, t_tests_from_summary,
    t_tests_from_summary_at, GroupSummary,
};
