//! Shared numerical helpers for the TOMS 708 (`toms708.c`)
//! incomplete-beta pieces: `brcmp1`'s log-space prefactor, `algdiv`
//! (log-gamma ratio), `rlog1` (`x − ln(1+x)` without cancellation),
//! `bcorr` (Stirling corrections via `stirlerr`) and `erfc1`'s scaled
//! erfc. Split out of `beta_large` so each beta file stays
//! single-purpose; ported verbatim like the rest of the engine's
//! R-derived special functions.

use super::beta_grat::{gam1, gamln1};
use super::gamma::{ln_gamma, stirlerr};

const LN_1_SQRT_2PI: f64 = -0.918938533204673; // log(1/sqrt(2*pi))

/// `x^a · y^b / B(a, b)` — `brcmp1`'s non-log form.
pub(crate) fn beta_prefactor(a: f64, b: f64, x: f64, y: f64) -> f64 {
    libm::exp(beta_log_prefactor(a, b, x, y))
}

/// `ln(x^a · y^b / B(a, b))` — `brcmp1` in log space, restricted to
/// the branches reachable in this engine (b0 ≥ 8 uses `algdiv`; the
/// a0 < 1 product/`gam1` forms are ported for `bup`). Never forms
/// `lgamma(a+b) − lgamma(b)` directly, so it stays accurate to
/// a, b ~ 1e300.
pub(crate) fn beta_log_prefactor(a: f64, b: f64, x: f64, y: f64) -> f64 {
    if x == 0.0 || y == 0.0 {
        return f64::NEG_INFINITY;
    }
    let a0 = a.min(b);
    if a0 < 8.0 {
        // lnx/lny picked as in brcmp1: log1p keeps ln(1−x) exact when
        // x is small instead of subtracting ~1 from 1.
        let (lnx, lny) = if x <= 0.375 {
            (libm::log(x), libm::log1p(-x))
        } else if y > 0.375 {
            (libm::log(x), libm::log(y))
        } else {
            (libm::log1p(-y), libm::log(y))
        };
        let z = a * lnx + b * lny;
        let mut b0 = a.max(b);
        if a0 >= 1.0 {
            // ln B = lnΓ(a0) + lnΓ(b0) − lnΓ(a0+b0): ln_gamma(a0) +
            // algdiv(a0, b0) for b0 ≥ 8 (betaln's own form), the
            // lgamma combination below that.
            let lnb = if b0 >= 8.0 {
                ln_gamma(a0) + algdiv(a0, b0)
            } else {
                ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
            };
            return z - lnb;
        }
        if b0 >= 8.0 {
            // L80: a0 < 1, b0 ≥ 8
            let u = gamln1(a0) + algdiv(a0, b0);
            return libm::log(a0) + z - u;
        }
        if b0 <= 1.0 {
            // L40/L50: a0 < 1, b0 ≤ 1
            let apb = a + b;
            let z2 = if apb > 1.0 {
                (gam1(apb - 1.0) + 1.0) / apb
            } else {
                gam1(apb) + 1.0
            };
            let c = (gam1(a) + 1.0) * (gam1(b) + 1.0) / z2;
            return z + libm::log(a0 * c) - libm::log1p(a0 / b0);
        }
        // L60/L70: a0 < 1 < b0 < 8 — peel b0's integer part.
        let mut u = gamln1(a0);
        let n = (b0 - 1.0) as i32;
        if n >= 1 {
            let mut c = 1.0;
            for _ in 1..=n {
                b0 -= 1.0;
                c *= b0 / (a0 + b0);
            }
            u += libm::log(c);
        }
        b0 -= 1.0;
        let apb = a0 + b0;
        let t = if apb > 1.0 {
            (gam1(apb - 1.0) + 1.0) / apb
        } else {
            gam1(apb) + 1.0
        };
        return libm::log(a0) + (z - u) + libm::log1p(gam1(b0)) - libm::log(t);
    }
    // a ≥ 8 and b ≥ 8: centered u/v form so nothing ~a+b cancels.
    let apb = a + b;
    let lambda = if apb.is_finite() {
        if a <= b {
            a - apb * x
        } else {
            apb * y - b
        }
    } else {
        a * y - b * x
    };
    let (x0, y0) = if a <= b {
        let h = a / b;
        (h / (h + 1.0), 1.0 / (h + 1.0))
    } else {
        let h = b / a;
        (1.0 / (h + 1.0), h / (h + 1.0))
    };
    let e = -lambda / a;
    let u = if e.abs() > 0.6 {
        e - libm::log(x / x0)
    } else {
        rlog1(e)
    };
    let e = lambda / b;
    let v = if e.abs() <= 0.6 {
        rlog1(e)
    } else {
        e - libm::log(y / y0)
    };
    LN_1_SQRT_2PI + 0.5 * libm::log(b * x0) - (a * u + b * v) - bcorr(a, b)
}

