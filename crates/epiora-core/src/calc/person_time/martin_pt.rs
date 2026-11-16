//! The person-time conditional-MLE rate ratio (Martin & Austin 1996),
//! `poly_strat_pt` + `stratified_cmle_pt_one` of
//! `fixtures/R/methods/tables.R` on the shared `counts::martin`
//! machinery: the conditional distribution of a stratum's exposed
//! case count is binomial, so the coefficient polynomial is
//! `C(m1, k)·(n0/n1)^(m1−k)` instead of the 2×2's hypergeometric.

use super::super::counts::{martin, shared};
use crate::special::ln_gamma;

/// One person-time stratum `[[a, b], [n1, n0]]` — exposed/unexposed
/// cases and person-time — as four reals (the `strata` cell kind is
/// `Real`; `m1` is rounded to a count as `poly_strat_pt` does).
pub(crate) struct PtStratum {
    /// Exposed cases.
    pub a: f64,
    /// Unexposed cases.
    pub b: f64,
    /// Exposed person-time.
    pub n1: f64,
    /// Unexposed person-time.
    pub n0: f64,
}

/// `oe_cells_pt` informativeness: the stratum carries no conditional
/// term when both `a·n0` and `b·n1` are 0.
fn informative(s: &PtStratum) -> bool {
    s.a * s.n0 != 0.0 || s.b * s.n1 != 0.0
}

/// `lchoose(m, k)` via `ln_gamma` — exact enough at the
/// `iterative` tolerance for the `m1` values the work cap admits
/// (the hypergeometric path needs `dhyper`'s `stirlerr`/`bd0`
/// composition at much larger margins).
fn lchoose(m: u64, k: u64) -> f64 {
    ln_gamma((m + 1) as f64) - ln_gamma((k + 1) as f64) - ln_gamma((m - k + 1) as f64)
}

/// `poly_strat_pt` — log coefficients `C(m1,k)·c0^(m1−k)` over
/// `k = 0..=m1` with `c0 = n0/n1`, max-normalised. Zero person-time
/// on one arm gives `c0` 0 or ∞ and a single-point mass; `m1` is
/// `round(a + b)` (R's `round`, half-to-even → `rint`).
fn poly_strat_pt(t: &PtStratum) -> Option<martin::Poly> {
    if !informative(t) {
        return None;
    }
    let m1 = libm::rint(t.a + t.b) as u64;
    let c0 = t.n0 / t.n1;
    let mut lp: Vec<f64> = if c0 == 0.0 {
        let mut v = vec![f64::NEG_INFINITY; m1 as usize + 1];
        v[m1 as usize] = 0.0;
        v
    } else if !c0.is_finite() {
        let mut v = vec![f64::NEG_INFINITY; m1 as usize + 1];
        v[0] = 0.0;
        v
    } else {
        let lc = libm::log(c0);
        (0..=m1)
            .map(|k| lchoose(m1, k) + (m1 - k) as f64 * lc)
            .collect()
    };
    let mx = lp.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    for v in &mut lp {
        *v -= mx;
    }
    Some(martin::Poly {
        lp,
        min_a: 0,
        max_a: m1,
    })
}

/// `m1 = rint(a + b)` as `poly_strat_pt` computes it. The `u64` cast
/// saturates at huge inputs — which are past `MAX_PT_SUPPORT`
/// anyway, so the saturating cast cannot hide an over-limit stratum.
fn m1(t: &PtStratum) -> u64 {
    (libm::rint(t.a + t.b)).max(0.0) as u64
}

/// Bounded-work guard (owner decisions D1, T1 and T4, applied to the
/// person-time path): a stratum's binomial support is `m1 + 1` — the
/// corpus's `large_counts` fixture (m1 = 1.2e6) must compute, so the
/// 1e6 *hypergeometric* support cap cannot apply here. Three caps
/// instead, all pure arithmetic on the inputs before any
/// allocation: T1's per-stratum `m1 ≤ MAX_PT_SUPPORT` (2e6), T4's
/// `Σm1 ≤ MAX_PT_TOTAL_SUPPORT` (2e6) over the informative strata
/// of the slice — so a multi-stratum slice trips it at the same
/// total as `call_too_large` — and the same
/// `MAX_EXACT_CONVOLUTION_WORK` (1e8) on total work — `m1 + 1`
/// coefficient evaluations per informative stratum plus the
/// `logmult_poly` multiply pairs `Σ_{i≥2} s_{<i}·s_i`.
pub(crate) fn too_large(strata: &[PtStratum]) -> bool {
    let mut work: u128 = 0;
    let mut total: u64 = 0;
    let mut convolved: Option<u64> = None;
    for t in strata {
        if !informative(t) {
            continue;
        }
        let m1 = m1(t);
        if m1 > shared::MAX_PT_SUPPORT {
            return true;
        }
        total = total.saturating_add(m1);
        if total > shared::MAX_PT_TOTAL_SUPPORT {
            return true;
        }
        let s = m1 + 1;
        work += s as u128;
        convolved = Some(match convolved {
            None => s,
            Some(c) => {
                work += c as u128 * s as u128;
                c.saturating_add(s).saturating_sub(1)
            }
        });
    }
    work > shared::MAX_EXACT_CONVOLUTION_WORK as u128
}

/// The T4 whole-call cap (owner decision on PR #78): `rate_compare`
/// computes each stratum's `cmle_rr` on its own single-stratum
/// slice, so `too_large` never sees the call's total — this check
/// sums `m1` over the call's informative strata only, with no
/// convolution-work term (the strata are never convolved together).
/// When it trips, every stratum's and the crude `cmle_rr.*` leaves
/// are `{"undefined": "too_large"}`. Pure arithmetic, no allocation.
pub(crate) fn call_too_large(strata: &[PtStratum]) -> bool {
    let mut total: u64 = 0;
    for t in strata {
        if !informative(t) {
            continue;
        }
        total = total.saturating_add(m1(t));
        if total > shared::MAX_PT_TOTAL_SUPPORT {
            return true;
        }
    }
    false
}

/// `cmle_rr` — est plus mid-p and Fisher limits at `conf` (percent),
/// or `None` when the support or work exceeds the caps (the caller
/// emits `{"undefined": "too_large"}` leaves). `NA` roots surface as
/// NaN inside the array, which the leaf writer records as
/// `{"undefined": "out_of_range"}` — the corpus's null leaves.
pub(crate) fn cmle_rr(strata: &[PtStratum], conf: f64) -> Option<[f64; 5]> {
    if too_large(strata) {
        return None;
    }
    let s = martin::convolve(
        strata
            .iter()
            .filter_map(|t| poly_strat_pt(t).map(|p| (t.a, p))),
    );
    let (a, b, n1, n0) = strata.iter().fold((0.0, 0.0, 0.0, 0.0), |m, t| {
        (m.0 + t.a, m.1 + t.b, m.2 + t.n1, m.3 + t.n0)
    });
    let approx = ((a / n1) / (b / n0)).max(1e-8);
    Some(martin::cmle(&s, approx, conf))
}
