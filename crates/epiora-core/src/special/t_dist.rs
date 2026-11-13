//! Student's t distribution (v1.0 spec §4.1): `pt`, `pt(lower.tail =
//! FALSE)` and `qt` through the incomplete beta —
//! `P(T > t) = ½·I_x(df/2, 1/2)` with `x = df/(df + t²)` — the relation
//! R's `pt`/`qt` reduce to.

use super::beta::{beta_quantile, reg_inc_beta};
use super::gamma::ln_gamma;
use super::root::{brent_root, RootError};

/// `I_{df/(df+t²)}(df/2, 1/2)` — twice `P(T > |t|)`. For `|t| ≳ √3`
/// `reg_inc_beta` takes its direct continued-fraction branch, so the
/// deep tail stays accurate down to ~1e-100; near `t = 0` it takes the
/// complement branch where nothing cancels.
fn tail(t: f64, df: f64) -> f64 {
    reg_inc_beta(df / (df + t * t), df * 0.5, 0.5)
}

/// `pt(t, df)`. `df` must be positive and finite; NaN inputs or a
/// non-positive/infinite `df` give NaN.
pub fn t_cdf(t: f64, df: f64) -> f64 {
    if t.is_nan() || !(df > 0.0 && df.is_finite()) {
        return f64::NAN;
    }
    if t == 0.0 {
        return 0.5;
    }
    let q = 0.5 * tail(t, df);
    if t > 0.0 {
        1.0 - q
    } else {
        q
    }
}

/// `pt(t, df, lower.tail = FALSE)`.
pub fn t_sf(t: f64, df: f64) -> f64 {
    if t.is_nan() || !(df > 0.0 && df.is_finite()) {
        return f64::NAN;
    }
    if t == 0.0 {
        return 0.5;
    }
    let q = 0.5 * tail(t, df);
    if t > 0.0 {
        q
    } else {
        1.0 - q
    }
}

/// `qt(p, df)` for `0 < p < 1`, `0 < df < ∞`: invert the beta relation
/// — `I_x(df/2, 1/2) = 2·min(p, 1−p)` at `x = df/(df + t²)`, so
/// `t = ±√(df·(1−x)/x)` — through `beta_quantile` (spec §4.1).
/// `Err(NotBracketed)` outside the domain, `Err(NoConvergence)` if the
/// root solve fails.
pub fn t_quantile(p: f64, df: f64) -> Result<f64, RootError> {
    if !(p > 0.0 && p < 1.0) || !(df > 0.0 && df.is_finite()) {
        return Err(RootError::NotBracketed);
    }
    if p == 0.5 {
        return Ok(0.0);
    }
    let (a, b, q) = (df * 0.5, 0.5, 2.0 * p.min(1.0 - p));
    let f = |x: f64| reg_inc_beta(x, a, b);
    let x = match beta_quantile(q, a, b) {
        // Brent's absolute bracket tolerance cannot resolve a root
        // x ≈ 1e-20 in linear space: accept the result only when the
        // residual confirms it, else solve on u = ln x.
        Some(x) if x > 0.0 && (f(x) - q).abs() <= q * 1e-6 => x,
        _ => log_solve(a, b, q)?,
    };
    let t = libm::sqrt(df * (1.0 - x) / x);
    Ok(if p < 0.5 { -t } else { t })
}

/// `I_x(a, b) = q` solved for `u = ln x`, bracketed from the small-`x`
/// asymptote `I_x ≈ x^a / (a·B(a,b))`, then widened until `f` changes
/// sign. The absolute tolerance on `u` becomes a ~1e-14 relative
/// tolerance on `x`.
fn log_solve(a: f64, b: f64, q: f64) -> Result<f64, RootError> {
    // Solve in v = u − u_est so the root sits near 0 (see
    // gamma_quantile.rs: brent_root's absolute tolerance is
    // unreachable where |u| ≳ 45).
    let u_est = (libm::log(q) + ln_gamma(a + 1.0) + ln_gamma(b) - ln_gamma(a + b)) / a;
    let f = |v: f64| reg_inc_beta(libm::exp(u_est + v), a, b) - q;
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
        if u_est + hi > 0.0 {
            // u = 0 gives I_1(a, b) = 1 > q — always the upper bracket.
            hi = -u_est;
            break;
        }
    }
    brent_root(f, lo, hi).map(|v| libm::exp(u_est + v))
}
