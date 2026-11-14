//! `pgamma_raw(x, alph, lower_tail, log_p)` — the complete R 4.6.2
//! `pgamma.c` dispatch for the regularised incomplete gamma, both
//! tails and both output modes. `inc_gamma_lower`/`inc_gamma_upper`
//! are its two non-log entry points behind `chisq_cdf`/`chisq_sf`,
//! `poisson_tail` and the §4.1 quantile solvers.
//!
//! Branch for branch this is R's structure: `pgamma_smallx` for
//! `x < 1`, `pd_upper_series·dpois_wrap` for `x ≤ alph−1` near alph,
//! the `d·sum`/`1 − d·sum` branch for `alph−1 < x` near x, and
//! `ppois_asymp(alph − 1, x, !lower_tail)` otherwise — including the
//! `res < DBL_MIN/DBL_EPSILON` redo in log space, which is what makes
//! denormal outputs (P or Q ≲ 1e-292, down to ~1e-320) come out
//! bit-identical to R where the non-log path lost them. The lower tail
//! for `x < 1` uses `pgamma_smallx`'s `f1·f2` form, which R keeps in
//! [0, 1] where the convergent-series form could exceed 1 by ulps.
//!
//! Sources: `src/nmath/pgamma.c` (`pgamma_raw`, `pgamma_smallx`,
//! `logcf`, `log1pmx`, `lgamma1p`, `R_Log1_Exp` via `dpq.h`); the
//! `pd_*`/`dpois_wrap` helpers live in `gamma.rs`, `ppois_asymp` and
//! `dpnorm` in `gamma_asymp.rs`, `dpois_raw` log space in
//! `gamma_ebd0.rs`.

use super::gamma::{
    dpois_raw, dpois_wrap, ln_gamma, pd_lower_cf, pd_lower_series, pd_upper_series,
};
use super::gamma_asymp::ppois_asymp;
use super::gamma_ebd0::dpois_raw_log;

/// `P(a, x)` for the §4.1 quantile solvers — `inc_gamma_lower` itself.
pub(crate) fn reg_inc_gamma_lower(a: f64, x: f64) -> f64 {
    inc_gamma_lower(a, x)
}

/// `P(a, x)` = `pgamma(x, a, lower_tail = TRUE)` — R's lower tail.
pub(crate) fn inc_gamma_lower(a: f64, x: f64) -> f64 {
    pgamma_raw(x, a, true, false)
}

/// `Q(a, x)` = `pgamma(x, a, lower_tail = FALSE)` — R's upper tail.
pub(crate) fn inc_gamma_upper(a: f64, x: f64) -> f64 {
    pgamma_raw(x, a, false, false)
}

/// `DBL_MIN / DBL_EPSILON` — the `res <` bound of `pgamma_raw`'s
/// log-space redo (`1.002084e-292`).
const REDO_LIMIT: f64 = 1.0020841800044864e-292;

/// `R_Log1_Exp(x)` = `log(1 − e^x)` for `x < 0` — `dpq.h`'s stable
/// form.
fn r_log1_exp(x: f64) -> f64 {
    if x > -std::f64::consts::LN_2 {
        libm::log(-libm::expm1(x))
    } else {
        libm::log1p(-libm::exp(x))
    }
}

