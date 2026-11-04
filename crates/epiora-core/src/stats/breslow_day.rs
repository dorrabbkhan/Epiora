//! `breslow_day` — Breslow–Day test on homogeneity of odds ratios with
//! Tarone's correction, a port of `DescTools::BreslowDayTest(x,
//! correct = TRUE)` (spec §3.5). Each stratum's expected `a` cell is
//! the quadratic root inside that stratum's support under the common
//! MH odds ratio; the degrees of freedom are `included strata − 1`, so
//! the test is undefined with fewer than two kept strata.

use super::estimate::{Estimate, UndefinedReason};
use super::two_by_two::Table2x2;
use crate::special::chisq_cdf;
use serde::Serialize;

/// `breslow_day_tarone.*` of the stratified fixtures.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct BreslowDayTarone {
    /// Tarone-corrected `X²`.
    pub statistic: Estimate,
    /// Included strata − 1.
    pub df: Estimate,
    /// `1 - pchisq(statistic, df)`, as `DescTools::BreslowDayTest`
    /// computes it.
    pub p_value: Estimate,
}

impl BreslowDayTarone {
    /// Every leaf `Undefined { reason }`.
    pub(crate) fn all_undefined(reason: UndefinedReason) -> BreslowDayTarone {
        let u = || Estimate::Undefined { reason };
        BreslowDayTarone {
            statistic: u(),
            df: u(),
            p_value: u(),
        }
    }
}

/// The root of `c2·E² + c1·E + c0 = 0` in `(0, min(n1, m1)]`, as
/// `Re(polyroot(c(c0, c1, c2)))` filtered by `0 < sol <= min(n1, m1)`:
/// a negative discriminant collapses to the real part `−c1/(2·c2)`,
/// matching R. `None` when no root lies in the support.
fn expected_cell(n1: f64, m1: f64, m0: f64, or: f64) -> Option<f64> {
    // DescTools: coef = c(-N1·M1·OR, M0 − N1 + OR·(N1 + M1), 1 − OR).
    let c0 = -n1 * m1 * or;
    let c1 = m0 - n1 + or * (m1 + n1);
    let c2 = 1.0 - or;
    let in_support = |e: f64| e > 0.0 && e <= n1.min(m1);
    if c2 == 0.0 {
        // OR = 1 collapses the quadratic to a line.
        return (c1 != 0.0).then_some(-c0 / c1).filter(|e| in_support(*e));
    }
    let disc = c1 * c1 - 4.0 * c2 * c0;
    let (r1, r2) = if disc >= 0.0 {
        let s = libm::sqrt(disc);
        ((-c1 + s) / (2.0 * c2), (-c1 - s) / (2.0 * c2))
    } else {
        let re = -c1 / (2.0 * c2);
        (re, re)
    };
    [r1, r2].into_iter().find(|e| in_support(*e))
}

/// Breslow–Day–Tarone over the kept `strata` (each of total > 0) under
/// the common MH odds ratio `or_mh`. Fewer than two strata, an
/// undefined `or_mh`, or a root outside a stratum's support make every
/// leaf `Undefined` — never NaN.
pub(crate) fn breslow_day_tarone(kept: &[Table2x2], or_mh: &Estimate) -> BreslowDayTarone {
    if kept.len() < 2 {
        // The df would be 0; EmptyTable when there is nothing at all.
        let reason = if kept.is_empty() {
            UndefinedReason::EmptyTable
        } else {
            UndefinedReason::ZeroMargin
        };
        return BreslowDayTarone::all_undefined(reason);
    }
    let or = match or_mh {
        Estimate::Value(v) => *v,
        Estimate::Undefined { reason } => return BreslowDayTarone::all_undefined(*reason),
    };

    let (mut x2, mut sum_a, mut sum_e, mut sum_var) = (0.0, 0.0, 0.0, 0.0);
    for t in kept {
        let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
        let (n1, n0, m1, m0) = (a + b, c + d, a + c, b + d);
        let e = match expected_cell(n1, m1, m0, or) {
            Some(e) => e,
            None => return BreslowDayTarone::all_undefined(UndefinedReason::NotBracketed),
        };
        let bt = n1 - e;
        let ct = m1 - e;
        // DescTools: tildedj = mj[2] − tildecj = N0 − (M1 − E).
        let dt = n0 - ct;
        let var = 1.0 / (1.0 / e + 1.0 / bt + 1.0 / ct + 1.0 / dt);
        x2 += (a - e) * (a - e) / var;
        sum_a += a;
        sum_e += e;
        sum_var += var;
    }
    let x2t = x2 - (sum_a - sum_e) * (sum_a - sum_e) / sum_var;
    let df = (kept.len() - 1) as f64;
    BreslowDayTarone {
        statistic: Estimate::from_f64(x2t, UndefinedReason::ZeroCell),
        df: Estimate::Value(df),
        p_value: if x2t.is_finite() {
            Estimate::from_f64(
                (1.0 - chisq_cdf(x2t, df)).clamp(0.0, 1.0),
                UndefinedReason::ZeroCell,
            )
        } else {
            Estimate::Undefined {
                reason: UndefinedReason::ZeroCell,
            }
        },
    }
}
