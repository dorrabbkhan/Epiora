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

/// Bounded-work guard (owner decision D1, applied to the person-time
/// path): a stratum's binomial support is `m1 + 1` — the corpus's
/// `large_counts` fixture (m1 = 1.2e6) must compute, so the 1e6
/// *hypergeometric* support cap cannot apply here. The same
/// `MAX_EXACT_CONVOLUTION_WORK` bounds the total work instead:
/// `m1 + 1` coefficient evaluations per informative stratum plus the
/// `logmult_poly` multiply pairs `Σ_{i≥2} s_{<i}·s_i`. Pure
/// arithmetic on the inputs — runs before any allocation.
pub(crate) fn too_large(strata: &[PtStratum]) -> bool {
    let mut work: u128 = 0;
    let mut convolved: Option<u64> = None;
    for t in strata {
        if !informative(t) {
            continue;
        }
        // `m1` as u64 saturates at huge inputs; the work total is all
        // that matters there.
        let m1 = (libm::rint(t.a + t.b)).max(0.0) as u64;
        let s = m1.saturating_add(1);
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

/// `cmle_rr` — est plus mid-p and Fisher limits at `conf` (percent),
/// or `None` when the exact work exceeds the D1 cap (the caller emits
/// `{"undefined": "too_large"}` leaves). `NA` roots surface as NaN
/// inside the array, which the leaf writer records as
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
