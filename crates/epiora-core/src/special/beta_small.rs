//! Small-parameter incomplete beta — the `min(a, b) <= 1` block of
//! R's `bratio` (toms708.c) plus the `bup`/`bgrat`/`bpser` pieces the
//! `1 < min < 40` L140 path needs. These are what R uses where the
//! Numerical Recipes continued fraction loses digits with one small
//! and one huge parameter (e.g. `I_x(0.5, 2e5)` off by ~4e-6 in the t
//! tail at df = 4e5). Ported verbatim like the rest of the engine's
//! R-derived special functions.

use super::beta_grat::{digamma, gam1, gamln1, grat_accum, EPS};
use super::beta_math::{algdiv, beta_log_prefactor};
use super::gamma::ln_gamma;

/// `mu` scaling from `bup`: `min(int(exparg(1)), int(exparg(0)))` =
/// 708 on IEEE doubles.
const BUP_MU: i32 = 708;

/// `(w, w1)` = (`I_x(a,b)`, `1 − I_x(a,b)`) for `min(a,b) <= 1`,
/// `0 < x <= 0.5` — the first dispatch block of `bratio`. Whichever
/// side is not produced directly is set by the `0.5 − v + 0.5`
/// complement, exactly as R's `L_end_from_w`/`L_end_from_w1` do.
pub(crate) fn bratio_small(x0: f64, a0: f64, b0: f64) -> (f64, f64) {
    let y0 = 1.0 - x0;
    if b0 < EPS.min(EPS * a0) {
        let w = fpser(a0, b0, x0);
        return (w, 0.5 - w + 0.5);
    }
    if a0 < EPS.min(EPS * b0) && b0 * x0 <= 1.0 {
        let w1 = apser(a0, b0, x0);
        return (0.5 - w1 + 0.5, w1);
    }
    if a0.max(b0) > 1.0 {
        // L20: min(a,b) <= 1 < max(a,b)
        if b0 <= 1.0 {
            let w = bpser(a0, b0, x0);
            return (w, 0.5 - w + 0.5);
        }
        if x0 >= 0.29 {
            let w1 = bpser(b0, a0, y0);
            return (0.5 - w1 + 0.5, w1);
        }
        if x0 < 0.1 && libm::pow(x0 * b0, a0) <= 0.7 {
            let w = bpser(a0, b0, x0);
            return (w, 0.5 - w + 0.5);
        }
        if b0 <= 15.0 {
            let w1 = bup(b0, a0, y0, x0, 20) + grat_accum(b0 + 20.0, a0, y0, x0, 0.0);
            return (0.5 - w1 + 0.5, w1);
        }
        let w1 = grat_accum(b0, a0, y0, x0, 0.0);
        return (0.5 - w1 + 0.5, w1);
    }
    // both a, b <= 1
    if a0 >= 0.2f64.min(b0) || libm::pow(x0, a0) <= 0.9 {
        let w = bpser(a0, b0, x0);
        return (w, 0.5 - w + 0.5);
    }
    if x0 >= 0.3 {
        let w1 = bpser(b0, a0, y0);
        return (0.5 - w1 + 0.5, w1);
    }
    let w1 = bup(b0, a0, y0, x0, 20) + grat_accum(b0 + 20.0, a0, y0, x0, 0.0);
    (0.5 - w1 + 0.5, w1)
}

/// The `b0 < 40` branch of `bratio`'s L30 (both a, b > 1 after the
/// lambda swap): `bpser` when `b0·x0 <= 0.7`, else the L140
/// `bup` + `bpser`/`bgrat` combination. Returns `I_{x0}(a0,b0)`.
pub(crate) fn l30_small_b(mut a0: f64, mut b0: f64, x0: f64, y0: f64) -> f64 {
    if b0 * x0 <= 0.7 {
        return bpser(a0, b0, x0);
    }
    // L140: peel the integer part off b0, then bup + bpser or bgrat.
    let mut n = b0 as i32;
    b0 -= n as f64;
    if b0 == 0.0 {
        n -= 1;
        b0 = 1.0;
    }
    let mut w = bup(b0, a0, y0, x0, n);
    if x0 <= 0.7 {
        return w + bpser(a0, b0, x0);
    }
    if a0 <= 15.0 {
        w += bup(a0, b0, x0, y0, 20);
        a0 += 20.0;
    }
    w + grat_accum(a0, b0, x0, y0, 0.0)
}

/// `I_x(a, b)` for `b < min(eps, eps·a)`, `x <= 0.5` — `fpser`.
fn fpser(a: f64, b: f64, x: f64) -> f64 {
    let mut ans = if a > EPS * 0.001 {
        let t = a * libm::log(x);
        if t < -709.0 {
            return 0.0;
        }
        libm::exp(t)
    } else {
        1.0
    };
    // note that 1/B(a,b) = b here
    ans *= b / a;
    let tol = EPS / a;
    let mut an = a + 1.0;
    let mut t = x;
    let mut s = t / an;
    loop {
        an += 1.0;
        t *= x;
        let c = t / an;
        s += c;
        if c.abs() <= tol {
            break;
        }
    }
    ans * (a * s + 1.0)
}