/// `pgamma_raw(x, alph, lower_tail, log_p)` — R's `pgamma.c` branch
/// for branch, including the underflow redo: a non-log `res` below
/// `DBL_MIN/DBL_EPSILON` is recomputed in log space and exponentiated,
/// recovering the denormal digits the direct path underflows away.
/// Assumes `(x, alph)` not-NaN and `alph > 0`, as R does.
fn pgamma_raw(x: f64, a: f64, lower_tail: bool, log_p: bool) -> f64 {
    // R_P_bounds_01(x, 0., ML_POSINF)
    if x <= 0.0 {
        // R_DT_0
        return if lower_tail {
            if log_p {
                f64::NEG_INFINITY
            } else {
                0.0
            }
        } else if log_p {
            0.0
        } else {
            1.0
        };
    }
    if x >= f64::INFINITY {
        // R_DT_1
        return if lower_tail {
            if log_p {
                0.0
            } else {
                1.0
            }
        } else if log_p {
            f64::NEG_INFINITY
        } else {
            0.0
        };
    }

    let res = if x < 1.0 {
        pgamma_smallx(x, a, lower_tail, log_p)
    } else if x <= a - 1.0 && x < 0.8 * (a + 50.0) {
        // incl. large alph compared to x
        let sum = pd_upper_series(x, a, log_p); // = x/alph + o(x/alph)
        let d = dpois_wrap(a, x, log_p);
        if lower_tail {
            if log_p {
                sum + d
            } else {
                sum * d
            }
        } else if log_p {
            r_log1_exp(d + sum)
        } else {
            1.0 - d * sum
        }
    } else if a - 1.0 < x && a < 0.8 * (x + 50.0) {
        // incl. large x compared to alph
        let d = dpois_wrap(a, x, log_p);
        let sum = if a < 1.0 {
            if x * f64::EPSILON > 1.0 - a {
                if log_p {
                    0.0
                } else {
                    1.0
                }
            } else {
                let f = pd_lower_cf(a, x - (a - 1.0)) * x / a;
                if log_p {
                    libm::log(f)
                } else {
                    f
                }
            }
        } else {
            // = (alph-1)/x + o((alph-1)/x)
            let s = pd_lower_series(x, a - 1.0);
            if log_p {
                libm::log1p(s)
            } else {
                1.0 + s
            }
        };
        if lower_tail {
            if log_p {
                r_log1_exp(d + sum)
            } else {
                1.0 - d * sum
            }
        } else if log_p {
            sum + d
        } else {
            sum * d
        }
    } else {
        // x >= 1 and x fairly near alph.
        ppois_asymp(a - 1.0, x, !lower_tail, log_p)
    };

    // We lose a fair amount of accuracy to underflow in the cases
    // where the final result is very close to DBL_MIN. In those
    // cases, simply redo via log space. (R's comment.)
    if !log_p && res < REDO_LIMIT {
        libm::exp(pgamma_raw(x, a, lower_tail, true))
    } else {
        res
    }
}

/// `scalefactor` of R's `pgamma.c` — `(2^32)^8 = 2^256`.
const SCALE_FACTOR: f64 = 1.157920892373162e77;

/// `logcf(x, i, d, eps)` of R's `pgamma.c` — continued fraction for
/// `1/i + x/(i+d) + x^2/(i+2d) + x^3/(i+3d) + …`, auxiliary in
/// `log1pmx`.
fn logcf(x: f64, i: f64, d: f64, eps: f64) -> f64 {
    let mut c1 = 2.0 * d;
    let mut c2 = i + d;
    let mut c4 = c2 + d;
    let mut a1 = c2;
    let mut b1 = i * (c2 - i * x);
    let mut b2 = d * d * x;
    let mut a2 = c4 * c2 - b2;

    b2 = c4 * b1 - i * b2;

    // R's `while` is uncapped; the loop is geometric (~1/9 ratio on
    // the |x| < 1e-2 / [−0.791, 1] inputs it sees) — the cap matches
    // `pd_lower_cf`'s max_it and is never reached.
    for _ in 0..200_000 {
        if (a2 * b1 - a1 * b2).abs() <= (eps * b1 * b2).abs() {
            break;
        }
        let c3 = c2 * c2 * x;
        c2 += d;
        c4 += d;
        a1 = c4 * a2 - c3 * a1;
        b1 = c4 * b2 - c3 * b1;

        let c3 = c1 * c1 * x;
        c1 += d;
        c4 += d;
        a2 = c4 * a1 - c3 * a2;
        b2 = c4 * b1 - c3 * b2;

        if b2.abs() > SCALE_FACTOR {
            a1 /= SCALE_FACTOR;
            b1 /= SCALE_FACTOR;
            a2 /= SCALE_FACTOR;
            b2 /= SCALE_FACTOR;
        } else if b2.abs() < 1.0 / SCALE_FACTOR {
            a1 *= SCALE_FACTOR;
            b1 *= SCALE_FACTOR;
            a2 *= SCALE_FACTOR;
            b2 *= SCALE_FACTOR;
        }
    }

    a2 / b2
}

/// `log1pmx(x)` = `log(1+x) − x`, accurate for small x — R's
/// `pgamma.c` (logcf expansion on `[−0.791, 1]`).
pub(crate) fn log1pmx(x: f64) -> f64 {
    const MIN_LOG1_VALUE: f64 = -0.79149064;

    if x > 1.0 || x < MIN_LOG1_VALUE {
        libm::log1p(x) - x
    } else {
        // expand in y = [x/(2+x)]^2:
        // log(1+x) - x =  x/(2+x) * (2*y*S(y) - x),
        // S(y) = sum_{k>=0} y^k/(2k+3)
        let r = x / (2.0 + x);
        let y = r * r;
        if x.abs() < 1e-2 {
            let two = 2.0;
            r * ((((two / 9.0 * y + two / 7.0) * y + two / 5.0) * y + two / 3.0) * y - x)
        } else {
            r * (2.0 * y * logcf(y, 3.0, 2.0, 1e-14) - x)
        }
    }
}