/// `ln Γ(b)/Γ(a+b)` for b ≥ 8 — `algdiv` (Stirling form; `del(x)` is
/// the same `stirlerr` correction the c-series encodes).
pub(crate) fn algdiv(a: f64, b: f64) -> f64 {
    const C: [f64; 6] = [
        0.0833333333333333,
        -0.00277777777760991,
        7.9365066682539e-4,
        -5.9520293135187e-4,
        8.37308034031215e-4,
        -0.00165322962780713,
    ];
    let (c, x, d) = if a > b {
        let h = b / a;
        (1.0 / (h + 1.0), h / (h + 1.0), a + (b - 0.5))
    } else {
        let h = a / b;
        (h / (h + 1.0), 1.0 / (h + 1.0), b + (a - 0.5))
    };
    let x2 = x * x;
    // s<n> = (1 − x^n)/(1 − x)
    let s3 = x + x2 + 1.0;
    let s5 = x + x2 * s3 + 1.0;
    let s7 = x + x2 * s5 + 1.0;
    let s9 = x + x2 * s7 + 1.0;
    let s11 = x + x2 * s9 + 1.0;
    // w = del(b) − del(a+b)
    let t = 1.0 / (b * b);
    let mut w = ((((C[5] * s11 * t + C[4] * s9) * t + C[3] * s7) * t + C[2] * s5) * t + C[1] * s3)
        * t
        + C[0];
    w *= c / b;
    let u = d * libm::log1p(a / b); // alnrel(a/b) = log(1 + a/b)
    let v = a * (libm::log(b) - 1.0);
    if u > v {
        w - v - u
    } else {
        w - u - v
    }
}

/// `x − ln(1 + x)` for |x| ≤ 0.6 without cancellation — `rlog1`
/// ported verbatim (rational Chebyshev + argument reduction).
pub(crate) fn rlog1(x: f64) -> f64 {
    const A: f64 = 0.0566749439387324;
    const B: f64 = 0.0456512608815524;
    const P: [f64; 3] = [0.333333333333333, -0.224696413112536, 0.00620886815375787];
    const Q: [f64; 2] = [-1.27408923933623, 0.354508718369557];
    if x < -0.39 || x > 0.57 {
        return x - libm::log(x + 0.5 + 0.5);
    }
    let (h, w1) = if x < -0.18 {
        ((x + 0.3) / 0.7, A - (x + 0.3) / 0.7 * 0.3)
    } else if x > 0.18 {
        (x * 0.75 - 0.25, B + (x * 0.75 - 0.25) / 3.0)
    } else {
        (x, 0.0)
    };
    let r = h / (h + 2.0);
    let t = r * r;
    let w = ((P[2] * t + P[1]) * t + P[0]) / ((Q[1] * t + Q[0]) * t + 1.0);
    t * 2.0 * (1.0 / (1.0 - r) - r * w) + w1
}

/// `del(a) + del(b) − del(a+b)` where `lnΓ(x) = (x−½)lnx − x +
/// ½ln2π + del(x)` — `bcorr`, computed through our verbatim
/// `stirlerr` port (identical del, more series terms).
pub(crate) fn bcorr(a: f64, b: f64) -> f64 {
    stirlerr(a) + stirlerr(b) - stirlerr(a + b)
}

/// Scaled complementary error function `exp(x²)·erfc(x)` —
/// `erfc1(1, x)`, ported verbatim.
pub(crate) fn erfcx(x: f64) -> f64 {
    const C: f64 = 0.564189583547756;
    const A: [f64; 5] = [
        7.7105849500132e-5,
        -0.00133733772997339,
        0.0323076579225834,
        0.0479137145607681,
        0.128379167095513,
    ];
    const B: [f64; 3] = [0.00301048631703895, 0.0538971687740286, 0.375795757275549];
    const P: [f64; 8] = [
        -1.36864857382717e-7,
        0.564195517478974,
        7.21175825088309,
        43.1622272220567,
        152.98928504694,
        339.320816734344,
        451.918953711873,
        300.459261020162,
    ];
    const Q: [f64; 8] = [
        1.0,
        12.7827273196294,
        77.0001529352295,
        277.585444743988,
        638.980264465631,
        931.35409485061,
        790.950925327898,
        300.459260956983,
    ];
    const R: [f64; 5] = [
        2.10144126479064,
        26.2370141675169,
        21.3688200555087,
        4.6580782871847,
        0.282094791773523,
    ];
    const S: [f64; 4] = [
        94.153775055546,
        187.11481179959,
        99.0191814623914,
        18.0124575948747,
    ];
    let ax = x.abs();
    if ax <= 0.5 {
        let t = x * x;
        let top = (((A[0] * t + A[1]) * t + A[2]) * t + A[3]) * t + A[4] + 1.0;
        let bot = ((B[0] * t + B[1]) * t + B[2]) * t + 1.0;
        return libm::exp(t) * (0.5 - x * (top / bot) + 0.5);
    }
    let mut ret_val = if ax <= 4.0 {
        let top = ((((((P[0] * ax + P[1]) * ax + P[2]) * ax + P[3]) * ax + P[4]) * ax + P[5]) * ax
            + P[6])
            * ax
            + P[7];
        let bot = ((((((Q[0] * ax + Q[1]) * ax + Q[2]) * ax + Q[3]) * ax + Q[4]) * ax + Q[5]) * ax
            + Q[6])
            * ax
            + Q[7];
        top / bot
    } else {
        if x <= -5.6 {
            return libm::exp(x * x) * 2.0;
        }
        let t = 1.0 / (x * x);
        let top = (((R[0] * t + R[1]) * t + R[2]) * t + R[3]) * t + R[4];
        let bot = (((S[0] * t + S[1]) * t + S[2]) * t + S[3]) * t + 1.0;
        (C - t * top / bot) / ax
    };
    if x < 0.0 {
        ret_val = libm::exp(x * x) * 2.0 - ret_val;
    }
    ret_val
}
