//! `qgamma`/`qchisq` (v1.0 spec §4.1): Brent's method on the
//! regularised lower incomplete gamma `reg_inc_gamma_lower`.
//!
//! The solve runs on `u = ln y` so the bracket's absolute tolerance of
//! 1e-14 becomes a ~1e-14 relative tolerance on `y` — the quantile can
//! be as small as 1e-60 (small shape, small `p`), where a linear-space
//! bracket could never meet the `iterative` class. The initial bracket
//! comes from the small-`x` asymptote `P(a, x) ≈ x^a / Γ(a+1)` and is
//! widened until `f` changes sign.

use super::gamma::ln_gamma;
use super::gamma_lower::{inc_gamma_upper, reg_inc_gamma_lower};
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
    // For shape ≲ 1e-300, P(a, x) ≈ x^a ≈ 1 on every positive f64
    // (a·ln x → 0): the quantile is smaller than the smallest
    // subnormal, and its f64 value is 0. Returning here also keeps the
    // expansions below finite — u_est ≈ −1e300 would need ~1e300
    // steps to reach its bound otherwise.
    if reg_inc_gamma_lower(shape, f64::from_bits(1)) >= p {
        return Ok(0.0);
    }
    // For p > 0.5 the solve runs on the upper tail `Q(a, y) = 1 − p`
    // instead: `P(a, y) − p` cancels in the last ulps once `1 − p` is
    // below ~1e-8, which is how `qgamma(1 − 1e-12, ·)` drifted off R.
    if p > 0.5 {
        return upper_tail_quantile(1.0 - p, shape, scale);
    }
    // Solve in v = u − u_est so the root sits near 0: brent_root's
    // absolute 1e-14 tolerance is unreachable where |u| ≳ 45 (the ulp
    // alone is wider), while in v-space it gives ~1e-14 relative on y.
    let u_est = (libm::log(p) + ln_gamma(shape + 1.0)) / shape;
    if !u_est.is_finite() {
        return Err(RootError::NotBracketed);
    }
    let f = |v: f64| reg_inc_gamma_lower(shape, libm::exp(u_est + v)) - p;
    // Each expansion doubles so a huge |u_est| is still reached in
    // ~1024 steps — linear steps could never terminate. f increases
    // with v, so on a bound crossing the bound point itself decides
    // whether a representable bracket exists.
    let mut lo = -4.0f64;
    while f(lo) >= 0.0 {
        let next = lo * 2.0;
        if u_est + next < -745.0 {
            let v_min = -745.0 - u_est;
            if f(v_min) >= 0.0 {
                return Err(RootError::NotBracketed);
            }
            lo = v_min;
            break;
        }
        lo = next;
    }
    let mut hi = 4.0f64;
    while f(hi) <= 0.0 {
        let next = hi * 2.0;
        if u_est + next > 709.0 {
            let v_max = 709.0 - u_est;
            if f(v_max) <= 0.0 {
                return Err(RootError::NotBracketed);
            }
            hi = v_max;
            break;
        }
        hi = next;
    }
    brent_root(f, lo, hi).map(|v| libm::exp(u_est + v) * scale)
}

/// `Q(a, y) = q` for `0 < q ≤ 0.5`, on `v = ln y − u_est` with
/// `u_est = ln(−ln q)` — the `Q ≈ y^{a−1}e^{−y}/Γ(a)` leading term.
/// `inc_gamma_upper` evaluates the small tail directly (R's `d·sum`
/// and `ppois_asymp` branches, plus the denormal log-space redo), so
/// `Q` keeps full precision to q ~ 1e-300. The root
/// is monotone decreasing; the bracket expansion mirrors the lower
/// solve's, doubling outward and clamping at the exp(-745)/exp(709)
/// bounds.
fn upper_tail_quantile(q: f64, shape: f64, scale: f64) -> Result<f64, RootError> {
    let u_est = libm::log(-libm::log(q));
    if !u_est.is_finite() {
        return Err(RootError::NotBracketed);
    }
    let f = |v: f64| inc_gamma_upper(shape, libm::exp(u_est + v)) - q;
    let mut lo = -4.0f64;
    while f(lo) <= 0.0 {
        let next = lo * 2.0;
        if u_est + next < -745.0 {
            let v_min = -745.0 - u_est;
            if f(v_min) <= 0.0 {
                return Err(RootError::NotBracketed);
            }
            lo = v_min;
            break;
        }
        lo = next;
    }
    let mut hi = 4.0f64;
    while f(hi) >= 0.0 {
        let next = hi * 2.0;
        if u_est + next > 709.0 {
            let v_max = 709.0 - u_est;
            if f(v_max) >= 0.0 {
                return Err(RootError::NotBracketed);
            }
            hi = v_max;
            break;
        }
        hi = next;
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
