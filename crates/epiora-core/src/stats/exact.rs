//! The iterative 2×2 statistics: conditional MLE odds ratio, Fisher
//! exact and mid-p exact confidence limits, and the two-sided p-values.
//! These are the same equations as R's `fisher.test` (`mle`, `ncp.L`,
//! `ncp.U` and the two-sided `relErr` sum) and epitools' `or.midp` /
//! `ormidp.test`, ported in `fixtures/R/nchyper.R`; R's `uniroot`
//! becomes `brent_root` (abs tol 1e-14) and every failure is
//! `Undefined { NotBracketed | NoConvergence }` (spec §8).
//!
//! Layout (as in `fisher.test`): `X | margins` is the `[a]` cell of
//! `[[a,b],[c,d]]` with column totals `m = a+c`, `n = b+d` and row-1
//! total `k = a+b`.

use super::estimate::{Ci, Estimate, UndefinedReason};
use crate::special::{brent_root, hypergeom_pmf, NcHypergeom, RootError};

/// `(1 − 0.95)/2` — both exact CIs are 95%.
const ALPHA: f64 = 0.025;
/// R's `.Machine$double.eps`: lower end of the `1/t` root brackets.
const EPS: f64 = f64::EPSILON;
/// epitools' `or.midp` search interval `[0, 1000]` on the odds ratio.
const MIDP_HI: f64 = 1000.0;

fn root_failure(e: RootError) -> Estimate {
    let reason = match e {
        RootError::NotBracketed => UndefinedReason::NotBracketed,
        RootError::NoConvergence => UndefinedReason::NoConvergence,
    };
    Estimate::Undefined { reason }
}

/// The conditional MLE odds ratio — `fisher.test`'s `mle`: 0 when the
/// observed cell is at the support minimum, `Undefined { Unbounded }`
/// at the maximum, else `mean(psi) = x` solved by `brent_root`.
pub fn cmle(h: &NcHypergeom, x: u64) -> Estimate {
    let (lo, hi) = h.support();
    if x == lo {
        return Estimate::Value(0.0);
    }
    if x == hi {
        return Estimate::Undefined {
            reason: UndefinedReason::Unbounded,
        };
    }
    let xf = x as f64;
    let mu = h.mean(1.0);
    if mu > xf {
        match brent_root(|t| h.mean(t) - xf, 0.0, 1.0) {
            Ok(root) => Estimate::Value(root),
            Err(e) => root_failure(e),
        }
    } else if mu < xf {
        match brent_root(|t| h.mean(1.0 / t) - xf, EPS, 1.0) {
            Ok(root) => Estimate::Value(1.0 / root),
            Err(e) => root_failure(e),
        }
    } else {
        Estimate::Value(1.0)
    }
}

/// `fisher.test`'s `ncp.U` at `alpha`: `Undefined { Unbounded }` when
/// the observed cell is at the support maximum, else the root of
/// `lower_tail(x, psi) = alpha`.
fn ncp_u(h: &NcHypergeom, x: u64) -> Estimate {
    let (_, hi) = h.support();
    if x == hi {
        return Estimate::Undefined {
            reason: UndefinedReason::Unbounded,
        };
    }
    let p = h.lower_tail(x, 1.0);
    if p < ALPHA {
        match brent_root(|t| h.lower_tail(x, t) - ALPHA, 0.0, 1.0) {
            Ok(root) => Estimate::Value(root),
            Err(e) => root_failure(e),
        }
    } else if p > ALPHA {
        match brent_root(|t| h.lower_tail(x, 1.0 / t) - ALPHA, EPS, 1.0) {
            Ok(root) => Estimate::Value(1.0 / root),
            Err(e) => root_failure(e),
        }
    } else {
        Estimate::Value(1.0)
    }
}

/// `fisher.test`'s `ncp.L` at `alpha`: 0 at the support minimum, else
/// the root of `upper_tail(x, psi) = alpha`.
fn ncp_l(h: &NcHypergeom, x: u64) -> Estimate {
    let (lo, _) = h.support();
    if x == lo {
        return Estimate::Value(0.0);
    }
    let p = h.upper_tail(x, 1.0);
    if p > ALPHA {
        match brent_root(|t| h.upper_tail(x, t) - ALPHA, 0.0, 1.0) {
            Ok(root) => Estimate::Value(root),
            Err(e) => root_failure(e),
        }
    } else if p < ALPHA {
        match brent_root(|t| h.upper_tail(x, 1.0 / t) - ALPHA, EPS, 1.0) {
            Ok(root) => Estimate::Value(1.0 / root),
            Err(e) => root_failure(e),
        }
    } else {
        Estimate::Value(1.0)
    }
}

