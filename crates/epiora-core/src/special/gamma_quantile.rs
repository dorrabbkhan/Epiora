//! `qgamma`/`qchisq` (v1.0 spec §4.1): Brent's method on the
//! regularised lower incomplete gamma `reg_inc_gamma_lower`.
//!
//! The solve runs on `u = ln y` so the bracket's absolute tolerance of
//! 1e-14 becomes a ~1e-14 relative tolerance on `y` — the quantile can
//! be as small as 1e-60 (small shape, small `p`), where a linear-space
//! bracket could never meet the `iterative` class. The initial bracket
//! comes from the small-`x` asymptote `P(a, x) ≈ x^a / Γ(a+1)` and is
//! widened until `f` changes sign.

use super::gamma::{ln_gamma, reg_inc_gamma_lower};
use super::root::{brent_root, RootError};

/// `qgamma(p, shape, scale = scale)` for `0 < p < 1`, positive finite
/// shape and scale. `Err(NotBracketed)` outside the domain or when no
/// representable bracket exists, `Err(NoConvergence)` if Brent does not
/// converge.
pub fn gamma_quantile(p: f64, shape: f64, scale: f64) -> Result<f64, RootError> {
    if !(p > 0.0 && p < 1.0)
        || !(shape > 0.0 && shape.is_finite())
        || !(scale > 0.0 && scale.is_finite())
    {
        return Err(RootError::NotBracketed);
    }
    // Solve in v = u − u_est so the root sits near 0: brent_root's
    // absolute 1e-14 tolerance is unreachable where |u| ≳ 45 (the ulp
    // alone is wider), while in v-space it gives ~1e-14 relative on y.
    let u_est = (libm::log(p) + ln_gamma(shape + 1.0)) / shape;
    let f = |v: f64| reg_inc_gamma_lower(shape, libm::exp(u_est + v)) - p;
    // f(−745) = −p < 0 always (exp underflows past it, P → 0), so the
    // lower bracket always exists unless the root itself underflows.
    let mut lo = -4.0f64;
    while f(lo) >= 0.0 {
        lo -= 8.0;
        if u_est + lo < -745.0 {
            return Err(RootError::NotBracketed);
        }
    }
    let mut hi = 4.0f64;
    while f(hi) <= 0.0 {
        hi += 4.0;
        if u_est + hi > 709.0 {
            return Err(RootError::NotBracketed);
        }
    }
    brent_root(f, lo, hi).map(|v| libm::exp(u_est + v) * scale)
}

/// `qchisq(p, df)` = `qgamma(p, df/2, scale = 2)` (R's identity). As
/// in R, `df = 0` is the point mass at 0 and quantiles are 0.
pub fn chisq_quantile(p: f64, df: f64) -> Result<f64, RootError> {
    if !(p > 0.0 && p < 1.0) || !(df >= 0.0 && df.is_finite()) {
        return Err(RootError::NotBracketed);
    }
    if df == 0.0 {
        return Ok(0.0);
    }
    gamma_quantile(p, df * 0.5, 2.0)
}
