//! `ln_gamma`, the regularised incomplete gamma used by `chisq_sf`,
//! and the two building blocks R uses for its discrete densities:
//! `stirlerr` (Stirling error of `log n!`) and `bd0` (the binomial
//! deviance), ported from R's `src/nmath/stirlerr.c` and `bd0.c` —
//! `dhyper`'s accuracy at `m = n = 2e6` depends on them (spec §3.4,
//! fixture `hypergeom_pmf`).

const FPMIN: f64 = 1e-300;
const ITMAX: usize = 100_000;

/// `lgamma(x)` for x > 0 (spec §3.4); the sign Γ(x) takes is dropped.
pub fn ln_gamma(x: f64) -> f64 {
    libm::lgamma_r(x).0
}

/// Regularised upper incomplete gamma Q(a, x) = Γ(a, x)/Γ(a): series
/// for x < a + 1 (as P, then 1 − P), Lentz continued fraction
/// otherwise (spec §3.4). For `a < 1` the `1 − P` form loses Q ≲ a to
/// the complement; R's own upper-tail branches for that region live
/// in `gamma_lower::inc_gamma_upper_lt1`.
pub(crate) fn inc_gamma_upper(a: f64, x: f64) -> f64 {
    let gln = ln_gamma(a);
    if x < a + 1.0 {
        if a < 1.0 {
            super::gamma_lower::inc_gamma_upper_lt1(a, x)
        } else {
            1.0 - lower_series(a, x, gln)
        }
    } else {
        upper_cf(a, x, gln)
    }
}

/// P(a, x) by its convergent series.
pub(crate) fn lower_series(a: f64, x: f64, gln: f64) -> f64 {
    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut del = sum;
    for _ in 0..ITMAX {
        ap += 1.0;
        del *= x / ap;
        sum += del;
        if del.abs() <= sum.abs() * f64::EPSILON {
            break;
        }
    }
    sum * libm::exp(-x + a * libm::log(x) - gln)
}

