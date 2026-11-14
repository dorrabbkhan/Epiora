//! The regularised incomplete beta I_x(a, b) — R's `pbeta`
//! (toms708.c `bratio` dispatch): `beta_small` for `min(a,b) <= 1`,
//! the L30 lambda dispatch (`bup`/`bgrat`/`bfrac`/`basym`) in
//! `beta_large` for `a, b > 1`. This replaces the Numerical Recipes
//! continued fraction, whose `exp(lgamma(a+b) − lgamma(·))` prefactor
//! cancels every significant digit once a or b ≳ 1e7 (NaN, >1 and
//! negative tail values at df ~ 1e15+) and whose CF drifts ~1e-4 off
//! R in mixed small/huge-parameter regions. `qbeta` is Brent on the
//! same function (spec §3.4).

use super::beta_large;
use super::beta_small;
use super::root::brent_root;

/// Regularised incomplete beta I_x(a, b) = B_x(a,b)/B(a,b).
pub fn reg_inc_beta(x: f64, a: f64, b: f64) -> f64 {
    inc_beta(x, a, b, false)
}

/// `1 − I_x(a, b)` computed as R's `pbeta(x, a, b, lower.tail =
/// FALSE)` does — the tail is evaluated directly instead of
/// subtracting a value ≈ 1, so probabilities near 1 keep full
/// precision (needed by `f_sf` and `binomial_cdf`).
pub(crate) fn reg_inc_beta_upper(x: f64, a: f64, b: f64) -> f64 {
    inc_beta(x, a, b, true)
}

fn inc_beta(x: f64, a: f64, b: f64, upper: bool) -> f64 {
    if x.is_nan() || a.is_nan() || b.is_nan() || a < 0.0 || b < 0.0 {
        return f64::NAN;
    }
    if a == 0.0 && b == 0.0 {
        return f64::NAN; // bratio's ierr 8
    }
    if !(x > 0.0 && x < 1.0) {
        if x <= 0.0 {
            return if upper { 1.0 } else { 0.0 };
        }
        return if upper { 0.0 } else { 1.0 };
    }
    if a == 0.0 {
        return if upper { 0.0 } else { 1.0 };
    }
    if b == 0.0 {
        return if upper { 1.0 } else { 0.0 };
    }
    if a.min(b) <= 1.0 {
        // bratio's first block: swap so x0 <= 0.5, compute whichever
        // tail its series produce directly, un-swap at the end.
        let (w, w1) = if x > 0.5 {
            let (ws, w1s) = beta_small::bratio_small(1.0 - x, b, a);
            (w1s, ws)
        } else {
            beta_small::bratio_small(x, a, b)
        };
        return if upper { w1 } else { w };
    }
    // a, b > 1: bratio's L30 lambda dispatch (beta_large).
    beta_large::inc_beta_large(x, a, b, upper)
}

/// `qbeta(p, a, b)` — Brent's method (abs tol 1e-14) on
/// `reg_inc_beta(x, a, b) − p` (spec §3.4). `None` unless
/// `0 < p < 1` and `a, b > 0` and finite.
pub fn beta_quantile(p: f64, a: f64, b: f64) -> Option<f64> {
    if !(0.0 < p && p < 1.0) || !(a > 0.0 && a.is_finite()) || !(b > 0.0 && b.is_finite()) {
        return None;
    }
    brent_root(|x| reg_inc_beta(x, a, b) - p, 0.0, 1.0).ok()
}
