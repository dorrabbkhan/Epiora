//! `brent_root` — Brent's method (spec §3.4): absolute tolerance
//! 1e-14, at most 200 iterations. Ported from Numerical Recipes'
//! `zbrent`.

/// Why a root solve failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootError {
    /// `f(lo)` and `f(hi)` have the same sign.
    NotBracketed,
    /// 200 iterations did not reach 1e-14.
    NoConvergence,
}

/// The root of `f` on `[lo, hi]` to absolute tolerance 1e-14.
pub fn brent_root(mut f: impl FnMut(f64) -> f64, lo: f64, hi: f64) -> Result<f64, RootError> {
    // TOL is the half-width: the root stays bracketed inside
    // [b, c], so |b − root| ≤ 2·TOL = 1e-14.
    const TOL: f64 = 5e-15;
    const MAXIT: usize = 200;

    let (mut a, mut b, mut c) = (lo, hi, lo);
    let mut fa = f(a);
    let mut fb = f(b);
    if fa == 0.0 {
        return Ok(a);
    }
    if fb == 0.0 {
        return Ok(b);
    }
    if (fa > 0.0) == (fb > 0.0) {
        return Err(RootError::NotBracketed);
    }
    let mut fc = fa;
    let mut d = b - a;
    let mut e = d;
    for _ in 0..MAXIT {
        if (fb > 0.0) == (fc > 0.0) {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let m = 0.5 * (c - b);
        if m.abs() <= TOL || fb == 0.0 {
            return Ok(b);
        }
        if e.abs() >= TOL && fa.abs() > fb.abs() {
            // inverse quadratic interpolation / secant step
            let s = fb / fa;
            let (p, q) = if a == c {
                (2.0 * m * s, 1.0 - s)
            } else {
                let q1 = fa / fc;
                let r = fb / fc;
                (
                    s * (2.0 * m * q1 * (q1 - r) - (b - a) * (r - 1.0)),
                    (q1 - 1.0) * (r - 1.0) * (s - 1.0),
                )
            };
            let (p, q) = if p > 0.0 { (p, -q) } else { (-p, q) };
            let bound1 = 3.0 * m * q - (TOL * q).abs();
            let bound2 = (e * q).abs();
            if 2.0 * p < bound1.min(bound2) {
                e = d;
                d = p / q;
            } else {
                d = m;
                e = m;
            }
        } else {
            d = m;
            e = m;
        }
        a = b;
        fa = fb;
        if d.abs() > TOL {
            b += d;
        } else {
            b += if m >= 0.0 { TOL } else { -TOL };
        }
        fb = f(b);
    }
    Err(RootError::NoConvergence)
}
