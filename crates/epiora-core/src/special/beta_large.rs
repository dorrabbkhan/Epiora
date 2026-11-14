//! Large-parameter regularised incomplete beta — the branches of R's
//! `pbeta` (TOMS 708 `toms708.c`) that stay accurate when the plain
//! `exp(lgamma(a+b) − lgamma(a) − lgamma(b))` prefactor cancels: for
//! a, b ≳ 1e7 the difference `lgamma(a+b) − lgamma(b)` is smaller than
//! one ulp of either operand, so the old `bt` was pure noise (it
//! produced NaN, >1 and negative values at df ~ 1e15+). Ported
//! pieces: `brcmp1` (log-space prefactor), `algdiv` (log-gamma
//! ratio), `rlog1` (x − ln(1+x)), `erfc1` (scaled erfc), `bfrac`
//! (continued fraction for the tail) and `basym` (asymptotic
//! expansion near the mean). `bcorr` maps onto the already-ported
//! `stirlerr`; the `b0 < 40` and `min(a,b) <= 1` dispatch lives in
//! `beta_small`.

use super::beta_math::{bcorr, beta_prefactor, erfcx, rlog1};
use super::beta_small::l30_small_b;

const EPS: f64 = 1e-15; // R sets eps = max(DBL_EPSILON, 1e-15) in bratio

/// `I_x(a, b)` (upper = false: the lower tail; upper = true: `1 −
/// I_x`) for a, b > 1 — `bratio`'s L30 dispatch: swap by
/// `lambda = a·y − b·x` so the computed tail is the one at/below the
/// mean, then `bpser`/`bup`/`bgrat` for `b0 < 40`, else `bfrac` far
/// from the mean or `basym` near it.
pub(crate) fn inc_beta_large(x: f64, a: f64, b: f64, upper: bool) -> f64 {
    let y = 1.0 - x;
    // lambda := a·y − b·x, in the numerically best version (bratio).
    let lambda = if (a + b).is_finite() {
        if a > b {
            (a + b) * y - b
        } else {
            a - (a + b) * x
        }
    } else {
        a * y - b * x
    };
    let (x0, y0, a0, b0, lam) = if lambda < 0.0 {
        (y, x, b, a, -lambda)
    } else {
        (x, y, a, b, lambda)
    };
    // bratio's L30 branch: b0 < 40 goes to the small-b methods;
    // otherwise bfrac off the mean, basym near it.
    let w = if b0 < 40.0 {
        l30_small_b(a0, b0, x0, y0)
    } else if a0 > b0 {
        if b0 <= 100.0 || lam > b0 * 0.03 {
            bfrac(a0, b0, x0, y0, lam, EPS * 15.0)
        } else {
            basym(a0, b0, lam, EPS * 100.0)
        }
    } else if a0 <= 100.0 || lam > a0 * 0.03 {
        bfrac(a0, b0, x0, y0, lam, EPS * 15.0)
    } else {
        basym(a0, b0, lam, EPS * 100.0)
    };
    // w = I_{x0}(a0, b0) with x0 ≤ a0/(a0+b0), so w ≲ 0.5 and 1 − w is
    // safe. lambda < 0 means the swap put the computed tail on y.
    match (lambda < 0.0, upper) {
        (false, false) => w,
        (false, true) => 1.0 - w,
        (true, false) => 1.0 - w,
        (true, true) => w,
    }
}

/// `I_x(a, b)` by the continued fraction on the tail — `bfrac`, for
/// lambda = a·y − b·x ≥ 0 (x at/below the mean).
fn bfrac(a: f64, b: f64, x: f64, y: f64, lambda: f64, eps: f64) -> f64 {
    if !lambda.is_finite() {
        return f64::NAN;
    }
    let brc = beta_prefactor(a, b, x, y);
    if brc.is_nan() || brc == 0.0 {
        return brc;
    }
    let c = lambda + 1.0;
    let c0 = b / a;
    let c1 = 1.0 / a + 1.0;
    let yp1 = y + 1.0;
    let (mut n, mut p, mut s) = (0.0f64, 1.0, a + 1.0);
    let (mut an, mut bn) = (0.0f64, 1.0);
    let (mut anp1, mut bnp1) = (1.0f64, c / c1);
    let mut r = c1 / c;
    const TWOPOW_M20: f64 = 9.5367431640625e-7; // 2^-20, R's ldexp(·, -20)
    loop {
        n += 1.0;
        let mut w = n * x * (b - n);
        let rescale = !w.is_finite();
        if rescale {
            w = n * x * (b - n) * TWOPOW_M20;
        }
        let t = n / a;
        let mut e = a / s;
        let alpha = p * (p + c0) * e * e * (w * x);
        e = (t + 1.0) / (c1 + t + t);
        let beta = w / s
            + if rescale {
                (n + e * (c + n * yp1)) * TWOPOW_M20
            } else {
                n + e * (c + n * yp1)
            };
        p = t + 1.0;
        s += 2.0;
        let t2 = alpha * an + beta * anp1;
        an = anp1;
        anp1 = t2;
        let t3 = alpha * bn + beta * bnp1;
        bn = bnp1;
        bnp1 = t3;
        let r0 = r;
        r = anp1 / bnp1;
        if (r - r0).abs() <= eps * r || n >= 1000.0 {
            break;
        }
        an /= bnp1;
        bn /= bnp1;
        anp1 = r;
        bnp1 = 1.0;
    }
    brc * r
}

