//! Summary-level MEANS test builders (v1.0 Task 8): the pooled
//! (Student) and Welch (Satterthwaite) two-sample t-tests, one-way
//! ANOVA and Bartlett's variance test computed from per-group
//! `n`/`mean`/`variance` summaries. MEANS derives the summaries from
//! raw data; the continuous-summary calculators supply them
//! directly — both share this one implementation.
//!
//! Extracted from `stats/means.rs` in Task 8 so that file stays under
//! the ~400-line limit.

use crate::special::{chisq_sf, f_sf, t_sf};
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

impl GroupSummary {
    /// `n·mean` — the group total `summarize` accumulates. Shared by
    /// `anova_parts`' grand sum and the explain trace (v1.0 Task 10,
    /// spec §6).
    pub fn total(&self) -> f64 {
        self.n as f64 * self.mean
    }

    /// `n − 1` — the sample-variance denominator, NaN on an empty
    /// group (`n = 0` has no `n − 1` in `u64`). Shared by the explain
    /// trace's `sample_variance`/`ss_within` intermediates.
    pub fn df(&self) -> f64 {
        self.n.checked_sub(1).map(|d| d as f64).unwrap_or(f64::NAN)
    }

    /// `(n−1)·v` — the Σ(x−mean)² `summarize` accumulates, NaN when
    /// no variance exists (or the group is empty). Shared by
    /// `anova_parts`' `ss_w` and the explain trace's
    /// `sample_variance` intermediate.
    pub fn ssq(&self) -> f64 {
        self.df() * self.variance.unwrap_or(f64::NAN)
    }
}

/// call would error in R — a group below two observations, or an
/// "essentially constant" pooled or Welch standard error
/// (`stderr < 10·ε·max(|m₁|, |m₂|)`), which is also how both tests
/// fail together on constant data.
pub(crate) fn t_tests_from_summary(a: &GroupSummary, b: &GroupSummary) -> Option<TwoGroupTests> {
    t_tests_from_summary_at(a, b, 95.0)
}

/// The closed-form intermediates of the pooled and Welch t-tests
/// (v1.0 Task 10, spec §6): the explain trace's named values. `None`
/// for the same inputs `t_tests_from_summary_at` rejects.
pub(crate) struct TParts {
    /// `mean₁ − mean₂`.
    pub diff: f64,
    /// Pooled variance `((n₁−1)v₁ + (n₂−1)v₂)/(n₁+n₂−2)`.
    pub var_pooled: f64,
    /// `sqrt(var_pooled·(1/n₁ + 1/n₂))`.
    pub se_pooled: f64,
    /// `sqrt(v₁/n₁ + v₂/n₂)`.
    pub se_welch: f64,
    /// `n₁ + n₂ − 2`.
    pub df_pooled: f64,
    /// The Satterthwaite df.
    pub df_welch: f64,
    /// `diff/se_pooled` — the Student t.
    pub t_pooled: f64,
    /// `diff/se_welch` — the Welch t.
    pub t_welch: f64,
}

/// The intermediates of `t_tests_from_summary_at(a, b, _)` — shared
/// by the result and the explain trace.
pub(crate) fn t_parts(a: &GroupSummary, b: &GroupSummary) -> Option<TParts> {
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
    Some(TParts {
        diff,
        var_pooled: vp,
        se_pooled: se_p,
        se_welch: se_w,
        df_pooled: df_p,
        df_welch: df_w,
        t_pooled: diff / se_p,
        t_welch: diff / se_w,
    })
}

/// `t.test(x1, x2)` for both variance assumptions at `conf` percent.
/// `diff_ci_95` carries the CI at `conf` — the field name is fixed by
/// the serialized `MeansResult`; calculators pass their `conf_level`.
pub(crate) fn t_tests_from_summary_at(
    a: &GroupSummary,
    b: &GroupSummary,
    conf: f64,
) -> Option<TwoGroupTests> {
    let p = t_parts(a, b)?;
    let diff = p.diff;
    let t_test = |t: f64, se: f64, df: f64| -> TTest {
        let p = 2.0 * t_sf(t.abs(), df);
        let crit = crate::calc::counts::shared::t_crit(conf, df);
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
        pooled: t_test(p.t_pooled, p.se_pooled, p.df_pooled),
        welch: t_test(p.t_welch, p.se_welch, p.df_welch),
    })
}

