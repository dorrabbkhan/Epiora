//! `qbinom(p, n, prob)` — R 4.6.2 `src/nmath/qbinom.c` +
//! `src/nmath/qDiscrete_search.h`, ported for the
//! `lower_tail = TRUE, log_p = FALSE` form the engine exposes
//! (spec §4.1): Cornish–Fisher initial value, the
//! `p * (1 − 8·DBL_EPSILON)` left-continuity fuzz, unit-step search
//! for `y < 4096`, and the shrinking-increment loop for larger y.
//! The CDF is `pbinom`'s `pbeta(prob, k+1, n−k, lower.tail = FALSE)`
//! (pbinom.c L44-47) — the upper incomplete-beta tail evaluated
//! directly by `reg_inc_beta_upper`, so the boundary quantiles R
//! distinguishes at ~1e-15 resolve the same way.

use super::beta::reg_inc_beta_upper;
use super::normal::normal_quantile;

/// `pbinom(x, n, p, lower_tail = TRUE)` — pbinom.c L44-47: `x` is a
/// double inside the search (increments, the Cornish–Fisher start),
/// and `floor(x + 1e-7)` is R's integer rounding.
fn pbinom(x: f64, n: f64, pr: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    let x = libm::floor(x + 1e-7);
    if n <= x {
        return 1.0;
    }
    reg_inc_beta_upper(pr, x + 1.0, n - x)
}

/// `do_search` — qDiscrete_search.h L61-126, `lower_tail = TRUE`:
/// `left ⇔ *z >= p`. Returns `(y, z)` — the search carries the last
/// evaluated CDF in `z` for the next call. `incr` is integral ≥ 1.
fn do_search(mut y: f64, mut z: f64, p: f64, n: f64, pr: f64, incr: f64) -> (f64, f64) {
    if z >= p {
        // L70-88: search to the left.
        loop {
            let mut newz = -1.0;
            if y > 0.0 {
                newz = pbinom(y - incr, n, pr);
            } else if y < 0.0 {
                y = 0.0;
            }
            // newz may be NaN from border-line _pDIST_ behaviour.
            if y == 0.0 || newz.is_nan() || newz < p {
                return (y, z); // previous z
            }
            y = libm::fmax(0.0, y - incr);
            z = newz;
        }
    }
    // L91-124: search to the right.
    loop {
        let prevy = y;
        let mut newz = -1.0;
        y += incr;
        if y < n {
            newz = pbinom(y, n, pr);
        } else if y > n {
            y = n;
        }
        if y == n || newz.is_nan() || newz >= p {
            if incr <= 1.0 {
                return (y, newz);
            }
            return (prevy, z);
        }
        z = newz;
    }
}

/// `qbinom(p, n, prob)` — qbinom.c L53-87 with `qDiscrete_search.h`'s
/// `q_DISCRETE_BODY` (L165-224). NaN/`p` outside `[0, 1]` map to the
/// integer edges (R warns and returns NaN; `u64` has no NaN channel),
/// matching the engine's previous contract.
pub fn binomial_quantile(p: f64, n: u64, prob: f64) -> u64 {
    if p.is_nan() || prob.is_nan() {
        return 0;
    }
    let nf = n as f64; // R's n is a double: R_forceint(u64) is exact
                       // L70 R_Q_P01_boundaries(p, 0, n): p outside [0,1] is NaN in R;
                       // the edges are p = 0 -> 0, p = 1 -> n.
    if p <= 0.0 {
        return 0;
    }
    if p >= 1.0 {
        return n;
    }
    let pr = prob;
    // L67/L72-73: invalid pr is NaN in R — clamped to the edges here,
    // as before; the degenerate point masses come after boundaries.
    if pr <= 0.0 || n == 0 {
        return 0;
    }
    if pr >= 1.0 {
        return n;
    }
    // L75-80: Cornish–Fisher moments.
    let q = 1.0 - pr;
    let mu = nf * pr;
    let sigma = libm::sqrt(nf * pr * q);
    let gamma = (q - pr) / sigma;
    // q_DISCRETE_BODY L169-171: z = qnorm(p), then the C-F estimate;
    // R_forceint is nearbyint (ties to even) on this platform.
    let z = normal_quantile(p).unwrap_or(0.0);
    let mut y = libm::rint(mu + sigma * (z + gamma * (z * z - 1.0) / 6.0));
    // q_DISCR_CHECK_BOUNDARY with _dist_MAX_y = n.
    if y > nf {
        y = nf;
    } else if y < 0.0 {
        y = 0.0;
    }
    let mut z = pbinom(y, nf, pr);
    // L196-203: left-continuity fuzz — e = _pf_n_·DBL_EPSILON = 8ε,
    // lower tail => p *= 1 - e.
    let pf = p * (1.0 - 8.0 * f64::EPSILON);
    // L206: unit-step search while the C-F value is small.
    if y < 4096.0 {
        return do_search(y, z, pf, nf, pr, 1.0).0 as u64;
    }
    // L210-223: large y — increments shrink by _iShrink_ = 8 until
    // unit steps, starting at floor(y·(1/64)) = floor(y·_incF_).
    let mut incr = libm::floor(y * (1.0 / 64.0));
    loop {
        let oldincr = incr;
        let (yy, zz) = do_search(y, z, pf, nf, pr, incr);
        y = yy;
        z = zz;
        incr = libm::fmax(1.0, libm::floor(incr / 8.0));
        if !(oldincr > 1.0 && incr > y * 1e-15) {
            break;
        }
    }
    y as u64
}
