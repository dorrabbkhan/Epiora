//! The Martin & Austin stratified conditional-MLE machinery of
//! `fixtures/R/methods/tables.R` — the conditional distribution of
//! the sum of the strata's `a` cells is the convolution of each
//! stratum's hypergeometric coefficient polynomial, kept in log
//! space and max-normalised. The CMLE and the mid-p/Fisher limits
//! are roots of that distribution's tails; the corpus records
//! `uniroot(tol = 1e-14)` roots, so `brent_root` suffices.

use crate::special::{brent_root, dhyper};
use crate::stats::Table2x2;

/// Per-stratum polynomial: log coefficients `lchoose(n1,k) +
/// lchoose(n0, m1-k)` over `k in min_a..=max_a`, max-normalised —
/// computed as `dhyper`'s log mass (`dbinom_raw`'s `stirlerr`/`bd0`
/// composition), which differs from the `lchoose` sum only by the
/// constant `−lchoose(n1+n0, m1)` the max-normalisation absorbs; the
/// `ln_gamma` difference form loses all accuracy once the margins
/// swamp it (fix round 2). Non-informative strata (`ad == 0 &&
/// bc == 0`) carry a unit polynomial and no support.
struct Poly {
    lp: Vec<f64>,
    min_a: u64,
    max_a: u64,
}

fn poly_strat_cc(t: &Table2x2) -> Option<Poly> {
    let informative = (t.a * t.d != 0) || (t.b * t.c != 0);
    if !informative {
        return None;
    }
    let m1 = t.a + t.c;
    let n1 = t.a + t.b;
    let n0 = t.c + t.d;
    let min_a = m1.saturating_sub(n0);
    let max_a = m1.min(n1);
    let mut lp: Vec<f64> = (min_a..=max_a)
        .map(|k| dhyper(k as f64, n1 as f64, n0 as f64, m1 as f64, true))
        .collect();
    let mx = lp.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    for v in &mut lp {
        *v -= mx;
    }
    Some(Poly { lp, min_a, max_a })
}

/// `logmult_poly` — convolution in log space, `log(exp(a)+exp(b))`
/// accumulation, output re-anchored to max 0.
fn logmult_poly(lp1: &[f64], lp2: &[f64]) -> Vec<f64> {
    let (n1, n2) = (lp1.len(), lp2.len());
    let mut out = vec![f64::NEG_INFINITY; n1 + n2 - 1];
    for (j, &l2) in lp2.iter().enumerate() {
        for (i, &l1) in lp1.iter().enumerate() {
            let lt = l1 + l2;
            let prev = out[i + j];
            let m = prev.max(lt);
            out[i + j] = if m == f64::NEG_INFINITY {
                f64::NEG_INFINITY
            } else {
                m + libm::log(libm::exp(prev - m) + libm::exp(lt - m))
            };
        }
    }
    let mx = out.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    for v in &mut out {
        *v -= mx;
    }
    out
}

/// `logeval_poly` — `log(sum_k c_k r^k)` via log-sum-exp.
fn logeval_poly(lc: &[f64], r: f64) -> f64 {
    if r == 0.0 {
        return lc[0];
    }
    let lt: Vec<f64> = lc
        .iter()
        .enumerate()
        .map(|(i, &c)| c + (i as f64) * libm::log(r))
        .collect();
    let m = lt.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if m == f64::NEG_INFINITY {
        return f64::NEG_INFINITY;
    }
    m + libm::log(lt.iter().map(|&t| libm::exp(t - m)).sum::<f64>())
}

struct Setup {
    lp_d: Vec<f64>,
    sum_a: u64,
    min_sum: u64,
    max_sum: u64,
}

fn martin_setup(strata: &[Table2x2]) -> Setup {
    let (mut sum_a, mut min_sum, mut max_sum) = (0, 0, 0);
    let mut lp_d: Option<Vec<f64>> = None;
    for t in strata {
        if let Some(p) = poly_strat_cc(t) {
            sum_a += t.a;
            min_sum += p.min_a;
            max_sum += p.max_a;
            lp_d = Some(match lp_d {
                None => p.lp,
                Some(ref prev) => logmult_poly(prev, &p.lp),
            });
        }
    }
    Setup {
        lp_d: lp_d.unwrap_or_else(|| vec![0.0]),
        sum_a,
        min_sum,
        max_sum,
    }
}