/// `lgamma1p(a)` = `log(Γ(a+1))`, accurate for small a — R's
/// `pgamma.c`: `loggamma(1+a) = −(log(1+a) − a) − γ·a + a²·lgam`,
/// `lgam` a logcf-accelerated ζ series (A&S 6.1.33).
fn lgamma1p(a: f64) -> f64 {
    if a.abs() >= 0.5 {
        return ln_gamma(a + 1.0);
    }
    const EULERS_CONST: f64 = std::f64::consts::EULER_GAMMA;
    /// coeffs[i] = (ζ(i+2)−1)/(i+2), i = 0..40 — R's table.
    const COEFFS: [f64; 40] = [
        0.3224670334241132,
        0.0673523010531981,
        0.020580808427784546,
        0.007385551028673986,
        0.0028905103307415234,
        0.001192753911703261,
        0.0005096695247430425,
        0.00022315475845357939,
        9.945751278180853e-05,
        4.492623673813314e-05,
        2.050721277567069e-05,
        9.439488275268397e-06,
        4.374866789907488e-06,
        2.039215753801366e-06,
        9.55141213040742e-07,
        4.492469198764566e-07,
        2.1207184805554665e-07,
        1.0043224823968099e-07,
        4.7698101693639804e-08,
        2.2711094608943164e-08,
        1.0838659214896955e-08,
        5.183475041970047e-09,
        2.4836745438024785e-09,
        1.1921401405860912e-09,
        5.731367241678862e-10,
        2.7595228851242334e-10,
        1.330476437424449e-10,
        6.4229645638381e-11,
        3.1044247747322276e-11,
        1.5021384080754142e-11,
        7.275974480239079e-12,
        3.527742476575915e-12,
        1.711991790559618e-12,
        8.315385841420285e-13,
        4.04220052528944e-13,
        1.9664756310966165e-13,
        9.573630387838556e-14,
        4.6640760264283744e-14,
        2.2737369600659724e-14,
        1.1091399470834522e-14,
    ];
    const C: f64 = 2.2737368458246524e-13; // ζ(42)−1

    let mut lgam = C * logcf(-a / 2.0, 42.0, 1.0, 1e-14);
    for &ck in COEFFS.iter().rev() {
        lgam = ck - a * lgam;
    }
    (a * lgam - EULERS_CONST) * a - log1pmx(a)
}

/// The `sum` of `pgamma_smallx` — the A&S 6.5.29 alternating series
/// with all terms premultiplied by `alph` (R's `pgamma.c`), returning
/// the remainder sum `f1 − 1`.
fn smallx_series(a: f64, x: f64) -> f64 {
    let mut sum = 0.0;
    let mut c = a;
    // Bounded like the other series: R's do/while terminates once the
    // term stops mattering; the cap is never reached.
    for n in 1..=100_000u32 {
        let n = n as f64;
        c *= -x / n;
        let term = c / (a + n);
        sum += term;
        if term.abs() <= f64::EPSILON * sum.abs() {
            break;
        }
    }
    sum
}

/// `pgamma_smallx(x, alph, lower_tail, log_p)` — all four output
/// modes (R's `pgamma.c`). The lower tail returns `f1·f2` —
/// `f2 = dpois_raw(alph, x)·e^x` for `alph > 1` — which R keeps
/// within [0, 1]; the upper tail is `−(f1m1 + f2m1 + f1m1·f2m1)`,
/// computed without forming `1 − P`.
fn pgamma_smallx(x: f64, a: f64, lower_tail: bool, log_p: bool) -> f64 {
    let sum = smallx_series(a, x);
    if lower_tail {
        let f1 = if log_p { libm::log1p(sum) } else { 1.0 + sum };
        let f2 = if a > 1.0 {
            let f2 = if log_p {
                dpois_raw_log(a, x)
            } else {
                dpois_raw(a, x)
            };
            if log_p {
                f2 + x
            } else {
                f2 * libm::exp(x)
            }
        } else if log_p {
            a * libm::log(x) - lgamma1p(a)
        } else {
            libm::pow(x, a) / libm::exp(lgamma1p(a))
        };
        if log_p {
            f1 + f2
        } else {
            f1 * f2
        }
    } else {
        let lf2 = a * libm::log(x) - lgamma1p(a);
        if log_p {
            r_log1_exp(libm::log1p(sum) + lf2)
        } else {
            let f1m1 = sum;
            let f2m1 = libm::expm1(lf2);
            -(f1m1 + f2m1 + f1m1 * f2m1)
        }
    }
}
