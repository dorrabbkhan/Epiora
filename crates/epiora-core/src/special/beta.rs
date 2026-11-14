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
/// precision (needed by `f_sf` and the `pbinom` inside `qbinom`).
pub(crate) fn reg_inc_beta_upper(x: f64, a: f64, b: f64) -> f64 {
    inc_beta(x, a, b, true)
}

/// `pbeta(x, a, b, lower.tail)` — the `pbeta` wrapper + `pbeta_raw`
/// entry checks (pbeta.c L41-58) restricted to `log_p = FALSE`.
fn inc_beta(x: f64, a: f64, b: f64, upper: bool) -> f64 {
    if x.is_nan() || a.is_nan() || b.is_nan() {
        return f64::NAN;
    }
    if a < 0.0 || b < 0.0 {
        return f64::NAN;
    }
    if x >= 1.0 {
        return if upper { 0.0 } else { 1.0 };
    }
    // pbeta_raw L46-56: the point-mass limit cases, in R's order.
    if a == 0.0 || b == 0.0 || !a.is_finite() || !b.is_finite() {
        if a == 0.0 && b == 0.0 {
            return 0.5; // point mass 1/2 at each of {0, 1}
        }
        if a == 0.0 || a / b == 0.0 {
            return if upper { 0.0 } else { 1.0 }; // point mass at 0
        }
        if b == 0.0 || b / a == 0.0 {
            return if upper { 1.0 } else { 0.0 }; // point mass at 1
        }
        // a = b = Inf: point mass at 1/2.
        return if x < 0.5 {
            if upper {
                1.0
            } else {
                0.0
            }
        } else if upper {
            0.0
        } else {
            1.0
        };
    }
    if x <= 0.0 {
        return if upper { 1.0 } else { 0.0 };
    }
    // L62: x1 = 0.5 − x + 0.5 — what bratio receives; the two-step
    // form stays exact where 1 − x would round twice.
    let y = 0.5 - x + 0.5;
    if a.min(b) <= 1.0 {
        // bratio's first block: swap so x0 <= 0.5, compute whichever
        // tail its series produce directly, un-swap at the end.
        let (w, w1) = if x > 0.5 {
            let (ws, w1s) = beta_small::bratio_small(y, x, b, a);
            (w1s, ws)
        } else {
            beta_small::bratio_small(x, y, a, b)
        };
        return if upper { w1 } else { w };
    }
    // a, b > 1: bratio's L30 lambda dispatch (beta_large).
    beta_large::inc_beta_large(x, y, a, b, upper)
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
