//! Student's t distribution (v1.0 spec §4.1): `pt`, `pt(lower.tail =
//! FALSE)` and `qt` through the incomplete beta, in the form R's `pt.c`
//! uses since R 2.7: with `nx = (df + t²)/df` evaluated as
//! `1 + (t/df)·t` (so `t²` can exceed `df` without overflow),
//! `P(T > |t|) = ½·val` where `val` is `pbeta(t²/(df+t²), ½, df/2,
//! lower.tail = FALSE)` for `df > t²` and `pbeta(1/nx, df/2, ½)` for
//! `df ≤ t²` — each direction chosen so the beta argument stays below
//! ½ and never rounds to 1. When `nx > 1e100` the argument would
//! underflow, so `val` is the A&S 26.5.4 log-space form instead.
//! `qt` inverts the same relation through `beta_quantile`; for
//! `df > 4e5`, where `x = df/(df + t²)` is within a few ulps of 1 for
//! every ordinary `t`, it instead solves `½·val(t) = min(p, 1−p)` on
//! `v = ln t` — the same function `pt` evaluates, so no approximation
//! error is introduced anywhere in the f64 df range.

use super::beta::{beta_quantile, reg_inc_beta, reg_inc_beta_upper};
use super::gamma::{ln_gamma, stirlerr};
use super::normal::normal_quantile;
use super::root::{brent_root, RootError};

/// ln √(2π) — used by `lbeta`'s both-large branch.
const M_LN_SQRT_2PI: f64 = 0.91893853320467274178032973640561764;

/// df above which `t_quantile` solves `val(t) = 2·min(p, 1−p)`
/// instead of inverting the beta CDF on `x = df/(df + t²)`: for
/// `df > 4e5` that `x` is within ~1e-11 of 1 at every `|t| ≤ ~10`,
/// so `t = √(df(1−x)/x)` cannot hold the needed precision in f64 —
/// while `val` itself stays accurate (it is computed in the
/// small-argument direction). This is the cutoff R's `pt.c` used
/// historically for its large-df branch.
const LARGE_DF: f64 = 4e5;

/// `log B(a, b)` — R's `lbeta.c` (Fullerton): `lgamma` differences
/// lose every digit once `max(a, b)` is large, so the big branches
/// keep the Stirling corrections separate.
fn lbeta(a: f64, b: f64) -> f64 {
    let (p, q) = if a < b { (a, b) } else { (b, a) };
    if p >= 10.0 {
        let corr = stirlerr(p) + stirlerr(q) - stirlerr(p + q);
        -0.5 * libm::log(q)
            + M_LN_SQRT_2PI
            + corr
            + (p - 0.5) * libm::log(p / (p + q))
            + q * libm::log1p(-p / (p + q))
    } else if q >= 10.0 {
        let corr = stirlerr(q) - stirlerr(p + q);
        ln_gamma(p) + corr + p - p * libm::log(p + q) + (q - 0.5) * libm::log1p(-p / (p + q))
    } else {
        ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
    }
}

/// `2·P(T > |t|)` — R `pt.c`'s `val`: `pbeta(t²/(df+t²), ½, df/2,
/// lower.tail = FALSE)` when `df > t²`, `pbeta(1/nx, df/2, ½)` when
/// `df ≤ t²`, and the A&S 26.5.4 log form when `nx` exceeds 1e100
/// (there `1/nx` underflows toward 0 and the beta prefactor loses
/// the tail; the same limit R uses).
fn tail_val(t: f64, df: f64) -> f64 {
    let nx = 1.0 + (t / df) * t;
    if nx > 1e100 {
        let lval = -0.5 * df * (2.0 * libm::log(libm::fabs(t)) - libm::log(df))
            - lbeta(df * 0.5, 0.5)
            - libm::log(df * 0.5);
        libm::exp(lval)
    } else if df > t * t {
        reg_inc_beta_upper(t * t / (df + t * t), 0.5, df * 0.5)
    } else {
        reg_inc_beta(1.0 / nx, df * 0.5, 0.5)
    }
}

/// Shared body of `t_cdf`/`t_sf` — R `pt.c`'s epilogue: `val/2` is
/// the probability on the `|t|` side, `x <= 0` swaps the tail, and
/// `0.5 − v + 0.5` returns `1 − v` without losing v's last bits.
fn pt_body(t: f64, df: f64, lower_tail: bool) -> f64 {
    if t.is_nan() || !(df > 0.0 && df.is_finite()) {
        return f64::NAN;
    }
    if t == 0.0 {
        return 0.5;
    }
    if !t.is_finite() {
        return if t < 0.0 {
            if lower_tail {
                0.0
            } else {
                1.0
            }
        } else if lower_tail {
            1.0
        } else {
            0.0
        };
    }
    let v = 0.5 * tail_val(t, df);
    let lt = if t <= 0.0 { !lower_tail } else { lower_tail };
    if lt {
        0.5 - v + 0.5
    } else {
        v
    }
}

