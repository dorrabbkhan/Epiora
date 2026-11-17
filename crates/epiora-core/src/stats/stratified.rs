//! `stratified` — Mantel–Haenszel statistics over k 2×2 strata plus
//! the Breslow–Day–Tarone homogeneity test (spec §3.5), serializing to
//! the Task 2 `stratified_two_by_two` fixture layout. The MH parts are
//! a port of the 2x2xK branch of `stats::mantelhaen.test` (the RBG
//! confidence interval included) and the risk ratio is the direct
//! Greenland–Robins formula `epiR` implements for `RR.mh.wald`.
//! Strata of total 0 are excluded from every sum and listed in
//! `excluded_strata`; with no included stratum every value is
//! `Undefined { EmptyTable }`, never NaN.

use super::breslow_day::{breslow_day_tarone, BreslowDayTarone};
use super::estimate::{Ci, Estimate, UndefinedReason};
use super::two_by_two::{two_by_two, ChiSquare, Table2x2, TwoByTwoStats};
use crate::special::{chisq_sf, normal_quantile};
use serde::Serialize;

/// Serialized stratified 2×2 statistics — the `expected` key layout of
/// the `stratified` fixtures.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct StratifiedStats {
    /// `TwoByTwoStats` of the summed kept table.
    pub crude: TwoByTwoStats,
    /// `mantel_haenszel.*`.
    pub mantel_haenszel: MantelHaenszel,
    /// `breslow_day_tarone.*`.
    pub breslow_day_tarone: BreslowDayTarone,
    /// 0-based positions of strata whose total is 0.
    pub excluded_strata: Vec<usize>,
}

/// `mantel_haenszel.*` — the pooled common-effect estimates and the
/// two chi-squares of `mantelhaen.test`.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MantelHaenszel {
    /// Common odds ratio `sum(ad/n) / sum(bc/n)` with its
    /// Robins–Breslow–Greenland 95% CI.
    pub odds_ratio: MhEstimate,
    /// Common risk ratio `sum(a·N0/n) / sum(c·N1/n)` with its
    /// Greenland–Robins 95% CI.
    pub risk_ratio: MhEstimate,
    /// `chi_square.*` — uncorrected and continuity-corrected.
    pub chi_square: MhChiSquare,
}

/// One MH point estimate and its 95% CI.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MhEstimate {
    /// The pooled estimate.
    pub estimate: Estimate,
    /// `ci_95.*`.
    pub ci_95: Ci,
}

/// `chi_square.*` of the stratified result.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct MhChiSquare {
    /// `DELTA² / sum(N1·N0·M1·M0 / n²(n−1))`.
    pub uncorrected: ChiSquare,
    /// `(|DELTA| − 0.5)²` over the same denominator when
    /// `|DELTA| ≥ 0.5`, the uncorrected value otherwise.
    pub corrected: ChiSquare,
}

impl MantelHaenszel {
    /// Every leaf `Undefined { reason }`.
    fn all_undefined(reason: UndefinedReason) -> MantelHaenszel {
        let u = || Estimate::Undefined { reason };
        let est = || MhEstimate {
            estimate: u(),
            ci_95: Ci {
                lower: u(),
                upper: u(),
            },
        };
        let stat = || ChiSquare {
            statistic: u(),
            p_value: u(),
        };
        MantelHaenszel {
            odds_ratio: est(),
            risk_ratio: est(),
            chi_square: MhChiSquare {
                uncorrected: stat(),
                corrected: stat(),
            },
        }
    }
}

fn z_975() -> f64 {
    normal_quantile(0.975).unwrap_or(1.959963984540054)
}

fn chi(statistic: f64) -> ChiSquare {
    ChiSquare {
        statistic: Estimate::from_f64(statistic, UndefinedReason::ZeroMargin),
        p_value: if statistic.is_finite() {
            Estimate::from_f64(
                chisq_sf(statistic, 1.0).clamp(0.0, 1.0),
                UndefinedReason::ZeroMargin,
            )
        } else {
            Estimate::Undefined {
                reason: UndefinedReason::ZeroMargin,
            }
        },
    }
}

