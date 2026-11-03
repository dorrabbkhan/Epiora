//! The regularised incomplete beta I_x(a, b) — Numerical Recipes'
//! continued-fraction form — and `qbeta` as Brent on it (spec §3.4).

use super::gamma::ln_gamma;
use super::root::brent_root;

const FPMIN: f64 = 1e-300;
const ITMAX: usize = 100_000;

/// Regularised incomplete beta I_x(a, b) = B_x(a,b)/B(a,b).
pub fn reg_inc_beta(x: f64, a: f64, b: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if !(x > 0.0 && x < 1.0) {
        return if x <= 0.0 { 0.0 } else { 1.0 };
    }
    let bt = libm::exp(
        ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * libm::log(x) + b * libm::log(1.0 - x),
    );
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(a, b, x) / a
    } else {
        1.0 - bt * betacf(b, a, 1.0 - x) / b
    }
}

/// Lentz continued fraction for the incomplete beta (NR `betacf`).
fn betacf(a: f64, b: f64, x: f64) -> f64 {
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < FPMIN {
        d = FPMIN;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=ITMAX {
        let m2 = 2 * m;
        let mut aa = m as f64 * (b - m as f64) * x / ((qam + m2 as f64) * (a + m2 as f64));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
        if c.abs() < FPMIN {
            c = FPMIN;
        }
        d = 1.0 / d;
        h *= d * c;
        aa = -(a + m as f64) * (qab + m as f64) * x / ((a + m2 as f64) * (qap + m2 as f64));
        d = 1.0 + aa * d;
        if d.abs() < FPMIN {
            d = FPMIN;
        }
        c = 1.0 + aa / c;
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
    h
}

/// `qbeta(p, a, b)` — Brent's method (abs tol 1e-14) on
/// `reg_inc_beta(x, a, b) − p` (spec §3.4). `None` unless
/// `0 < p < 1` and `a, b > 0` and finite.
pub fn beta_quantile(p: f64, a: f64, b: f64) -> Option<f64> {
    if !(0.0 < p && p < 1.0) || !(a > 0.0 && a.is_finite()) || !(b > 0.0 && b.is_finite()) {
        return None;
    }
    brent_root(|x| reg_inc_beta(x, a, b) - p, 0.0, 1.0).ok()
}
