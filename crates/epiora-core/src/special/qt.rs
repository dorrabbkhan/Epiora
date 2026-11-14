//! `qt(p, df)` — R 4.6.2 `src/nmath/qt.c` (Hill's 1970 Algo 396 with
//! the 1981 two-term Taylor refinement and the `P_ok` log-space
//! branch), ported for the `lower_tail = TRUE, log_p = FALSE` form
//! the engine exposes (spec §4.1). R's non-finite results map to the
//! typed error: `R_Q_P01_boundaries` gives −Inf/+Inf at `p = 0`/`1`
//! (qt.c L55), `p > 1 − DBL_EPSILON` gives +Inf for `df < 1` (L71),
//! the `P ≤ DBL_MIN` arms of the df ≈ 2 / df ≈ 1 branches give +Inf
//! (L125, L137), and the `df < 1` bisection can itself underflow to
//! −Inf (`qt(1e-300, 0.5)`).

use super::gamma::{bd0, stirlerr};
use super::normal::{normal_quantile, qnorm_log_lower};
use super::root::RootError;
use super::t_dist::{t_cdf, t_sf};

/// `1e-12` — qt.c L46 `eps`: the `|df − 2|` and `df ≈ 1` bands.
const EPS: f64 = 1e-12;
/// `M_PI_2`.
const M_PI_2: f64 = std::f64::consts::FRAC_PI_2;
/// `M_LN2`.
const M_LN2: f64 = std::f64::consts::LN_2;
/// `M_1_SQRT_2PI`.
const M_1_SQRT_2PI: f64 = 0.3989422804014327;
/// `DBL_MANT_DIG` = 53 (qt.c L174's `-M_LN2 * DBL_MANT_DIG`).
const DBL_MANT_DIG: f64 = 53.0;

/// `qt(p, ndf, lower_tail = TRUE, log_p = FALSE)` — qt.c L44-215.
/// `Err(NotBracketed)` where R returns NaN or a signed infinity
/// (there is no infinity channel in `Result<f64, RootError>`).
pub(crate) fn qt_lower(p: f64, ndf: f64) -> Result<f64, RootError> {
    // L50-55: NaN inputs and R_Q_P01_boundaries. p < 0 / p > 1 is the
    // warned-NaN case; p == 0 / p == 1 are the ±Inf boundaries.
    if p.is_nan() || ndf.is_nan() || !(0.0..=1.0).contains(&p) {
        return Err(RootError::NotBracketed);
    }
    if p == 0.0 || p == 1.0 {
        return Err(RootError::NotBracketed); // ML_NEGINF / ML_POSINF
    }
    if ndf <= 0.0 {
        return Err(RootError::NotBracketed); // L57: ndf <= 0 -> NaN
    }
    if ndf < 1.0 {
        return small_df(p, ndf);
    }
    if ndf > 1e20 {
        // L100: the normal limit — qnorm(p, 0, 1, TRUE, FALSE).
        return normal_quantile(p).ok_or(RootError::NotBracketed);
    }

    // L102-109: P = 2·min(p, 1−p); R_D_Cval is `0.5 − p + 0.5`.
    let neg = p < 0.5;
    let big_p = if neg { 2.0 * p } else { 2.0 * (0.5 - p + 0.5) };

    let q = if (ndf - 2.0).abs() < EPS {
        // L112-127: df ~= 2
        if big_p > f64::MIN_POSITIVE {
            if 3.0 * big_p < f64::EPSILON {
                1.0 / libm::sqrt(big_p)
            } else if big_p > 0.9 {
                (1.0 - big_p) * libm::sqrt(2.0 / (big_p * (2.0 - big_p)))
            } else {
                libm::sqrt(2.0 / (big_p * (2.0 - big_p)) - 2.0)
            }
        } else {
            return Err(RootError::NotBracketed); // L125: ML_POSINF
        }
    } else if ndf < 1.0 + EPS {
        // L128-139: df ~= 1 — the Cauchy quantile.
        if big_p == 1.0 {
            0.0
        } else if big_p > 0.0 {
            1.0 / tanpi(big_p * 0.5)
        } else {
            return Err(RootError::NotBracketed); // L137: ML_POSINF
        }
    } else {
        usual_case(p, ndf, big_p, neg)
    };
    if !q.is_finite() {
        // A non-finite estimate is R's ±Inf/NaN — the typed error.
        return Err(RootError::NotBracketed);
    }
    Ok(if neg { -q } else { q })
}

