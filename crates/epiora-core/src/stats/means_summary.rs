//! Summary-level MEANS test builders (v1.0 Task 8): the pooled
//! (Student) and Welch (Satterthwaite) two-sample t-tests, one-way
//! ANOVA and Bartlett's variance test computed from per-group
//! `n`/`mean`/`variance` summaries. MEANS derives the summaries from
//! raw data; the continuous-summary calculators supply them
//! directly — both share this one implementation.
//!
//! Extracted from `stats/means.rs` in Task 8 so that file stays under
//! the ~400-line limit.

use crate::special::{chisq_sf, f_sf, t_quantile, t_sf};
use crate::stats::means::{Anova, Bartlett, TTest, TwoGroupTests};
use crate::stats::summation::neumaier_sum;
use crate::stats::{Ci, Estimate, UndefinedReason};

/// R's `.Machine$double.eps` — the t-test's "essentially constant"
/// threshold factor.
const DOUBLE_EPS: f64 = 2.220446049250313e-16;

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

/// call would error in R — a group below two observations, or an
/// "essentially constant" pooled or Welch standard error
/// (`stderr < 10·ε·max(|m₁|, |m₂|)`), which is also how both tests
/// fail together on constant data.
pub(crate) fn t_tests_from_summary(a: &GroupSummary, b: &GroupSummary) -> Option<TwoGroupTests> {
    t_tests_from_summary_at(a, b, 95.0)
}

/// `t.test(x1, x2)` for both variance assumptions at `conf` percent.
/// `diff_ci_95` carries the CI at `conf` — the field name is fixed by
/// the serialized `MeansResult`; calculators pass their `conf_level`.
pub(crate) fn t_tests_from_summary_at(
    a: &GroupSummary,
    b: &GroupSummary,
    conf: f64,
) -> Option<TwoGroupTests> {
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
        let crit = t_quantile((1.0 + conf / 100.0) / 2.0, df).unwrap_or(f64::NAN);
        TTest {
            t: Estimate::from_f64(t, UndefinedReason::ZeroVariance),
            df: Estimate::from_f64(df, UndefinedReason::OutOfRange),
            p_value: Estimate::from_f64(p, UndefinedReason::ZeroVariance),
            diff: Estimate::from_f64(diff, UndefinedReason::OutOfRange),
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
/// `ss_total`), exactly as R's NA propagation. A sum of squares that
/// overflows f64's range is `Undefined { out_of_range }`, propagated
/// the same way.
pub(crate) fn anova_from_summary(groups: &[GroupSummary]) -> Anova {
    let k = groups.len();
    let n_tot: u64 = groups.iter().map(|g| g.n).sum();
    let (k_f, n_f) = (k as f64, n_tot as f64);
    let grand = neumaier_sum(groups.iter().map(|g| g.n as f64 * g.mean)) / n_f;
    let ss_b = Estimate::from_f64(
        neumaier_sum(
            groups
                .iter()
                .map(|g| g.n as f64 * (g.mean - grand) * (g.mean - grand)),
        ),
        UndefinedReason::OutOfRange,
    );
    let ss_w = if groups.iter().all(|g| g.variance.is_some()) {
        Estimate::from_f64(
            neumaier_sum(
                groups
                    .iter()
                    .map(|g| (g.n - 1) as f64 * g.variance.unwrap_or(0.0)),
            ),
            UndefinedReason::OutOfRange,
        )
    } else {
        Estimate::Undefined {
            reason: UndefinedReason::TooFew,
        }
    };
    let (df_b, df_w, df_t) = (k_f - 1.0, n_f - k_f, n_f - 1.0);
    let ms_b = match ss_b {
        Estimate::Value(b) => Estimate::from_f64(b / df_b, UndefinedReason::OutOfRange),
        Estimate::Undefined { reason } => Estimate::Undefined { reason },
    };
    let ms_w = match ss_w {
        Estimate::Value(w) => Estimate::from_f64(w / df_w, UndefinedReason::ZeroVariance),
        Estimate::Undefined { reason } => Estimate::Undefined { reason },
    };
    let f = match (&ms_b, &ms_w) {
        (Estimate::Value(b), Estimate::Value(w)) => {
            if *w == 0.0 {
                Estimate::Undefined {
                    reason: UndefinedReason::ZeroVariance,
                }
            } else {
                Estimate::from_f64(b / w, UndefinedReason::ZeroVariance)
            }
        }
        // `f` follows whichever mean square is undefined.
        (Estimate::Undefined { reason }, _) | (_, Estimate::Undefined { reason }) => {
            Estimate::Undefined { reason: *reason }
        }
    };
    let p = match f {
        Estimate::Value(v) => {
            Estimate::from_f64(f_sf(v, df_b, df_w), UndefinedReason::ZeroVariance)
        }
        Estimate::Undefined { reason } => Estimate::Undefined { reason },
    };
    // `df_*` are exact integer counts (k − 1, N − k, N − 1), always
    // finite, so they stay `Estimate::Value`.
    let ss_total = match (&ss_b, &ss_w) {
        (Estimate::Value(b), Estimate::Value(w)) => {
            Estimate::from_f64(b + w, UndefinedReason::OutOfRange)
        }
        (Estimate::Undefined { reason }, _) | (_, Estimate::Undefined { reason }) => {
            Estimate::Undefined { reason: *reason }
        }
    };
    Anova {
        ss_between: ss_b,
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
