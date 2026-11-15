//! `zeroin` — a bit-faithful port of R's `R_zeroin2`
//! (`src/library/stats/src/zeroin.c`, the Brent/Dekker algorithm
//! `uniroot` calls). The calculator fixtures record stock-package
//! roots — `fisher.test`, `epitools::or.midp`, `exactci` — all of
//! which call `uniroot` at its default tolerance, so the engine
//! needs the identical iteration sequence and the identical
//! stopping rule, not merely the same root.

use super::root::RootError;

/// uniroot's default tolerance: `.Machine$double.eps^0.25`.
pub const UNIROOT_TOL: f64 = 0.0001220703125;

const EPSILON: f64 = f64::EPSILON;

/// `uniroot(f, c(lo, hi), tol)` — the R_zeroin2 iteration over a
/// bracket on which `f` changes sign. `f(lo)` and `f(hi)` are
/// evaluated first (as uniroot does), an equal-sign pair extends
/// nothing here — callers pass only brackets that already confine
/// the root. Errors are `RootError::NotBracketed` /
/// `RootError::NoConvergence`.
pub fn zeroin(f: impl Fn(f64) -> f64, lo: f64, hi: f64, tol: f64) -> Result<f64, RootError> {
    let mut fa = f(lo);
    let mut fb = f(hi);
    if fa == 0.0 {
        return Ok(lo);
    }
    if fb == 0.0 {
        return Ok(hi);
    }
    if fa * fb > 0.0 || !fa.is_finite() || !fb.is_finite() {
        return Err(RootError::NotBracketed);
    }

    let mut a = lo;
    let mut b = hi;
    let mut c = a;
    let mut fc = fa;
    let mut maxit = 1001i64; // R_zeroin2: *Maxit + 1 with Maxit = 1000

    while maxit > 0 {
        maxit -= 1;
        let prev_step = b - a;
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let tol_act = 2.0 * EPSILON * b.abs() + tol / 2.0;
        let mut new_step = (c - b) / 2.0;

        if new_step.abs() <= tol_act || fb == 0.0 {
            return Ok(b);
        }

        if prev_step.abs() >= tol_act && fa.abs() > fb.abs() {
            let cb = c - b;
            let (mut p, mut q);
            if a == c {
                let t1 = fb / fa;
                p = cb * t1;
                q = 1.0 - t1;
            } else {
                let rq = fa / fc;
                let t1 = fb / fc;
                let t2 = fb / fa;
                p = t2 * (cb * rq * (rq - t1) - (b - a) * (t1 - 1.0));
                q = (rq - 1.0) * (t1 - 1.0) * (t2 - 1.0);
            }
            if p > 0.0 {
                q = -q;
            } else {
                p = -p;
            }
            if p < (0.75 * cb * q - (tol_act * q).abs() / 2.0) && p < (prev_step * q / 2.0).abs() {
                new_step = p / q;
            }
        }

        if new_step.abs() < tol_act {
            if new_step > 0.0 {
                new_step = tol_act;
            } else {
                new_step = -tol_act;
            }
        }
        a = b;
        fa = fb;
        b += new_step;
        fb = f(b);
        if (fb > 0.0 && fc > 0.0) || (fb < 0.0 && fc < 0.0) {
            c = a;
            fc = fa;
        }
    }
    Err(RootError::NoConvergence)
}