/// Per-stratum margins of `t` as f64: (N1, N0, M1, M0, n).
fn margins(t: &Table2x2) -> (f64, f64, f64, f64, f64) {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    (a + b, c + d, a + c, b + d, a + b + c + d)
}

/// The closed-form intermediates `stratified` accumulates over the
/// kept strata — the explain trace's named values (v1.0 Task 10,
/// spec §6). The run path calls `stratified_parts` and the explain
/// path reuses it.
pub(crate) struct StratParts {
    /// `sum(a − N1·M1/n)` over kept strata.
    pub delta: f64,
    /// `sum(N1·N0·M1·M0 / (n²·(n−1)))`.
    pub var_delta: f64,
    /// `sum(a·d/n)`.
    pub s_diag: f64,
    /// `sum(b·c/n)`.
    pub s_offd: f64,
    /// `sum((a+d)·a·d/n²)` — first RBG variance term.
    pub rbg_diag: f64,
    /// `sum(((a+d)·b·c + (b+c)·a·d)/n²)` — second RBG term.
    pub rbg_cross: f64,
    /// `sum((b+c)·b·c/n²)` — third RBG term.
    pub rbg_offd: f64,
    /// `sum(a·N0/n)` — MH risk-ratio numerator.
    pub rr_num: f64,
    /// `sum(c·N1/n)` — MH risk-ratio denominator.
    pub rr_den: f64,
    /// `sum(M1·N1·N0/n² − a·c/n)` — Greenland–Robins numerator.
    pub gr_num: f64,
}

/// The accumulated sums of `stratified(kept)` (spec §6).
pub(crate) fn stratified_parts(kept: &[Table2x2]) -> StratParts {
    let (mut delta, mut var_delta) = (0.0, 0.0);
    let (mut s_diag, mut s_offd) = (0.0, 0.0);
    let (mut rbg_diag, mut rbg_cross, mut rbg_offd) = (0.0, 0.0, 0.0);
    let (mut rr_num, mut rr_den, mut gr_num) = (0.0, 0.0, 0.0);
    for t in kept {
        let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
        let (n1, n0, m1, m0, n) = margins(t);
        // mantelhaen.test: DELTA = sum(a − N1·M1/n) with variance
        // sum(N1·N0·M1·M0 / (n²·(n−1))).
        delta += a - n1 * m1 / n;
        var_delta += n1 * n0 * m1 * m0 / (n * n * (n - 1.0));
        // Common OR: sum(ad/n) / sum(bc/n) with the RBG variance.
        s_diag += a * d / n;
        s_offd += b * c / n;
        rbg_diag += (a + d) * a * d / (n * n);
        rbg_cross += ((a + d) * b * c + (b + c) * a * d) / (n * n);
        rbg_offd += (b + c) * b * c / (n * n);
        // Greenland–Robins MH risk ratio.
        rr_num += a * n0 / n;
        rr_den += c * n1 / n;
        gr_num += m1 * n1 * n0 / (n * n) - a * c / n;
    }
    StratParts {
        delta,
        var_delta,
        s_diag,
        s_offd,
        rbg_diag,
        rbg_cross,
        rbg_offd,
        rr_num,
        rr_den,
        gr_num,
    }
}

/// The estimates `stratified` derives from `StratParts` — the
/// explain trace's named values (v1.0 Task 10, spec §6).
pub(crate) struct StratDerived {
    /// `delta²/var_delta` — the uncorrected MH chi-square.
    pub x2: f64,
    /// `0.5` when `|delta| ≥ 0.5`, else 0 — the continuity correction.
    pub yates: f64,
    /// `(|delta| − yates)²/var_delta` — the corrected chi-square.
    pub x2c: f64,
    /// `s_diag/s_offd` — the common odds ratio.
    pub or: f64,
    /// The Robins–Breslow–Greenland SD of `log or`.
    pub sd_or: f64,
    /// `rr_num/rr_den` — the Greenland–Robins MH risk ratio.
    pub rr: f64,
    /// `sqrt(gr_num/(rr_num·rr_den))` — the log-RR SE.
    pub se_rr: f64,
}