/// The closed-form intermediates of `anova_from_summary` — the
/// explain trace's named values (v1.0 Task 10, spec §6). `ss_w` is
/// `None` for the same inputs that yield an `Undefined { too_few }`
/// within SS: a group below two observations.
pub(crate) struct AnovaParts {
    /// `Σnᵢ` — the total observation count.
    pub n_tot: f64,
    /// The grand mean `Σnᵢ·meanᵢ / Σnᵢ`.
    pub grand: f64,
    /// `Σnᵢ·(meanᵢ − grand)²` — between-groups SS.
    pub ss_b: f64,
    /// `Σ(nᵢ−1)·vᵢ` — within-groups SS, when every group has one.
    pub ss_w: Option<f64>,
    /// `k − 1`.
    pub df_b: f64,
    /// `N − k`.
    pub df_w: f64,
    /// `N − 1`.
    pub df_t: f64,
}

impl AnovaParts {
    /// `ss_b/df_b`.
    pub(crate) fn ms_b(&self) -> f64 {
        self.ss_b / self.df_b
    }

    /// `ss_w/df_w` — NaN when no within SS exists.
    pub(crate) fn ms_w(&self) -> f64 {
        self.ss_w.map(|w| w / self.df_w).unwrap_or(f64::NAN)
    }

    /// `ms_b/ms_w` — the F ratio before the zero-variance guard.
    pub(crate) fn f_ratio(&self) -> f64 {
        self.ms_b() / self.ms_w()
    }
}

/// The intermediates of `anova_from_summary(groups)` — shared by the
/// result and the explain trace.
pub(crate) fn anova_parts(groups: &[GroupSummary]) -> AnovaParts {
    let n_tot: f64 = groups.iter().map(|g| g.n).sum::<u64>() as f64;
    let k = groups.len() as f64;
    let grand = neumaier_sum(groups.iter().map(|g| g.total())) / n_tot;
    let ss_b = neumaier_sum(
        groups
            .iter()
            .map(|g| g.n as f64 * (g.mean - grand) * (g.mean - grand)),
    );
    let ss_w = groups
        .iter()
        .all(|g| g.variance.is_some())
        .then(|| neumaier_sum(groups.iter().map(|g| g.ssq())));
    AnovaParts {
        n_tot,
        grand,
        ss_b,
        ss_w,
        df_b: k - 1.0,
        df_w: n_tot - k,
        df_t: n_tot - 1.0,
    }
}

/// One-way ANOVA from per-group summaries (spec §4.2). `ss_within`
/// needs a variance from every group; one `too_few` group poisons it
/// and everything downstream (`ms_within`, `f`, `p_value`,
/// `ss_total`), exactly as R's NA propagation. A sum of squares that
/// overflows f64's range is `Undefined { out_of_range }`, propagated
/// the same way.
pub(crate) fn anova_from_summary(groups: &[GroupSummary]) -> Anova {
    let p = anova_parts(groups);
    let (df_b, df_w, df_t) = (p.df_b, p.df_w, p.df_t);
    let ss_b = Estimate::from_f64(p.ss_b, UndefinedReason::OutOfRange);
    let ss_w = match p.ss_w {
        Some(w) => Estimate::from_f64(w, UndefinedReason::OutOfRange),
        None => Estimate::Undefined {
            reason: UndefinedReason::TooFew,
        },
    };
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

/// The closed-form intermediates of `bartlett_from_summary` — the
/// explain trace's named values (v1.0 Task 10, spec §6). `None` for
/// the same inputs that yield no Bartlett result.
pub(crate) struct BartlettParts {
    /// `N·log(v_pooled) − Σ nᵢ·log(vᵢ)`.
    pub numerator: f64,
    /// The small-sample correction `1 + (Σ1/nᵢ − 1/N)/(3(k−1))`.
    pub correction: f64,
    /// k − 1.
    pub df: f64,
}

impl BartlettParts {
    /// `numerator / correction` — the corrected chi-square.
    pub(crate) fn stat(&self) -> f64 {
        self.numerator / self.correction
    }
}

/// The intermediates of `bartlett_from_summary(groups)` — shared by
/// the result and the explain trace.
pub(crate) fn bartlett_parts(groups: &[GroupSummary]) -> Option<BartlettParts> {
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
    Some(BartlettParts {
        numerator,
        correction,
        df: k - 1.0,
    })
}

/// R's `bartlett.test(x, g)`: `None` when any group has fewer than 2
/// observations (R's error); otherwise the corrected χ² — NaN or
/// infinite on zero-variance input, hence `Undefined { zero_variance }`.
pub(crate) fn bartlett_from_summary(groups: &[GroupSummary]) -> Option<Bartlett> {
    let p = bartlett_parts(groups)?;
    let stat = Estimate::from_f64(p.stat(), UndefinedReason::ZeroVariance);
    let f_p = match stat {
        Estimate::Value(x) => Estimate::from_f64(chisq_sf(x, p.df), UndefinedReason::ZeroVariance),
        Estimate::Undefined { reason } => Estimate::Undefined { reason },
    };
    Some(Bartlett {
        statistic: stat,
        df: Estimate::Value(p.df),
        p_value: f_p,
    })
}
