//! Exact-CI helpers for the counts calculators (continued from
//! `shared.rs`): the `exactci` binomial/Poisson central and mid-p
//! intervals, which `zeroin` at R `uniroot`'s default tolerance
//! reproduces bit for bit.

use super::shared::{dbinom, dpois};
use crate::special::{binomial_tail, gamma_quantile};

/// `exactci::binom.exact`'s mid-p `p.L(x, alpha)`: the uniroot
/// (default tolerance) of `P(X >= x) - dbinom(x)/2 - alpha` on
/// [0, 1] — `pbinom(x-1, lower.tail=FALSE)` in R's spelling.
fn binom_midp_lower(x: u64, n: u64, alpha: f64) -> f64 {
    if x == 0 {
        return 0.0;
    }
    if x == n && alpha >= 0.5 {
        return 1.0;
    }
    crate::special::zeroin(
        |p| binomial_tail(x - 1, n, p, false) - 0.5 * dbinom(x, n, p) - alpha,
        0.0,
        1.0,
        crate::special::UNIROOT_TOL,
    )
    .unwrap_or(f64::NAN)
}

/// `exactci::binom.exact`'s mid-p `p.U(x, alpha)`.
fn binom_midp_upper(x: u64, n: u64, alpha: f64) -> f64 {
    if x == n {
        return 1.0;
    }
    if x == 0 && alpha >= 0.5 {
        return 0.0;
    }
    crate::special::zeroin(
        |p| binomial_tail(x, n, p, true) - 0.5 * dbinom(x, n, p) - alpha,
        0.0,
        1.0,
        crate::special::UNIROOT_TOL,
    )
    .unwrap_or(f64::NAN)
}

/// `exactci::binom.exact` two-sided mid-p CI; `conf` in percent.
pub fn binom_midp_ci(x: u64, n: u64, conf: f64) -> [f64; 2] {
    let alpha = (1.0 - conf / 100.0) / 2.0;
    [binom_midp_lower(x, n, alpha), binom_midp_upper(x, n, alpha)]
}

/// `exactci::binom.exact` central (Clopper–Pearson) CI; `conf` in percent.
pub fn binom_cp_ci(x: u64, n: u64, conf: f64) -> [f64; 2] {
    let alpha = (1.0 - conf / 100.0) / 2.0;
    let lower = if x == 0 {
        0.0
    } else {
        crate::special::beta_quantile(alpha, x as f64, (n - x + 1) as f64).unwrap_or(f64::NAN)
    };
    let upper = if x == n {
        1.0
    } else {
        crate::special::beta_quantile(1.0 - alpha, (x + 1) as f64, (n - x) as f64)
            .unwrap_or(f64::NAN)
    };
    [lower, upper]
}

/// `exactci::poisson.exact`'s central CI on the expected count scale
/// (rate CIs scale by `1/T`): `[qgamma(alpha, x), qgamma(1-alpha, x+1)]`.
pub fn pois_fisher_ci(x: u64, conf: f64) -> [f64; 2] {
    let alpha = (1.0 - conf / 100.0) / 2.0;
    let lower = if x == 0 {
        0.0
    } else {
        gamma_quantile(alpha, x as f64, 1.0).unwrap_or(f64::NAN)
    };
    let upper = gamma_quantile(1.0 - alpha, (x + 1) as f64, 1.0).unwrap_or(f64::NAN);
    [lower, upper]
}

/// `exactci::poisson.exact`'s mid-p CI on the expected count scale:
/// `mu.L` solves `1 - ppois(x, mu) + dpois(x, mu)/2 = alpha` on
/// `[0, qgamma(alpha, x) + 1]`; `mu.U` solves `ppois(x, mu) -
/// dpois(x, mu)/2 = alpha` on `[0, qgamma(1-alpha, x+1) + 1]`,
/// both at uniroot's default tolerance.
pub fn pois_midp_ci(x: u64, conf: f64) -> [f64; 2] {
    let alpha = (1.0 - conf / 100.0) / 2.0;
    let lower = if x == 0 {
        0.0
    } else {
        let hi = gamma_quantile(alpha, x as f64, 1.0).unwrap_or(0.0) + 1.0;
        crate::special::zeroin(
            |m| 1.0 - crate::special::poisson_tail(x, m, false) + 0.5 * dpois(x, m) - alpha,
            0.0,
            hi,
            crate::special::UNIROOT_TOL,
        )
        .unwrap_or(f64::NAN)
    };
    let hi = gamma_quantile(1.0 - alpha, (x + 1) as f64, 1.0).unwrap_or(0.0) + 1.0;
    let upper = crate::special::zeroin(
        |m| crate::special::poisson_tail(x, m, false) - 0.5 * dpois(x, m) - alpha,
        0.0,
        hi,
        crate::special::UNIROOT_TOL,
    )
    .unwrap_or(f64::NAN);
    [lower, upper]
}

/// `exactci::poisson.exact` two-sided p at `lambda`: central is
/// `min(1, 2*min(less, greater))`; mid-p halves the point mass.
pub fn pois_pvals(x: u64, lambda: f64) -> (f64, f64) {
    let less = crate::special::poisson_tail(x, lambda, false);
    let greater = if x == 0 {
        1.0
    } else {
        crate::special::poisson_tail(x, lambda, true)
    };
    let fisher = (2.0 * less.min(greater)).min(1.0);
    let pm = 0.5 * dpois(x, lambda);
    let midp = (2.0 * (less - pm).min(greater - pm)).min(1.0);
    (fisher, midp)
}