/// The Fisher exact 95% CI on the odds ratio (`fisher.test`'s CINT).
pub fn fisher_ci(h: &NcHypergeom, x: u64) -> Ci {
    Ci {
        lower: ncp_l(h, x),
        upper: ncp_u(h, x),
    }
}

/// epitools' mid-p tail: `0.5 * (P(X ≤ x | psi) − P(X ≥ x | psi) + 1)`.
/// `tails` computes both tails from one weights window; each is the
/// identical `f64` `lower_tail`/`upper_tail` returns (Task 11b).
fn midp(h: &NcHypergeom, x: u64, psi: f64) -> f64 {
    let (lower, upper) = h.tails(x, psi);
    0.5 * (lower - upper + 1.0)
}

/// The mid-p exact 95% CI — epitools `or.midp`'s LCL/UCL roots on
/// `[0, 1000]`. When the observed cell is at either end of the
/// conditional support the reference reports no interval, so both
/// bounds are `Undefined { Unbounded }` without root-finding; inside
/// the support an unbracketed root is `Undefined { NotBracketed }`.
pub fn midp_ci(h: &NcHypergeom, x: u64) -> Ci {
    let (lo, hi) = h.support();
    if x == lo || x == hi {
        let u = || Estimate::Undefined {
            reason: UndefinedReason::Unbounded,
        };
        return Ci {
            lower: u(),
            upper: u(),
        };
    }
    let lower = match brent_root(|or| 1.0 - midp(h, x, or) - ALPHA, 0.0, MIDP_HI) {
        Ok(root) => Estimate::Value(root),
        Err(e) => root_failure(e),
    };
    let upper = match brent_root(|or| midp(h, x, 1.0 / or) - ALPHA, 0.0, MIDP_HI) {
        Ok(root) => Estimate::Value(1.0 / root),
        Err(e) => root_failure(e),
    };
    Ci { lower, upper }
}

/// epitools `ormidp.test`'s two-sided mid-p at `psi = 1`:
/// `2 * min(p, 1 − p)` with `p = 0.5 * (P(X ≤ x) − P(X ≥ x) + 1)`.
pub fn midp_p_two_sided(h: &NcHypergeom, x: u64) -> f64 {
    let p1 = midp(h, x, 1.0);
    (2.0 * p1.min(1.0 - p1)).clamp(0.0, 1.0)
}

/// `fisher.test`'s two-sided p: the sum of table probabilities at most
/// `relErr = 1 + 1e-7` times the observed table's probability, over the
/// full support `max(0, k−n) ..= min(k, m)`.
///
/// Construction: the hypergeometric pmf is unimodal, so the qualifying
/// set always contains the whole tail from `x` outward to the support
/// edge away from the mode — every `i <= x` when `x` is below the
/// mode, every `i >= x` when above. Each candidate below anchors on
/// the identical `f64` `lower_tail`/`upper_tail` returns for `x`
/// (same terms, same order, so it equals the reported one-sided p)
/// and adds only the qualifying mass on the other side. At least one
/// of the two therefore reproduces `fisher.test`'s sum exactly while
/// the other merely adds non-qualifying tail mass, so their `min` is
/// the two-sided value; and because each rest is a sum of
/// nonnegatives, the result is `>= min(p_lower, p_upper)` in floating
/// point by construction — no tolerance involved.
pub fn fisher_p_two_sided(h: &NcHypergeom, m: u64, n: u64, k: u64, x: u64) -> f64 {
    let (lo, hi) = h.support();
    let observed = hypergeom_pmf(x, m, n, k);
    let threshold = observed * (1.0 + 1e-7);
    let mut below = 0.0;
    let mut above = 0.0;
    for i in lo..=hi {
        if i == x {
            continue;
        }
        let d = hypergeom_pmf(i, m, n, k);
        if d <= threshold {
            if i < x {
                below += d;
            } else {
                above += d;
            }
        }
    }
    // `tails` evaluates one weights window for both tails; each is
    // the identical `f64` `lower_tail`/`upper_tail` returns (Task 11b).
    let (lower, upper) = h.tails(x, 1.0);
    (lower + above).min(upper + below).clamp(0.0, 1.0)
}
