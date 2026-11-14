//! The `else` region of R 4.6.2 `pgamma.c`'s `pgamma_raw` — x ≥ 1
//! "fairly near alph" — where R computes the lower tail by
//! `ppois_asymp(alph − 1, x, !lower_tail)` rather than by a
//! complement. The previous `1 − inc_gamma_upper` there formed a
//! double complement (`inc_gamma_upper` itself is `1 − lower_series`
//! for x < a + 1) and floored any P ≲ 2·DBL_EPSILON to multiples of
//! `DBL_EPSILON` — `pgamma(9254, 1e4)` was off by 1e-14 in relative
//! terms. `ppois_asymp` evaluates the small tail directly through
//! `pnorm`/`dnorm`, so the deep lower tail keeps full precision.
//!
//! Also ported: `log1pmx` and its `logcf` auxiliary (needed by
//! `ppois_asymp`), and the non-log `dnorm(x, 0, 1)` with R's split-x
//! tail form. R's `res < DBL_MIN/DBL_EPSILON` log-space redo is not
//! ported: every branch below already yields the denormal or zero R
//! would recover, so it can only change results inside the last
//! denormal ulps.
//!
//! Sources: `src/nmath/pgamma.c` (`pgamma_smallx`, `ppois_asymp`,
//! `logcf`, `log1pmx`, `lgamma1p`), `src/nmath/dnorm.c` (`dnorm4`,
//! non-log, σ = 1), `src/nmath/pnorm.c` via
//! `normal::{normal_cdf, normal_sf}`.

use super::gamma::{
    dpois_wrap, ln_gamma, lower_series, pd_lower_cf, pd_lower_series, pd_upper_series,
};
use super::normal::{normal_cdf, normal_sf};

/// `P(a, x)` for the §4.1 quantile solvers — `inc_gamma_lower` itself.
pub(crate) fn reg_inc_gamma_lower(a: f64, x: f64) -> f64 {
    inc_gamma_lower(a, x)
}

/// `pgamma_raw(x, a, lower_tail = TRUE, log_p = FALSE)` — the lower
/// tail as R's `pgamma` computes it (nmath `pgamma.c` branch for
/// branch). The "x large" region is a verbatim port: it returns
/// `1 − d·sum` directly, so `1 − chisq_cdf` reproduces `1 − pchisq`
/// even in the deep upper tail where it differs from `chisq_sf`. The
/// `x < 1` series is the same A&S 6.5.29 form R uses; the else region
/// is `ppois_asymp`, as in R — a `1 −` complement there floored
/// P ≲ 2·DBL_EPSILON.
pub(crate) fn inc_gamma_lower(a: f64, x: f64) -> f64 {
    if x < 1.0 {
        lower_series(a, x, ln_gamma(a))
    } else if x <= a - 1.0 && x < 0.8 * (a + 50.0) {
        pd_upper_series(x, a) * dpois_wrap(a, x)
    } else if a - 1.0 < x && a < 0.8 * (x + 50.0) {
        let d = dpois_wrap(a, x);
        let sum = if a < 1.0 {
            if x * f64::EPSILON > 1.0 - a {
                1.0
            } else {
                pd_lower_cf(a, x - (a - 1.0)) * x / a
            }
        } else {
            1.0 + pd_lower_series(x, a - 1.0)
        };
        1.0 - d * sum
    } else {
        ppois_asymp(a - 1.0, x, false)
    }
}

/// `scalefactor` of R's `pgamma.c` — `(2^32)^8 = 2^256`.
const SCALE_FACTOR: f64 = 1.157920892373162e77;
/// `M_1_SQRT_2PI` — `1/sqrt(2π)` as in R's `dnorm.c`.
const M_1_SQRT_2PI: f64 = 0.3989422804014327;
/// `M_LN2` — `log(2)`.
const M_LN2: f64 = std::f64::consts::LN_2;

