//! The stock-package exact odds-ratio machinery the calc corpus
//! records: `stats::fisher.test`'s conditional MLE and central
//! interval, `epitools::or.midp`'s mid-p limits on [0, 1000], and
//! `exact2x2`'s `fisherCalcMidp` bounds. Every root is `zeroin` at
//! uniroot's default tolerance — that is what the recorded values
//! are — on the same noncentral hypergeometric tails
//! `stats::exact` uses for the v0.2 corpus.

use crate::special::{zeroin, NcHypergeom, UNIROOT_TOL};

/// `.Machine$double.eps` — the lower end of the `1/t` brackets.
const EPS: f64 = f64::EPSILON;
/// epitools' `or.midp` search interval `[0, 1000]`.
const MIDP_HI: f64 = 1000.0;

/// `fisher.test`'s `mle` at uniroot's default tolerance: 0 at the
/// support minimum, Inf at the maximum, else `mean(psi) = x`.
pub fn cmle_or(h: &NcHypergeom, x: u64) -> f64 {
    let (lo, hi) = h.support();
    if x == lo {
        return 0.0;
    }
    if x == hi {
        return f64::INFINITY;
    }
    let xf = x as f64;
    let mu = h.mean(1.0);
    if mu > xf {
        zeroin(|t| h.mean(t) - xf, 0.0, 1.0, UNIROOT_TOL).unwrap_or(f64::NAN)
    } else if mu < xf {
        zeroin(|t| h.mean(1.0 / t) - xf, EPS, 1.0, UNIROOT_TOL)
            .map(|r| 1.0 / r)
            .unwrap_or(f64::NAN)
    } else {
        1.0
    }
}

/// `fisher.test`'s `ncp.U` at `alpha` (upper central bound).
fn ncp_u(h: &NcHypergeom, x: u64, alpha: f64) -> f64 {
    let (_, hi) = h.support();
    if x == hi {
        return f64::INFINITY;
    }
    let p = h.lower_tail(x, 1.0);
    if p < alpha {
        zeroin(|t| h.lower_tail(x, t) - alpha, 0.0, 1.0, UNIROOT_TOL).unwrap_or(f64::NAN)
    } else if p > alpha {
        zeroin(|t| h.lower_tail(x, 1.0 / t) - alpha, EPS, 1.0, UNIROOT_TOL)
            .map(|r| 1.0 / r)
            .unwrap_or(f64::NAN)
    } else {
        1.0
    }
}

/// `fisher.test`'s `ncp.L` at `alpha` (lower central bound).
fn ncp_l(h: &NcHypergeom, x: u64, alpha: f64) -> f64 {
    let (lo, _) = h.support();
    if x == lo {
        return 0.0;
    }
    let p = h.upper_tail(x, 1.0);
    if p > alpha {
        zeroin(|t| h.upper_tail(x, t) - alpha, 0.0, 1.0, UNIROOT_TOL).unwrap_or(f64::NAN)
    } else if p < alpha {
        zeroin(|t| h.upper_tail(x, 1.0 / t) - alpha, EPS, 1.0, UNIROOT_TOL)
            .map(|r| 1.0 / r)
            .unwrap_or(f64::NAN)
    } else {
        1.0
    }
}

/// `fisher.test`'s central CI (`exact2x2 tsmethod="central"`) at
/// `conf` in percent.
pub fn fisher_ci(h: &NcHypergeom, x: u64, conf: f64) -> [f64; 2] {
    let alpha = (1.0 - conf / 100.0) / 2.0;
    [ncp_l(h, x, alpha), ncp_u(h, x, alpha)]
}

/// `epitools::or.midp` limits at `conf`: roots of `1 - midp(or) =
/// alpha` and `midp(1/or) = alpha` on `[0, 1000]` where
/// `midp(or) = (lower_tail(x,or) - upper_tail(x,or) + 1) / 2`.
pub fn or_midp_ci(h: &NcHypergeom, x: u64, conf: f64) -> [f64; 2] {
    let alpha = (1.0 - conf / 100.0) / 2.0;
    let midp = |or: f64| {
        let (lower, upper) = h.tails(x, or);
        0.5 * (lower - upper + 1.0)
    };
    let lower = zeroin(|or| 1.0 - midp(or) - alpha, 0.0, MIDP_HI, UNIROOT_TOL).unwrap_or(f64::NAN);
    let upper = zeroin(|or| midp(1.0 / or) - alpha, 0.0, MIDP_HI, UNIROOT_TOL)
        .map(|r| 1.0 / r)
        .unwrap_or(f64::NAN);
    [lower, upper]
}

/// `exact2x2`'s `fisherCalcMidp` bounds at `conf`: `pnhyperMidp(x,
/// ncp) = tail(x, ncp) - 0.5*pmf(x, ncp)` roots, support-boundary
/// cases first as the package defines them.
pub fn midp_ci(h: &NcHypergeom, x: u64, conf: f64) -> [f64; 2] {
    let alpha = (1.0 - conf / 100.0) / 2.0;
    let (lo, hi) = h.support();
    let pmid = |q: u64, ncp: f64, upper: bool| {
        let (lt, ut) = h.tails(q, ncp);
        let tail = if upper { ut } else { lt };
        tail - 0.5 * h.pmf(q, ncp)
    };
    let lower = if x == lo {
        0.0
    } else if x == hi && alpha >= 0.5 {
        f64::INFINITY
    } else {
        let p = pmid(x, 1.0, true);
        if p > alpha {
            zeroin(|t| pmid(x, t, true) - alpha, 0.0, 1.0, UNIROOT_TOL).unwrap_or(f64::NAN)
        } else if p < alpha {
            zeroin(|t| pmid(x, 1.0 / t, true) - alpha, EPS, 1.0, UNIROOT_TOL)
                .map(|r| 1.0 / r)
                .unwrap_or(f64::NAN)
        } else {
            1.0
        }
    };
    let upper = if x == hi {
        f64::INFINITY
    } else if x == lo && alpha >= 0.5 {
        0.0
    } else {
        let p = pmid(x, 1.0, false);
        if p < alpha {
            zeroin(|t| pmid(x, t, false) - alpha, 0.0, 1.0, UNIROOT_TOL).unwrap_or(f64::NAN)
        } else if p > alpha {
            zeroin(|t| pmid(x, 1.0 / t, false) - alpha, EPS, 1.0, UNIROOT_TOL)
                .map(|r| 1.0 / r)
                .unwrap_or(f64::NAN)
        } else {
            1.0
        }
    };
    [lower, upper]
}
