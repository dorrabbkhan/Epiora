//! MEANS statistics (v1.0 spec §4.2): per-group descriptives, the
//! pooled (Student) and Welch (Satterthwaite) two-sample t-tests,
//! one-way ANOVA and Bartlett's variance test. Percentiles are R
//! `quantile(type = 2)` — Epi Info 7's definition, verified in v1.0
//! Task 4 — and the mode is the smallest of tied most-frequent
//! values. Variance is the two-pass method on Neumaier sums.

use crate::special::{chisq_sf, f_sf, t_quantile, t_sf};
use crate::stats::summation::neumaier_sum;
use crate::stats::{Ci, Estimate, UndefinedReason};
use serde::Serialize;

/// R's `.Machine$double.eps` — the t-test's "essentially constant"
/// threshold factor.
const DOUBLE_EPS: f64 = 2.220446049250313e-16;

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

/// The per-group inputs the test builders share — also what Task 8's
/// summary-input calculators call (`t_tests_from_summary`,
/// `anova_from_summary`). `variance` is `None` below n = 2.
pub(crate) struct GroupSummary {
    /// Observations.
    pub n: u64,
    /// Mean.
    pub mean: f64,
    /// Sample variance when `n ≥ 2`.
    pub variance: Option<f64>,
}

pub(crate) fn summarize(values: &[f64]) -> GroupSummary {
    let n = values.len() as u64;
    let mean = neumaier_sum(values.iter().copied()) / n as f64;
    let variance = if n >= 2 {
        Some(neumaier_sum(values.iter().map(|x| (x - mean) * (x - mean))) / (n - 1) as f64)
    } else {
        None
    };
    GroupSummary { n, mean, variance }
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
/// family only.
pub(crate) fn group_stats(label: String, values: &[f64]) -> GroupStats {
    let n = values.len() as u64;
    let total = neumaier_sum(values.iter().copied());
    let value = |x: f64| Estimate::from_f64(x, UndefinedReason::TooFew);
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
    let variance = s
        .variance
        .map(Estimate::Value)
        .unwrap_or(Estimate::Undefined {
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
        total: Estimate::Value(total),
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
/// call would error in R — a group below two observations, or an
/// "essentially constant" pooled or Welch standard error
/// (`stderr < 10·ε·max(|m₁|, |m₂|)`), which is also how both tests
/// fail together on constant data.
pub(crate) fn t_tests_from_summary(a: &GroupSummary, b: &GroupSummary) -> Option<TwoGroupTests> {
    if a.n < 2 || b.n < 2 {
        return None;
    }
    let (n1, n2) = (a.n as f64, b.n as f64);
    let v1 = a.variance?;
    let v2 = b.variance?;
    let diff = a.mean - b.mean;
    let threshold = 10.0 * DOUBLE_EPS * libm::fmax(a.mean.abs(), b.mean.abs());

    let vp = ((n1 - 1.0) * v1 + (n2 - 1.0) * v2) / (n1 + n2 - 2.0);
    let se_p = libm::sqrt(vp * (1.0 / n1 + 1.0 / n2));
    let se1 = libm::sqrt(v1 / n1);
    let se2 = libm::sqrt(v2 / n2);
    let se_w = libm::sqrt(se1 * se1 + se2 * se2);
    if se_p < threshold || se_w < threshold {
        return None;
    }

    let df_p = n1 + n2 - 2.0;
    let df_w = (se_w * se_w * se_w * se_w)
        / (se1 * se1 * se1 * se1 / (n1 - 1.0) + se2 * se2 * se2 * se2 / (n2 - 1.0));
    let t_test = |se: f64, df: f64| -> TTest {
        let t = diff / se;
        let p = 2.0 * t_sf(t.abs(), df);
        let crit = t_quantile(0.975, df).unwrap_or(f64::NAN);
        TTest {
            t: Estimate::from_f64(t, UndefinedReason::ZeroVariance),
            df: Estimate::Value(df),
            p_value: Estimate::from_f64(p, UndefinedReason::ZeroVariance),
            diff: Estimate::Value(diff),
            diff_ci_95: Ci {
                lower: Estimate::from_f64(diff - crit * se, UndefinedReason::ZeroVariance),
                upper: Estimate::from_f64(diff + crit * se, UndefinedReason::ZeroVariance),
            },
        }
    };
    Some(TwoGroupTests {
        pooled: t_test(se_p, df_p),
        welch: t_test(se_w, df_w),
    })
}

/// One-way ANOVA from per-group summaries (spec §4.2). `ss_within`
/// needs a variance from every group; one `too_few` group poisons it
/// and everything downstream (`ms_within`, `f`, `p_value`,
/// `ss_total`), exactly as R's NA propagation.
pub(crate) fn anova_from_summary(groups: &[GroupSummary]) -> Anova {
    let k = groups.len();
    let n_tot: u64 = groups.iter().map(|g| g.n).sum();
    let (k_f, n_f) = (k as f64, n_tot as f64);
    let grand = neumaier_sum(groups.iter().map(|g| g.n as f64 * g.mean)) / n_f;
    let ss_b = neumaier_sum(
        groups
            .iter()
            .map(|g| g.n as f64 * (g.mean - grand) * (g.mean - grand)),
    );
    let ss_w = if groups.iter().all(|g| g.variance.is_some()) {
        Estimate::Value(neumaier_sum(
            groups
                .iter()
                .map(|g| (g.n - 1) as f64 * g.variance.unwrap_or(0.0)),
        ))
    } else {
        Estimate::Undefined {
            reason: UndefinedReason::TooFew,
        }
    };
    let (df_b, df_w, df_t) = (k_f - 1.0, n_f - k_f, n_f - 1.0);
    let ms_b = Estimate::from_f64(ss_b / df_b, UndefinedReason::ZeroVariance);
    let ms_w = match ss_w {
        Estimate::Value(w) => Estimate::from_f64(w / df_w, UndefinedReason::ZeroVariance),
        Estimate::Undefined { reason } => Estimate::Undefined { reason },
    };
    let f = match (&ms_b, &ms_w) {
        (Estimate::Value(_), Estimate::Undefined { reason }) => {
            Estimate::Undefined { reason: *reason }
        }
        (Estimate::Value(b), Estimate::Value(w)) => {
            if *w == 0.0 {
                Estimate::Undefined {
                    reason: UndefinedReason::ZeroVariance,
                }
            } else {
                Estimate::from_f64(b / w, UndefinedReason::ZeroVariance)
            }
        }
        _ => Estimate::Undefined {
            reason: UndefinedReason::ZeroVariance,
        },
    };
    let p = match f {
        Estimate::Value(v) => {
            Estimate::from_f64(f_sf(v, df_b, df_w), UndefinedReason::ZeroVariance)
        }
        Estimate::Undefined { reason } => Estimate::Undefined { reason },
    };
    let ss_total = match ss_w {
        Estimate::Value(w) => Estimate::Value(ss_b + w),
        Estimate::Undefined { reason } => Estimate::Undefined { reason },
    };
    Anova {
        ss_between: Estimate::Value(ss_b),
        ss_within: ss_w,
        ss_total,
        df_between: Estimate::Value(df_b),
        df_within: Estimate::Value(df_w),
        df_total: Estimate::Value(df_t),
        ms_between: ms_b,
        ms_within: ms_w,
        f,
        p_value: p,
    }
}

/// R's `bartlett.test(x, g)`: `None` when any group has fewer than 2
/// observations (R's error); otherwise the corrected χ² — NaN or
/// infinite on zero-variance input, hence `Undefined { zero_variance }`.
pub(crate) fn bartlett_from_summary(groups: &[GroupSummary]) -> Option<Bartlett> {
    if groups.iter().any(|g| g.n < 2) {
        return None;
    }
    let k = groups.len() as f64;
    let n: Vec<f64> = groups.iter().map(|g| (g.n - 1) as f64).collect();
    let v: Vec<f64> = groups.iter().filter_map(|g| g.variance).collect();
    if v.len() != groups.len() {
        return None;
    }
    let n_tot = neumaier_sum(n.iter().copied());
    let v_pooled = neumaier_sum(n.iter().zip(&v).map(|(ni, vi)| ni * vi)) / n_tot;
    let numerator = n_tot * libm::log(v_pooled)
        - neumaier_sum(n.iter().zip(&v).map(|(ni, vi)| ni * libm::log(*vi)));
    let correction =
        1.0 + (neumaier_sum(n.iter().map(|ni| 1.0 / ni)) - 1.0 / n_tot) / (3.0 * (k - 1.0));
    let df = k - 1.0;
    let stat = Estimate::from_f64(numerator / correction, UndefinedReason::ZeroVariance);
    let p = match stat {
        Estimate::Value(x) => Estimate::from_f64(chisq_sf(x, df), UndefinedReason::ZeroVariance),
        Estimate::Undefined { reason } => Estimate::Undefined { reason },
    };
    Some(Bartlett {
        statistic: stat,
        df: Estimate::Value(df),
        p_value: p,
    })
}