/// The derived estimates of `stratified_parts(kept)` — shared by the
/// result and the explain trace.
pub(crate) fn stratified_derived(p: &StratParts) -> StratDerived {
    let x2 = p.delta * p.delta / p.var_delta;
    let yates = if p.delta.abs() >= 0.5 { 0.5 } else { 0.0 };
    StratDerived {
        x2,
        yates,
        x2c: (p.delta.abs() - yates) * (p.delta.abs() - yates) / p.var_delta,
        or: p.s_diag / p.s_offd,
        sd_or: libm::sqrt(
            p.rbg_diag / (2.0 * p.s_diag * p.s_diag)
                + p.rbg_cross / (2.0 * p.s_diag * p.s_offd)
                + p.rbg_offd / (2.0 * p.s_offd * p.s_offd),
        ),
        rr: p.rr_num / p.rr_den,
        se_rr: libm::sqrt(p.gr_num / (p.rr_num * p.rr_den)),
    }
}

/// All stratified statistics of `strata` (spec §3.5). Strata of total
/// 0 are excluded and listed; the crude table is the sum of the kept
/// strata. Nothing panics and no NaN is ever emitted.
pub fn stratified(strata: &[Table2x2]) -> StratifiedStats {
    let mut excluded_strata = Vec::new();
    let mut kept: Vec<Table2x2> = Vec::with_capacity(strata.len());
    let mut sum = Table2x2 {
        a: 0,
        b: 0,
        c: 0,
        d: 0,
    };
    for (i, t) in strata.iter().enumerate() {
        if t.a
            .saturating_add(t.b)
            .saturating_add(t.c)
            .saturating_add(t.d)
            == 0
        {
            excluded_strata.push(i);
        } else {
            kept.push(*t);
            sum.a = sum.a.saturating_add(t.a);
            sum.b = sum.b.saturating_add(t.b);
            sum.c = sum.c.saturating_add(t.c);
            sum.d = sum.d.saturating_add(t.d);
        }
    }
    let crude = two_by_two(&sum);
    if kept.is_empty() {
        return StratifiedStats {
            crude,
            mantel_haenszel: MantelHaenszel::all_undefined(UndefinedReason::EmptyTable),
            breslow_day_tarone: BreslowDayTarone::all_undefined(UndefinedReason::EmptyTable),
            excluded_strata,
        };
    }

    let z = z_975();
    let p = stratified_parts(&kept);
    let dv = stratified_derived(&p);
    let chi_square = MhChiSquare {
        uncorrected: chi(dv.x2),
        corrected: chi(dv.x2c),
    };

    // mantelhaen.test estimate and RBG CI: CI = OR · exp(±z·sd).
    let (or, sd) = (dv.or, dv.sd_or);
    let odds_ratio = MhEstimate {
        estimate: Estimate::from_f64(or, UndefinedReason::ZeroCell),
        ci_95: Ci {
            lower: Estimate::from_f64(or * libm::exp(-z * sd), UndefinedReason::ZeroCell),
            upper: Estimate::from_f64(or * libm::exp(z * sd), UndefinedReason::ZeroCell),
        },
    };

    // Greenland–Robins: RR = sum(a·N0/n)/sum(c·N1/n),
    // var(log RR) = sum(M1·N1·N0/n² − ac/n) / (sum(a·N0/n)·sum(c·N1/n)).
    let rr = dv.rr;
    let half = z * dv.se_rr;
    let log_rr = libm::log(rr);
    let risk_ratio = MhEstimate {
        estimate: Estimate::from_f64(rr, UndefinedReason::ZeroCell),
        ci_95: Ci {
            lower: Estimate::from_f64(libm::exp(log_rr - half), UndefinedReason::ZeroCell),
            upper: Estimate::from_f64(libm::exp(log_rr + half), UndefinedReason::ZeroCell),
        },
    };

    StratifiedStats {
        crude,
        mantel_haenszel: MantelHaenszel {
            odds_ratio,
            risk_ratio,
            chi_square,
        },
        breslow_day_tarone: breslow_day_tarone(
            &kept,
            &Estimate::from_f64(or, UndefinedReason::ZeroCell),
        ),
        excluded_strata,
    }
}