/// `pt(t, df)`. `df` must be positive and finite; NaN inputs or a
/// non-positive/infinite `df` give NaN.
pub fn t_cdf(t: f64, df: f64) -> f64 {
    pt_body(t, df, true)
}

/// `pt(t, df, lower.tail = FALSE)`.
pub fn t_sf(t: f64, df: f64) -> f64 {
    pt_body(t, df, false)
}

/// `qt(p, df)` for `0 < p < 1`, `0 < df < ∞`: invert the beta relation
/// — `I_x(df/2, 1/2) = 2·min(p, 1−p)` at `x = df/(df + t²)`, so
/// `t = ±√(df·(1−x)/x)` — through `beta_quantile` (spec §4.1).
/// `Err(NotBracketed)` outside the domain, `Err(NoConvergence)` if the
/// root solve fails. For `df > 4e5` the quantile solves
/// `tail_val(t) = 2·min(p, 1−p)` on `v = ln t` instead — the function
/// `pt` itself computes — because `x` is then too close to 1 for
/// `t = √(df(1−x)/x)` to keep f64 precision (at df = 1e300 the beta
/// form returned ~8e142 for a quantile R gives as 3.09).
pub fn t_quantile(p: f64, df: f64) -> Result<f64, RootError> {
    if !(p > 0.0 && p < 1.0) || !(df > 0.0 && df.is_finite()) {
        return Err(RootError::NotBracketed);
    }
    if p == 0.5 {
        return Ok(0.0);
    }
    if df > LARGE_DF {
        return t_quantile_tail_solve(p, df);
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

/// `tail_val(t) = q2` for `0 < q2 < 1` on `v = ln t`, the |t| > 0
/// side only. `tail_val` decreases in `t`; the initial estimate is
/// the normal quantile — exact as df → ∞ — so the doubling bracket
/// almost never expands. The absolute tolerance on `v` is a ~1e-14
/// relative tolerance on `t`.
fn t_quantile_tail_solve(p: f64, df: f64) -> Result<f64, RootError> {
    let q2 = 2.0 * p.min(1.0 - p);
    // Normal-quantile seed: |z| ≤ ~38.5 for every representable p;
    // z = 0 means p is within an ulp of 0.5, where qt is 0 anyway.
    let z = normal_quantile(q2 * 0.5)
        .map(libm::fabs)
        .ok_or(RootError::NotBracketed)?;
    if z == 0.0 {
        return Ok(0.0);
    }
    let v_est = libm::log(z.max(f64::MIN_POSITIVE));
    if !v_est.is_finite() {
        return Err(RootError::NotBracketed);
    }
    let f = |v: f64| tail_val(libm::exp(v), df) - q2;
    let mut lo = v_est - 4.0;
    while f(lo) <= 0.0 {
        let next = lo - (v_est - lo) * 2.0;
        if !next.is_finite() || libm::exp(next) == 0.0 {
            lo = if f(-745.0) <= 0.0 {
                return Err(RootError::NotBracketed);
            } else {
                -745.0
            };
            break;
        }
        lo = next;
    }
    let mut hi = v_est + 4.0;
    while f(hi) >= 0.0 {
        let next = hi + (hi - v_est) * 2.0;
        if !next.is_finite() || libm::exp(next).is_infinite() {
            hi = if f(709.0) >= 0.0 {
                return Err(RootError::NotBracketed);
            } else {
                709.0
            };
            break;
        }
        hi = next;
    }
    let v = brent_root(f, lo, hi)?;
    let t = libm::exp(v);
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
    if !u_est.is_finite() {
        return Err(RootError::NotBracketed);
    }
    let f = |v: f64| reg_inc_beta(libm::exp(u_est + v), a, b) - q;
    // Expansions double each step so a huge |u_est| (df → 0 gives
    // u_est ≈ −1e300) is still reached in ~1024 steps — linear steps
    // could never terminate (see gamma_quantile.rs).
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
        if u_est + next > 0.0 {
            // u = 0 gives I_1(a, b) = 1 > q — always the upper bracket.
            hi = -u_est;
            break;
        }
        hi = next;
    }
    brent_root(f, lo, hi).map(|v| libm::exp(u_est + v))
}