/// `logcf(x, i, d, eps)` of R's `pgamma.c` — continued fraction for
/// `1/i + x/(i+d) + x^2/(i+2d) + …`, auxiliary in `log1pmx`.
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
fn log1pmx(x: f64) -> f64 {
    const MIN_LOG1_VALUE: f64 = -0.79149064;

    if x > 1.0 || x < MIN_LOG1_VALUE {
        libm::log1p(x) - x
    } else {
        // expand in y = [x/(2+x)]^2:
        // log(1+x) - x = x/(2+x) * (2*y*S(y) - x),
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

/// `pgamma_smallx(x, alph, lower_tail = FALSE, log_p = FALSE)` —
/// `−(f1m1 + f2m1 + f1m1·f2m1)`, the upper tail computed without
/// forming `1 − P` (R's `pgamma.c`). This is the only place
/// `inc_gamma_upper`'s `1 − lower_series` could floor: Q < ~2e-7 with
/// x < a+1 requires a ≲ 1e-5 — for a ≥ 1, x < a+1 keeps Q ≳ 0.4.
fn pgamma_smallx_upper(a: f64, x: f64) -> f64 {
    let f1m1 = smallx_series(a, x);
    let f2m1 = libm::expm1(a * libm::log(x) - lgamma1p(a));
    -(f1m1 + f2m1 + f1m1 * f2m1)
}

/// The `pgamma_raw` upper tail for `a < 1` and `x < a + 1` — where
/// `inc_gamma_upper`'s former `1 − lower_series` lost every digit
/// below Q ~ a (and could go negative at a ≲ 1e-13). For `x < 1` it
/// is `pgamma_smallx`'s `!lower_tail` form; for `x ≥ 1` (always the
/// "x large" branch when a < 1) it is `d·sum` without the `1 −`
/// complement.
pub(crate) fn inc_gamma_upper_lt1(a: f64, x: f64) -> f64 {
    if x < 1.0 {
        pgamma_smallx_upper(a, x)
    } else {
        let d = dpois_wrap(a, x);
        let sum = if x * f64::EPSILON > 1.0 - a {
            1.0
        } else {
            pd_lower_cf(a, x - (a - 1.0)) * x / a
        };
        sum * d
    }
}

/// `dnorm(x, 0, 1, give_log = FALSE)` — the standard-normal density as
/// R's `dnorm4` computes it, including the `x ≥ 5` split that keeps
/// `exp(-x²/2)` accurate to the underflow boundary (`dnorm.c`, the
/// non-`MATHLIB_FAST_dnorm` default build).
fn dnorm_std(x: f64) -> f64 {
    let x = x.abs();
    if !x.is_finite() || x >= 2.0 * libm::sqrt(f64::MAX) {
        return 0.0;
    }
    if x < 5.0 {
        return M_1_SQRT_2PI * libm::exp(-0.5 * x * x);
    }
    // -x^2/2 < log(2)·(.Machine$double.min.exp + .Machine$double.ulp.digits):
    // underflows to zero anyway (DBL_MIN_EXP = -1022, DBL_MANT_DIG = 53).
    if x > libm::sqrt(-2.0 * M_LN2 * (-1022.0 + 1.0 - 53.0)) {
        return 0.0;
    }
    // Split x = x1 + x2 with |x2| ≤ 2^-16 so x1² is exact.
    let x1 = libm::ldexp(libm::rint(libm::ldexp(x, 16)), -16);
    let x2 = x - x1;
    M_1_SQRT_2PI * (libm::exp(-0.5 * x1 * x1) * libm::exp((-0.5 * x2 - x1) * x2))
}

/// `ppois_asymp(x, lambda, lower_tail)` of R's `pgamma.c` — the
/// asymptotic expansion for `P(Poisson(lambda) ≤ x)` (or its
/// complement), non-log form only. `pgamma_raw` calls it with
/// `(alph − 1, x, !lower_tail)` in the region where x is fairly near
/// alph and neither `pd_*` series applies.
pub(crate) fn ppois_asymp(x: f64, lambda: f64, lower_tail: bool) -> f64 {
    const COEFS_A: [f64; 8] = [
        -1e99, // placeholder used for 1-indexing
        2.0 / 3.0,
        -4.0 / 135.0,
        8.0 / 2835.0,
        16.0 / 8505.0,
        -8992.0 / 12629925.0,
        -334144.0 / 492567075.0,
        698752.0 / 1477701225.0,
    ];
    const COEFS_B: [f64; 8] = [
        -1e99, // placeholder
        1.0 / 12.0,
        1.0 / 288.0,
        -139.0 / 51840.0,
        -571.0 / 2488320.0,
        163879.0 / 209018880.0,
        5246819.0 / 75246796800.0,
        -534703531.0 / 902961561600.0,
    ];

    let dfm = lambda - x;
    // If lambda is large, the distribution is highly concentrated
    // about lambda.  So representation error in x or lambda can lead
    // to arbitrarily large values of pt_ and hence divergence of the
    // coefficients of this approximation.  (R's comment.)
    let pt_ = -log1pmx(dfm / x);
    let mut s2pt = libm::sqrt(2.0 * x * pt_);
    if dfm < 0.0 {
        s2pt = -s2pt;
    }

    let mut res12 = 0.0;
    let mut res1_term = libm::sqrt(x);
    let mut res1_ig = res1_term;
    let mut res2_term = s2pt;
    let mut res2_ig = res2_term;
    for i in 1..8usize {
        let i = i as f64;
        res12 += res1_ig * COEFS_A[i as usize];
        res12 += res2_ig * COEFS_B[i as usize];
        res1_term *= pt_ / i;
        res2_term *= 2.0 * pt_ / (2.0 * i + 1.0);
        res1_ig = res1_ig / x + res1_term;
        res2_ig = res2_ig / x + res2_term;
    }

    let mut elfb = x;
    let mut elfb_term = 1.0;
    for &cb in &COEFS_B[1..] {
        elfb += elfb_term * cb;
        elfb_term /= x;
    }
    if !lower_tail {
        elfb = -elfb;
    }

    let f = res12 / elfb;
    // np = pnorm(s2pt, 0, 1, !lower_tail): the normal tail opposite
    // the Poisson one — for pgamma's lower tail this is Φ(s2pt).
    let np = if lower_tail {
        normal_sf(s2pt)
    } else {
        normal_cdf(s2pt)
    };
    let nd = dnorm_std(s2pt);
    np + f * nd
}