/// qt.c L59-88 — `ndf < 1`, "based on qnt": bound loops around `pt`
/// then a plain bisection (R's comment: regula falsi failed on
/// `qt(0.1, 0.1)`).
fn small_df(p: f64, ndf: f64) -> Result<f64, RootError> {
    const ACCU: f64 = 1e-13;
    const EPS_B: f64 = 1e-11; // must be > accu

    if p > 1.0 - f64::EPSILON {
        return Err(RootError::NotBracketed); // L71: ML_POSINF
    }
    // L72-75: bracket so pt(ux) >= pp_up and pt(lx) <= pp_lo.
    let pp = (1.0 - f64::EPSILON).min(p * (1.0 + EPS_B));
    let mut ux = 1.0;
    while ux < f64::MAX && t_cdf(ux, ndf) < pp {
        ux *= 2.0;
    }
    let pp = p * (1.0 - EPS_B);
    let mut lx = -1.0;
    while lx > -f64::MAX && t_cdf(lx, ndf) > pp {
        lx *= 2.0;
    }
    // L80-83: interval halving — a do-while, so one step always runs;
    // ++iter is only evaluated when the ratio check is true.
    let mut iter = 0u32;
    let mut nx;
    loop {
        nx = 0.5 * (lx + ux);
        if t_cdf(nx, ndf) > p {
            ux = nx;
        } else {
            lx = nx;
        }
        if (ux - lx) / libm::fabs(nx) <= ACCU {
            break;
        }
        iter += 1;
        if iter >= 1000 {
            break;
        }
    }
    let q = 0.5 * (lx + ux);
    if !q.is_finite() {
        // R returns the underflowed ±Inf midpoint (qt(1e-300, df<1)).
        return Err(RootError::NotBracketed);
    }
    Ok(q)
}

/// qt.c L140-213 — the usual case: Hill's AS 396 initial estimate
/// (the asymptotic inverse expansion about the normal, or the
/// "y"-form below it), then the Taylor two-term refinement that uses
/// `dt` and `pt`.
#[allow(clippy::too_many_lines)]
fn usual_case(p: f64, ndf: f64, big_p: f64, neg: bool) -> f64 {
    let mut x = 0.0;
    let mut log_p2 = 0.0;
    let a = 1.0 / (ndf - 0.5);
    let b = 48.0 / (a * a);
    let mut c = ((20700.0 * a / b - 98.0) * a - 16.0) * a + 96.36;
    let d = ((94.5 / (b + c) - 3.0) / b + 1.0) * libm::sqrt(a * M_PI_2) * ndf;
    // L146-157: P_ok1 — qt.c L147 `P > DBL_MIN || !log_p`, always
    // true in this log_p = FALSE port; P_ok — `(d·P)^(2/df)` did
    // not underflow.
    let p_ok1 = true;
    let mut p_ok = p_ok1;
    let mut y = 0.0;
    if p_ok1 {
        y = libm::pow(d * big_p, 2.0 / ndf);
        p_ok = y >= f64::EPSILON;
    }
    if !p_ok {
        // L153-157: log_P2 = log(P/2) — is_neg_lower == neg here;
        // R_D_log is log(p), R_D_LExp is log1p(-p).
        log_p2 = if neg { libm::log(p) } else { libm::log1p(-p) };
        x = (libm::log(d) + M_LN2 + log_p2) / ndf;
        y = libm::exp(2.0 * x);
    }
    let mut q = if (ndf < 2.1 && big_p > 0.5) || y > 0.05 + a {
        // L159-173: asymptotic inverse expansion about the normal.
        if p_ok {
            x = normal_quantile(0.5 * big_p).unwrap_or(0.0);
        } else {
            // L163-164: qnorm(log_P2, ·, log_p = TRUE). Unreachable
            // for non-log p (P > 0 ⇒ y >= eps); kept for parity.
            x = qnorm_log_lower(log_p2);
        }
        y = x * x;
        if ndf < 5.0 {
            c += 0.3 * (ndf - 4.5) * (x + 0.6);
        }
        c += (((0.05 * d * x - 5.0) * x - 7.0) * x - 2.0) * x + b;
        y = (((((0.4 * y + 6.3) * y + 36.0) * y + 94.5) / c - y - 3.0) / b + 1.0) * x;
        y = libm::expm1(a * y * y);
        libm::sqrt(ndf * y)
    } else if !p_ok && x < -M_LN2 * DBL_MANT_DIG {
        // L174-177: y underflowed — use the log-space estimate.
        libm::sqrt(ndf) * libm::exp(-x)
    } else {
        // L178-183: the "y"-form, reusing y from above.
        y = ((1.0 / (((ndf + 6.0) / (ndf * y) - 0.089 * d - 0.822) * (ndf + 2.0) * 3.0)
            + 0.5 / (ndf + 4.0))
            * y
            - 1.0)
            * (ndf + 1.0)
            / (ndf + 2.0)
            + 1.0 / y;
        libm::sqrt(ndf * y)
    };
    // L193-213: two-term Taylor refinement (one term = Newton);
    // conditions evaluate in order — dt is not called once it > 10.
    if p_ok1 {
        let big_m = libm::fabs(libm::sqrt(f64::MAX / 2.0) - ndf);
        let mut it = 0u32;
        loop {
            it += 1;
            if it > 10 {
                break;
            }
            y = t_density(q, ndf);
            if y <= 0.0 || y.is_nan() {
                break;
            }
            x = (t_sf(q, ndf) - big_p * 0.5) / y;
            if !x.is_finite() || libm::fabs(x) <= 1e-14 * libm::fabs(q) {
                break;
            }
            let bigf = if libm::fabs(q) < big_m {
                q * (ndf + 1.0) / (2.0 * (q * q + ndf))
            } else {
                (ndf + 1.0) / (2.0 * (q + ndf / q))
            };
            let del_q = x * (1.0 + x * bigf);
            if del_q.is_finite() && (q + del_q).is_finite() {
                q += del_q;
            } else if (q + x).is_finite() {
                q += x;
            } else {
                break;
            }
        }
    }
    q
}