fn martin_func(lp_n: &[f64], lp_d: &[f64], r: f64, value: f64) -> f64 {
    if r > 0.0 {
        return libm::exp(logeval_poly(lp_n, r) - logeval_poly(lp_d, r)) - value;
    }
    let jn = lp_n.iter().position(|x| x.is_finite());
    let jd = lp_d.iter().position(|x| x.is_finite());
    match (jn, jd) {
        (None, _) | (Some(_), None) => -value,
        (Some(a), Some(b)) if a > b => -value,
        (Some(a), Some(b)) if a < b => f64::INFINITY - value,
        (Some(a), Some(b)) => libm::exp(lp_n[a] - lp_d[b]) - value,
    }
}

/// `martin_root` — the expanding bracket `x0=0, x1=max(0.5,approx)`
/// grown by `x1 *= 1.5 * iter`, then a root on the bracket; the
/// corpus value uses `uniroot(tol=1e-14)`, so `brent_root` returns
/// the same root to its own tolerance.
fn martin_root(lp_n: &[f64], lp_d: &[f64], value: f64, approx: f64) -> f64 {
    let f = |r: f64| martin_func(lp_n, lp_d, r, value);
    let mut x0 = 0.0;
    let mut x1 = if approx.is_finite() {
        approx.max(0.5)
    } else {
        1.0
    };
    let mut f0 = f(x0);
    let mut f1 = f(x1);
    let mut iter = 0u64;
    while f1 * f0 > 0.0 && iter < 10000 {
        iter += 1;
        x0 = x1;
        f0 = f1;
        x1 *= 1.5 * iter as f64;
        if !x1.is_finite() {
            return f64::NAN;
        }
        f1 = f(x1);
    }
    if f1 * f0 > 0.0 {
        return f64::NAN;
    }
    brent_root(f, x0, x1).unwrap_or(f64::NAN)
}

/// `stratified_cmle_one` — est plus mid-p and Fisher limits at `conf`
/// (percent). `NA` in the port becomes NaN here, which the leaf
/// writer records as `{"undefined"}` — matching the corpus's null
/// leaves for unbracketed roots.
pub fn stratified_cmle(strata: &[Table2x2], conf: f64) -> [f64; 5] {
    let s = martin_setup(strata);
    let (a, b, c, d) = strata.iter().fold((0u64, 0u64, 0u64, 0u64), |m, t| {
        (m.0 + t.a, m.1 + t.b, m.2 + t.c, m.3 + t.d)
    });
    let approx = ((a as f64) * (d as f64) / ((b as f64) * (c as f64)).max(1e-8)).max(1e-8);

    let est = if s.min_sum == s.max_sum {
        f64::NAN
    } else if s.sum_a == s.min_sum {
        0.0
    } else if s.sum_a == s.max_sum {
        f64::INFINITY
    } else {
        let lp_n: Vec<f64> = s
            .lp_d
            .iter()
            .enumerate()
            .map(|(i, &l)| libm::log((s.min_sum + i as u64) as f64) + l)
            .collect();
        martin_root(&lp_n, &s.lp_d, s.sum_a as f64, approx)
    };

    let lim = |lower: bool, fisher: bool| -> f64 {
        let diff = s.sum_a - s.min_sum;
        let value = if lower {
            0.5 * (1.0 + conf / 100.0)
        } else {
            0.5 * (1.0 - conf / 100.0)
        };
        if lower && s.sum_a == s.min_sum {
            return 0.0;
        }
        if !lower && s.sum_a == s.max_sum {
            return f64::INFINITY;
        }
        let deg_n = if lower && fisher {
            diff as i64 - 1
        } else {
            diff as i64
        };
        if deg_n < 0 {
            return 0.0;
        }
        let mut lp_n: Vec<f64> = s.lp_d[..=deg_n as usize].to_vec();
        if !fisher {
            let last = lp_n.len() - 1;
            lp_n[last] += libm::log(0.5);
        }
        martin_root(&lp_n, &s.lp_d, value, approx)
    };

    [
        est,
        lim(true, false),
        lim(false, false),
        lim(true, true),
        lim(false, true),
    ]
}
