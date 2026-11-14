//! Student's t distribution (v1.0 spec §4.1): `pt`, `pt(lower.tail =
//! FALSE)` and `qt`. `pt` is R's `pt.c` since R 2.7: with
//! `nx = (df + t²)/df` evaluated as `1 + (t/df)·t` (so `t²` can exceed
//! `df` without overflow), `P(T > |t|) = ½·val` where `val` is
//! `pbeta(t²/(df+t²), ½, df/2, lower.tail = FALSE)` for `df > t²` and
//! `pbeta(1/nx, df/2, ½)` for `df ≤ t²` — each direction chosen so the
//! beta argument stays below ½ and never rounds to 1. When `nx >
//! 1e100` the argument would underflow, so `val` is the A&S 26.5.4
//! log-space form instead. `qt` is R 4.6.2's `qt.c` verbatim
//! (owner decision 2026-11-14, option 1): Hill's AS 396 initial
//! estimate plus the two-term Taylor refinement — see `qt.rs`.

use super::beta::{reg_inc_beta, reg_inc_beta_upper};
use super::gamma::{ln_gamma, stirlerr};
use super::normal::{normal_cdf, normal_sf};
use super::qt;
use super::root::RootError;

/// ln √(2π) — used by `lbeta`'s both-large branch.
const M_LN_SQRT_2PI: f64 = 0.91893853320467274178032973640561764;

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
    if t.is_nan() || df.is_nan() {
        return f64::NAN;
    }
    if df <= 0.0 {
        return f64::NAN; // pt.c L37: n <= 0 (covers -Inf)
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
    // pt.c L40: !R_FINITE(n) — +Inf only (-Inf is n <= 0 above) —
    // is the normal limit pnorm(x, 0, 1, lower_tail).
    if !df.is_finite() {
        return if lower_tail {
            normal_cdf(t)
        } else {
            normal_sf(t)
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

/// `pt(t, df)`. `df` must be positive; `+Inf` is the normal limit
/// (pt.c L40), NaN or a non-positive/`−Inf` `df` gives NaN.
pub fn t_cdf(t: f64, df: f64) -> f64 {
    pt_body(t, df, true)
}

/// `pt(t, df, lower.tail = FALSE)`.
pub fn t_sf(t: f64, df: f64) -> f64 {
    pt_body(t, df, false)
}

/// `qt(p, df)` — R 4.6.2's `qt.c` ported for
/// `lower.tail = TRUE, log.p = FALSE` (qt.rs): the `df < 1`
/// bisection, the df ≈ 2 / df ≈ 1 special forms, Hill's AS 396
/// initial estimate, and the two-term Taylor refinement with its
/// `P_ok`/`P_ok1` small-probability handling. `Err(NotBracketed)`
/// outside the domain and wherever R's result is NaN or a signed
/// infinity (p = 0/1 boundaries, `p > 1 − DBL_EPSILON` for df < 1,
/// the `P ≤ DBL_MIN` arms, and the underflowed bisection).
pub fn t_quantile(p: f64, df: f64) -> Result<f64, RootError> {
    qt::qt_lower(p, df)
}