/// `dt(x, df, give_log = FALSE)` — R 4.6.2 `src/nmath/dt.c` L38-82:
/// `exp(t − u)·M_1_SQRT_2PI·I_sqrt_` with the `bd0`/`stirlerr`
/// prefactor and the three `x²/n` ranges. Called by the refinement
/// loop only, so `n` is always positive and finite.
fn t_density(x: f64, n: f64) -> f64 {
    if x.is_nan() || n.is_nan() {
        return x + n;
    }
    if n <= 0.0 {
        return f64::NAN;
    }
    if !x.is_finite() {
        return 0.0;
    }
    if !n.is_finite() {
        // unreachable from t_quantile (df <= 1e20); dnorm(x, 0, 1).
        return M_1_SQRT_2PI * libm::exp(-0.5 * x * x);
    }
    let t = -bd0(n * 0.5, (n + 1.0) * 0.5) + stirlerr((n + 1.0) * 0.5) - stirlerr(n * 0.5);
    let x2n = x * x / n;
    let lrg_x2n = x2n > 1.0 / f64::EPSILON;
    let mut ax = 0.0;
    let (l_x2n, u);
    if lrg_x2n {
        ax = libm::fabs(x);
        l_x2n = libm::log(ax) - libm::log(n) * 0.5;
        u = n * l_x2n;
    } else if x2n > 0.2 {
        l_x2n = libm::log(1.0 + x2n) * 0.5;
        u = n * l_x2n;
    } else {
        l_x2n = libm::log1p(x2n) * 0.5;
        u = -bd0(n * 0.5, (n + x * x) * 0.5) + x * x * 0.5;
    }
    let i_sqrt = if lrg_x2n {
        libm::sqrt(n) / ax
    } else {
        libm::exp(-l_x2n)
    };
    libm::exp(t - u) * M_1_SQRT_2PI * i_sqrt
}

/// `tanpi(x)` — cospi.c L73-88 `Rtanpi`: `tan(πx)` with the exact
/// quarter values; called only for `P/2 ∈ (0, 0.5)` in the df ≈ 1 arm.
fn tanpi(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if !x.is_finite() {
        return f64::NAN;
    }
    let x = libm::fmod(x, 1.0);
    let x = if x <= -0.5 {
        x + 1.0
    } else if x > 0.5 {
        x - 1.0
    } else {
        x
    };
    if x == 0.0 {
        0.0
    } else if x == 0.5 {
        f64::NAN
    } else if x == 0.25 {
        1.0
    } else if x == -0.25 {
        -1.0
    } else {
        libm::tan(std::f64::consts::PI * x)
    }
}
