//! `ppois_asymp` — the Temme asymptotic expansion `pgamma_raw` uses in
//! the "x ≥ 1 fairly near alph" region — in both its non-log and
//! log-space forms, plus the two pieces its log form calls: `dpnorm`
//! (dnorm/pnorm in one pass) and the non-log `dnorm(x, 0, 1)` with R's
//! split-x tail. `pnorm`'s log form lives in `normal.rs` next to the
//! `pnorm_both` tables it shares.
//!
//! Sources: `src/nmath/pgamma.c` (`ppois_asymp`, `dpnorm`),
//! `src/nmath/dnorm.c` (`dnorm4`, non-log, σ = 1), `src/nmath/pnorm.c`
//! via `normal::{normal_cdf, normal_sf, pnorm_log}`.

use super::gamma_lower::log1pmx;
use super::normal::{normal_cdf, normal_sf, pnorm_log};

/// `M_1_SQRT_2PI` — `1/sqrt(2π)` as in R's `dnorm.c`.
const M_1_SQRT_2PI: f64 = 0.3989422804014327;
/// `M_LN2` — `log(2)`.
const M_LN2: f64 = std::f64::consts::LN_2;

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

/// `dpnorm(x, lower_tail, lp)` of `pgamma.c` —
/// `dnorm(x)/pnorm(x, lower_tail)` evaluated to higher accuracy than
/// the direct ratio (A&S 26.2.12). `lp` is `pnorm(x, lower_tail,
/// log_p = TRUE)`, used only when |x| ≤ 10 or the asked tail is near 1.
fn dpnorm(x: f64, lower_tail: bool, lp: f64) -> f64 {
    let (x, lower_tail) = if x < 0.0 {
        (-x, !lower_tail)
    } else {
        (x, lower_tail)
    };
    if x > 10.0 && !lower_tail {
        let mut term = 1.0 / x;
        let mut sum = term;
        let x2 = x * x;
        let mut i = 1.0;
        // R's do/while is uncapped; the asymptotic terms decay below
        // eps·sum quickly for x > 10 — the cap matches `pd_lower_cf`'s
        // max_it and is never reached.
        for _ in 0..200_000 {
            term *= -i / x2;
            sum += term;
            i += 2.0;
            if term.abs() <= f64::EPSILON * sum {
                break;
            }
        }
        1.0 / sum
    } else {
        dnorm_std(x) / libm::exp(lp)
    }
}

/// `ppois_asymp(x, lambda, lower_tail, log_p)` of R's `pgamma.c` — the
/// asymptotic expansion for `P(Poisson(lambda) ≤ x)` (or its
/// complement / log). `pgamma_raw` calls it with `(alph − 1, x,
/// !lower_tail)` in the region where x is fairly near alph and neither
/// `pd_*` series applies. The log form is what `pgamma_raw`'s denormal
/// recompute uses: `np` in log space plus `log1p(f·nd/np)` via
/// `dpnorm` rather than `np + f·nd`.
pub(crate) fn ppois_asymp(x: f64, lambda: f64, lower_tail: bool, log_p: bool) -> f64 {
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
    // np = pnorm(s2pt, 0, 1, !lower_tail, log_p): the normal tail
    // opposite the Poisson one — for pgamma's lower tail this is
    // Φ(s2pt) (or log Φ(s2pt)).
    if log_p {
        let np = pnorm_log(s2pt, !lower_tail);
        let n_d_over_p = dpnorm(s2pt, !lower_tail, np);
        np + libm::log1p(f * n_d_over_p)
    } else {
        let np = if lower_tail {
            normal_sf(s2pt)
        } else {
            normal_cdf(s2pt)
        };
        np + f * dnorm_std(s2pt)
    }
}