/// `I_{1−x}(b, a)` for `a <= min(eps, eps·b)`, `b·x <= 1`, `x <= 0.5`
/// — `apser`.
fn apser(a: f64, b: f64, x: f64) -> f64 {
    const G: f64 = std::f64::consts::EULER_GAMMA; // Euler's constant
    let bx = b * x;
    let t = x - bx;
    let c = if b * EPS <= 0.02 {
        libm::log(x) + digamma(b) + G + t
    } else {
        // b > 2e13: psi(b) ~ log(b)
        libm::log(bx) + G + t
    };
    let tol = EPS * 5.0 * c.abs();
    let (mut j, mut s, mut t) = (1.0f64, 0.0, t);
    loop {
        j += 1.0;
        t *= x - bx / j;
        let aj = t / j;
        s += aj;
        if aj.abs() <= tol || j > 1e7 {
            break;
        }
    }
    -a * (c + s)
}

/// Power series for `I_x(a,b)` — `bpser`, non-log (`log_p = FALSE`)
/// branches only.
fn bpser(a: f64, b: f64, x: f64) -> f64 {
    if x == 0.0 {
        return 0.0;
    }
    // ans := x^a / (a·B(a,b))
    let a0 = a.min(b);
    let mut ans;
    if a0 >= 1.0 {
        let b0 = a.max(b);
        // ln B(a,b): cancellation-safe via lnΓ(a0)+algdiv for b0 ≥ 8.
        let lnb = if b0 >= 8.0 {
            ln_gamma(a0) + algdiv(a0, b0)
        } else {
            ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b)
        };
        let z = a * libm::log(x) - lnb;
        ans = libm::exp(z) / a;
    } else {
        let mut b0 = a.max(b);
        if b0 < 8.0 {
            if b0 <= 1.0 {
                // a0 < 1 and a0 <= b0 <= 1
                ans = libm::pow(x, a);
                if ans == 0.0 {
                    return ans;
                }
                let apb = a + b;
                let z = if apb > 1.0 {
                    (gam1(apb - 1.0) + 1.0) / apb
                } else {
                    gam1(apb) + 1.0
                };
                let c = (gam1(a) + 1.0) * (gam1(b) + 1.0) / z;
                ans *= c * (b / apb);
            } else {
                // a0 < 1 < b0 < 8
                let mut u = gamln1(a0);
                let m = (b0 - 1.0) as i32;
                if m >= 1 {
                    let mut c = 1.0;
                    for _ in 1..=m {
                        b0 -= 1.0;
                        c *= b0 / (a0 + b0);
                    }
                    u += libm::log(c);
                }
                let z = a * libm::log(x) - u;
                b0 -= 1.0; // b0 now in (0, 7)
                let apb = a0 + b0;
                let t = if apb > 1.0 {
                    (gam1(apb - 1.0) + 1.0) / apb
                } else {
                    gam1(apb) + 1.0
                };
                ans = libm::exp(z) * (a0 / a) * (gam1(b0) + 1.0) / t;
            }
        } else {
            // a0 < 1 <= 8 <= b0
            let u = gamln1(a0) + algdiv(a0, b0);
            let z = a * libm::log(x) - u;
            ans = a0 / a * libm::exp(z);
        }
    }
    if ans == 0.0 || a <= EPS * 0.1 {
        return ans;
    }
    // the series: c *= (1 − b/n)·x; w = c/(a+n)
    let tol = EPS / a;
    let (mut n, mut sum, mut c) = (0.0f64, 0.0, 1.0);
    loop {
        n += 1.0;
        c *= (0.5 - b / n + 0.5) * x;
        let w = c / (a + n);
        sum += w;
        if !(n < 1e7 && w.abs() > tol) {
            break;
        }
    }
    if a * sum > -1.0 {
        ans * (a * sum + 1.0)
    } else {
        0.0
    }
}

/// `I_x(a,b) − I_x(a+n,b)` — `bup`, non-log form. The `exp(mu)`
/// scaling keeps `x^a·y^b/B` from underflowing when `a` is large;
/// `beta_log_prefactor` supplies the log-space prefactor `esum(mu,·)`
/// needs.
fn bup(a: f64, b: f64, x: f64, y: f64, n: i32) -> f64 {
    let apb = a + b;
    let ap1 = a + 1.0;
    let (mu, d) = if n > 1 && a >= 1.0 && apb >= ap1 * 1.1 {
        (BUP_MU, libm::exp(-(BUP_MU as f64)))
    } else {
        (0, 1.0)
    };
    // ret_val = exp(mu)·x^a y^b/(a·B(a,b))
    let mut ret_val = libm::exp(mu as f64 + beta_log_prefactor(a, b, x, y)) / a;
    if n == 1 || ret_val == 0.0 {
        return ret_val;
    }
    let nm1 = (n - 1) as usize;
    let mut w = d;
    // k = index of the maximum term
    let mut k = 0usize;
    if b > 1.0 {
        if y > 1e-4 {
            let r = (b - 1.0) * x / y - a;
            if r >= 1.0 {
                k = if r < nm1 as f64 { r as usize } else { nm1 };
            }
        } else {
            k = nm1;
        }
    }
    let mut dd = d;
    // increasing terms (up to k), then the rest with the eps break —
    // dd carries over between the two loops as in R.
    for i in 0..k {
        dd *= (apb + i as f64) / (ap1 + i as f64) * x;
        w += dd;
    }
    for i in k..nm1 {
        dd *= (apb + i as f64) / (ap1 + i as f64) * x;
        w += dd;
        if dd <= EPS * w {
            break;
        }
    }
    ret_val *= w;
    ret_val
}