/// Q(a, x) by Lentz's continued fraction.
fn upper_cf(a: f64, x: f64, gln: f64) -> f64 {
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / FPMIN;
    let mut d = 1.0 / b;
    let mut h = d;
    for i in 1..=ITMAX {
        let an = -(i as f64) * (i as f64 - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = b + an / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() <= f64::EPSILON {
            break;
        }
    }
    libm::exp(-x + a * libm::log(x) - gln) * h
}

const M_LN_SQRT_2PI: f64 = -0.9189385332046728; // -log(sqrt(2*pi))
pub(crate) const M_LN_2PI: f64 = 1.8378770664093456; // log(2*pi)
const M_2PI: f64 = std::f64::consts::TAU; // 2*pi
/// `M_cutoff` of R's `pgamma.c` — `ln(2)·DBL_MAX_EXP/DBL_EPSILON`.
const M_CUTOFF: f64 = 3.196577180618028e18;
/// `scalefactor` of R's `pgamma.c` — `(2^32)^8 = 2^256`.
const SCALE_FACTOR: f64 = 1.157920892373162e77;

/// `dpois_raw(x, lambda, give_log = FALSE)` — Poisson probability
/// `lambda^x·e^-lambda / x!` for fractional `x` (R `dpois.c`). R ≥ 4.5
/// splits the deviance via `ebd0` (double-double arithmetic); the plain
/// `bd0 + stirlerr` form agrees to ~1e-13 relative in the worst case —
/// inside the `closed_form` class.
fn dpois_raw(x: f64, lambda: f64) -> f64 {
    if lambda == 0.0 {
        return if x == 0.0 { 1.0 } else { 0.0 };
    }
    if !lambda.is_finite() || x < 0.0 {
        return 0.0;
    }
    if x <= lambda * f64::MIN_POSITIVE {
        return libm::exp(-lambda);
    }
    if lambda < x * f64::MIN_POSITIVE {
        if !x.is_finite() {
            return 0.0;
        }
        return libm::exp(-lambda + x * libm::log(lambda) - ln_gamma(x + 1.0));
    }
    libm::exp(-bd0(x, lambda) - stirlerr(x)) / libm::sqrt(M_2PI * x)
}

/// `dpois_wrap(x_plus_1, lambda)` = `dpois(x_plus_1 − 1, lambda)`, the
/// form `pgamma` needs for fractional `x` (R `pgamma.c`, non-log only).
pub(crate) fn dpois_wrap(x_plus_1: f64, lambda: f64) -> f64 {
    if !lambda.is_finite() {
        return 0.0;
    }
    if x_plus_1 > 1.0 {
        return dpois_raw(x_plus_1 - 1.0, lambda);
    }
    if lambda > (x_plus_1 - 1.0).abs() * M_CUTOFF {
        libm::exp(-lambda - ln_gamma(x_plus_1))
    } else {
        dpois_raw(x_plus_1, lambda) * (x_plus_1 / lambda)
    }
}

/// `pd_upper_series(x, y)` of R's `pgamma.c`:
/// `sum_{n>=0} x^(n+1) / (y·(y+1)·…·(y+n))`, non-log only.
pub(crate) fn pd_upper_series(x: f64, mut y: f64) -> f64 {
    let mut term = x / y;
    let mut sum = term;
    // R's do/while: the first check `x/y > (x/y)·eps` always passes
    // for positive term, so the while form is identical.
    while term > sum * f64::EPSILON {
        y += 1.0;
        term *= x / y;
        sum += term;
    }
    sum
}

/// `pd_lower_cf(y, d)` of R's `pgamma.c` — the continued fraction for
/// the scaled upper tail.
pub(crate) fn pd_lower_cf(y: f64, d: f64) -> f64 {
    if y == 0.0 {
        return 0.0;
    }
    let mut f0 = y / d;
    if (y - 1.0).abs() < d.abs() * f64::EPSILON {
        return f0;
    }
    if f0 > 1.0 {
        f0 = 1.0;
    }
    let mut c2 = y;
    let mut c4 = d;
    let (mut a1, mut b1, mut a2, mut b2) = (0.0, 1.0, y, d);
    while b2 > SCALE_FACTOR {
        a1 /= SCALE_FACTOR;
        b1 /= SCALE_FACTOR;
        a2 /= SCALE_FACTOR;
        b2 /= SCALE_FACTOR;
    }
    let mut f = 0.0;
    let mut of = -1.0;
    let mut i = 0.0;
    while i < 200_000.0 {
        i += 1.0;
        c2 -= 1.0;
        let c3 = i * c2;
        c4 += 2.0;
        a1 = c4 * a2 + c3 * a1;
        b1 = c4 * b2 + c3 * b1;
        i += 1.0;
        c2 -= 1.0;
        let c3 = i * c2;
        c4 += 2.0;
        a2 = c4 * a1 + c3 * a2;
        b2 = c4 * b1 + c3 * b2;
        if b2 > SCALE_FACTOR {
            a1 /= SCALE_FACTOR;
            b1 /= SCALE_FACTOR;
            a2 /= SCALE_FACTOR;
            b2 /= SCALE_FACTOR;
        }
        if b2 != 0.0 {
            f = a2 / b2;
            if (f - of).abs() <= f64::EPSILON * f0.max(f.abs()) {
                return f;
            }
            of = f;
        }
    }
    f
}

/// `pd_lower_series(lambda, y)` of R's `pgamma.c`.
pub(crate) fn pd_lower_series(lambda: f64, mut y: f64) -> f64 {
    let mut term = 1.0;
    let mut sum = 0.0;
    while y >= 1.0 && term > sum * f64::EPSILON {
        term *= y / lambda;
        sum += term;
        y -= 1.0;
    }
    if y != libm::floor(y) {
        let f = pd_lower_cf(y, lambda + 1.0 - y);
        sum += term * f;
    }
    sum
}

/// `(S0 − (S1 − (S2 − … − Sk/nn)/nn …)/nn)/n` — the nested form every
/// non-tabulated branch of R's `stirlerr` uses (`c` is `S0..=Sk`).
fn s_series(c: &[f64], nn: f64, n: f64) -> f64 {
    let mut acc = c[c.len() - 1] / nn;
    for &ck in c[1..c.len() - 1].iter().rev() {
        acc = (ck - acc) / nn;
    }
    (c[0] - acc) / n
}

/// `stirlerr(n)` = `log(n!) − log(sqrt(2πn)·(n/e)^n)` — R 4.6.x
/// `src/nmath/stirlerr.c`, ported verbatim (half-integer table,
/// rational branches at the same cutoffs).
pub(crate) fn stirlerr(n: f64) -> f64 {
    const S: [f64; 17] = [
        0.08333333333333333,   // 1/12
        0.002777777777777778,  // 1/360
        0.0007936507936507937, // 1/1260
        0.0005952380952380953, // 1/1680
        0.0008417508417508417, // 1/1188
        0.0019175269175269176, // 691/360360
        0.00641025641025641,   // 1/156
        0.029550653594771242,  // 3617/122400
        0.17964437236883057,   // 43867/244188
        1.3924322169059011,    // 174611/125400
        13.402864044168393,    // 77683/5796
        156.84828462600203,    // 236364091/1506960
        2193.1033333333335,    // 657931/300
        36108.77125372499,     // 3392780147/93960
        691472.268851313,      // 1723168255201/2492028
        15238221.539407415,    // 7709321041217/505920
        382900751.39141417,    // 151628697551/396
    ];

    // exact values for 0, 0.5, 1.0, ..., 14.5, 15.0
    const SFERR_HALVES: [f64; 31] = [
        0.0,                   // n = 0 (place holder)
        0.15342640972002736,   // 0.5
        0.08106146679532726,   // 1.0
        0.05481412105191765,   // 1.5
        0.0413406959554093,    // 2.0
        0.03316287351993629,   // 2.5
        0.02767792568499834,   // 3.0
        0.023746163656297496,  // 3.5
        0.020790672103765093,  // 4.0
        0.018488450532673187,  // 4.5
        0.016644691189821193,  // 5.0
        0.015134973221917378,  // 5.5
        0.013876128823070748,  // 6.0
        0.012810465242920227,  // 6.5
        0.01189670994589177,   // 7.0
        0.011104559758206917,  // 7.5
        0.010411265261972096,  // 8.0
        0.009799416126158804,  // 8.5
        0.009255462182712733,  // 9.0
        0.008768700134139386,  // 9.5
        0.00833056343336287,   // 10.0
        0.00793411456431402,   // 10.5
        0.007573675487951841,  // 11.0
        0.007244554301320383,  // 11.5
        0.00694284010720953,   // 12.0
        0.006665247032707682,  // 12.5
        0.006408994188004207,  // 13.0
        0.006171712263039458,  // 13.5
        0.0059513701127588475, // 14.0
        0.0057462165130101155, // 14.5
        0.005554733551962801,  // 15.0
    ];

    if n <= 23.5 {
        let nn = n + n;
        if n <= 15.0 && nn == libm::trunc(nn) {
            return SFERR_HALVES[nn as usize];
        }
        if n <= 5.25 {
            if n >= 1.0 {
                // slightly more accurate than the direct form (R "MM2")
                let l_n = libm::log(n);
                return ln_gamma(n) + n * (1.0 - l_n) + (l_n - M_LN_2PI) * 0.5;
            }
            return ln_gamma(n + 1.0) - (n + 0.5) * libm::log(n) + n + M_LN_SQRT_2PI;
        }
        let nn = n * n;
        // R's branch table: the higher the term count, the lower n
        if n > 12.8 {
            s_series(&S[..7], nn, n) // k = 7
        } else if n > 12.3 {
            s_series(&S[..8], nn, n) // k = 8
        } else if n > 8.9 {
            s_series(&S[..9], nn, n) // k = 9
        } else if n > 7.3 {
            s_series(&S[..11], nn, n) // k = 11
        } else if n > 6.6 {
            s_series(&S[..13], nn, n) // k = 13
        } else if n > 6.1 {
            s_series(&S[..15], nn, n) // k = 15
        } else {
            s_series(&S[..17], nn, n) // k = 17 (6.1 >= n > 5.25)
        }
    } else {
        // n > 23.5
        let nn = n * n;
        if n > 15700000.0 {
            S[0] / n
        } else if n > 6180.0 {
            s_series(&S[..2], nn, n)
        } else if n > 205.0 {
            s_series(&S[..3], nn, n)
        } else if n > 86.0 {
            s_series(&S[..4], nn, n)
        } else if n > 27.0 {
            s_series(&S[..5], nn, n)
        } else {
            s_series(&S[..6], nn, n)
        }
    }
}

/// `bd0(x, np)` = `x·log(x/np) + np − x` — the binomial deviance,
/// R 4.6.x `src/nmath/bd0.c`, ported verbatim (Taylor series for
/// x ≈ np, direct form otherwise).
pub(crate) fn bd0(x: f64, np: f64) -> f64 {
    if !x.is_finite() || !np.is_finite() || np == 0.0 {
        return f64::NAN;
    }
    if (x - np).abs() < 0.1 * (x + np) {
        let d = x - np;
        let mut v = d / (x + np);
        if d != 0.0 && v == 0.0 {
            // v underflowed to 0 because x + np overflowed
            let (x_, n_) = (x * 0.25, np * 0.25);
            v = (x_ - n_) / (x_ + n_);
        }
        let mut s = (d * 0.5) * v;
        if (s * 2.0).abs() < f64::MIN_POSITIVE {
            return s * 2.0;
        }
        let mut ej = x * v;
        v *= v;
        for j in 1..1000i64 {
            ej *= v;
            let s_ = s;
            s += ej / ((2 * j + 1) as f64);
            if s == s_ {
                return s * 2.0;
            }
        }
        // Taylor series did not converge: R warns and falls through to
        // the direct evaluation below.
    }
    let lg_x_n = if (x / np).is_finite() {
        libm::log(x / np)
    } else {
        libm::log(x) - libm::log(np)
    };
    if x > np {
        x * (lg_x_n - 1.0) + np
    } else {
        x * lg_x_n + np - x
    }
}