/// `I_x(a, b)` for large a and b near the mean (lambda ≥ 0, a, b ≥ 15)
/// — `basym` (Temme's expansion), ported verbatim.
fn basym(a: f64, b: f64, lambda: f64, eps: f64) -> f64 {
    const NUM_IT: usize = 20; // maximum n in the do loop; must be even
    const E0: f64 = std::f64::consts::FRAC_2_SQRT_PI; // 2/sqrt(pi)
    const E1: f64 = 0.353553390593274; // 2^(-3/2)
    let mut a0 = [0.0f64; NUM_IT + 1];
    let mut b0 = [0.0f64; NUM_IT + 1];
    let mut c = [0.0f64; NUM_IT + 1];
    let mut d = [0.0f64; NUM_IT + 1];

    let f = a * rlog1(-lambda / a) + b * rlog1(lambda / b);
    let t = libm::exp(-f);
    if t == 0.0 {
        return 0.0;
    }
    let z0 = libm::sqrt(f);
    let z = z0 / E1 * 0.5;
    let z2 = f + f;
    let (h, r0, r1, w0) = if a < b {
        let h = a / b;
        (
            h,
            1.0 / (h + 1.0),
            (b - a) / b,
            1.0 / libm::sqrt(a * (h + 1.0)),
        )
    } else {
        let h = b / a;
        (
            h,
            1.0 / (h + 1.0),
            (b - a) / a,
            1.0 / libm::sqrt(b * (h + 1.0)),
        )
    };
    let mut w = w0;
    a0[0] = r1 * (2.0 / 3.0);
    c[0] = a0[0] * -0.5;
    d[0] = -c[0];
    let mut j0 = 0.5 / E0 * erfcx(z0);
    let mut j1 = E1;
    let mut sum = j0 + d[0] * w * j1;
    let mut s = 1.0;
    let h2 = h * h;
    let mut hn = 1.0;
    let mut znm1 = z;
    let mut zn = z2;
    for n in (2..=NUM_IT).step_by(2) {
        let nf = n as f64;
        hn *= h2;
        a0[n - 1] = r0 * 2.0 * (h * hn + 1.0) / (nf + 2.0);
        let np1 = n + 1;
        s += hn;
        a0[np1 - 1] = r1 * 2.0 * s / (nf + 3.0);
        for i in n..=np1 {
            let r = (i as f64 + 1.0) * -0.5;
            b0[0] = r * a0[0];
            for m in 2..=i {
                let mut bsum = 0.0;
                for j in 1..=m - 1 {
                    let mmj = m - j;
                    bsum += (j as f64 * r - mmj as f64) * a0[j - 1] * b0[mmj - 1];
                }
                b0[m - 1] = r * a0[m - 1] + bsum / m as f64;
            }
            c[i - 1] = b0[i - 1] / (i as f64 + 1.0);
            let mut dsum = 0.0;
            for j in 1..=i - 1 {
                dsum += d[i - j - 1] * c[j - 1];
            }
            d[i - 1] = -(dsum + c[i - 1]);
        }
        j0 = E1 * znm1 + (nf - 1.0) * j0;
        j1 = E1 * zn + nf * j1;
        znm1 *= z2;
        zn *= z2;
        w *= w0;
        let t0 = d[n - 1] * w * j0;
        w *= w0;
        let t1 = d[np1 - 1] * w * j1;
        sum += t0 + t1;
        if t0.abs() + t1.abs() <= eps * sum {
            break;
        }
    }
    E0 * t * libm::exp(-bcorr(a, b)) * sum
}
