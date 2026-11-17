//! `shared`'s closed-form statistic families — proportion CIs
//! (`PropCIs::{wald2ci,addz2ci,scoreci}`), Byar's Poisson
//! approximation and the diagnostic-measure measures (`simel_lr`,
//! `kappa`). Split from `shared.rs` in v1.0 Task 10 to keep both
//! files under the ~400-line budget; `shared` re-exports all of
//! it, so `use super::shared::*` callers are unaffected. The
//! `*_parts` helpers expose the intermediates the explain traces
//! name (spec §6).

use super::shared::z;

// --- closed-form statistic families -----------------------------------------

/// Taylor (Wald) CI for a proportion, in the unit of `x/n`
/// (`risk_ci` in `tables.R`). `stat_ci`-ready.
pub fn prop_taylor(x: u64, n: u64, conf: f64) -> [f64; 3] {
    let p = x as f64 / n as f64;
    let se = libm::sqrt(p * (1.0 - p) / n as f64);
    let h = z(conf) * se;
    [p, p - h, p + h]
}

/// `PropCIs::wald2ci` — the plain Wald interval.
pub fn wald_ci(x: u64, n: u64, conf: f64) -> [f64; 2] {
    let t = prop_taylor(x, n, conf);
    [t[1], t[2]]
}

/// `PropCIs::addz2ci` — Agresti–Coull modified Wald.
pub fn agresti_coull_ci(x: u64, n: u64, conf: f64) -> [f64; 2] {
    let zz = z(conf) * z(conf);
    let (x, n) = (x as f64, n as f64);
    let tn = n + zz;
    let tp = (x + zz / 2.0) / tn;
    let h = z(conf) * libm::sqrt(tp * (1.0 - tp) / tn);
    [(tp - h).max(0.0), (tp + h).min(1.0)]
}

/// The unrounded Wilson score interval.
fn wilson_raw(x: u64, n: u64, conf: f64) -> [f64; 2] {
    let zz = z(conf) * z(conf);
    let (x, n) = (x as f64, n as f64);
    let p = x / n;
    let den = 1.0 + zz / n;
    let center = p + zz / (2.0 * n);
    let half = z(conf) * libm::sqrt(p * (1.0 - p) / n + zz / (4.0 * n * n));
    [(center - half) / den, (center + half) / den]
}

/// `PropCIs::scoreci` — the Wilson score interval, rounded to four
/// decimals as the package reports it.
pub fn scoreci(x: u64, n: u64, conf: f64) -> [f64; 2] {
    let [l, u] = wilson_raw(x, n, conf);
    let r = |v: f64| (v * 1e4).round() / 1e4;
    [r(l), r(u)]
}

/// `wilson_fpc_ci` — Wilson with the `sqrt((N-n)/(N-1))` FPC applied
/// to the score half-width (`fixtures/R/methods/proportion.R`).
pub fn wilson_fpc_ci(x: u64, n: u64, conf: f64, pop: f64) -> [f64; 2] {
    let zz = z(conf) * z(conf);
    let (x, n) = (x as f64, n as f64);
    let p = x / n;
    let fpc = libm::sqrt((pop - n) / (pop - 1.0));
    let r = libm::sqrt(x * (n - x) / (n * n * n) + zz / (4.0 * n * n));
    let lower = if p == 0.0 {
        0.0
    } else {
        (n / (n + zz)) * ((p + zz / (2.0 * n)) - fpc * z(conf) * r)
    };
    let upper = if p == 1.0 {
        1.0
    } else {
        (n / (n + zz)) * ((p + zz / (2.0 * n)) + fpc * z(conf) * r)
    };
    [lower, upper]
}

/// `score_cc_ci` — the Fleiss score interval with continuity
/// correction (`fixtures/R/methods/proportion.R`).
pub fn score_cc_ci(x: u64, n: u64, conf: f64) -> [f64; 2] {
    let zz = z(conf) * z(conf);
    let (x, n) = (x as f64, n as f64);
    let p = x / n;
    let den = 2.0 * (n + zz);
    let lower = if p == 0.0 {
        0.0
    } else {
        ((2.0 * n * p + zz - 1.0)
            - z(conf) * libm::sqrt(zz - (2.0 + 1.0 / n) + 4.0 * p * (n * (1.0 - p) + 1.0)))
            / den
    };
    let upper = if p == 1.0 {
        1.0
    } else {
        ((2.0 * n * p + zz + 1.0)
            + z(conf) * libm::sqrt(zz + (2.0 - 1.0 / n) + 4.0 * p * (n * (1.0 - p) - 1.0)))
            / den
    };
    [lower, upper]
}

/// epitools `pois.byar` — Byar's Poisson approximation on the count
/// scale (divide by person-time for a rate): both limits expand
/// around `x + 0.5`.
pub fn pois_byar(x: u64, conf: f64) -> [f64; 2] {
    let ap = x as f64 + 0.5;
    let zin = (z(conf) / 3.0) * libm::sqrt(1.0 / ap);
    let base = 1.0 - 1.0 / (9.0 * ap);
    [
        ap * libm::pow(base - zin, 3.0),
        ap * libm::pow(base + zin, 3.0),
    ]
}

/// `simel_lr`'s intermediates — `(p1, p2, lr, se)` with
/// `se = sqrt(1/a - 1/(a+c) + 1/b - 1/(b+d))`; `a`/`b` are the row
/// counts of the positive cell in each column. Shared by the run
/// and the explain trace (v1.0 Task 10, spec §6).
pub fn simel_parts(a: u64, c: u64, b: u64, d: u64) -> (f64, f64, f64, f64) {
    let (a, c, b, d) = (a as f64, c as f64, b as f64, d as f64);
    let (p1, p2) = (a / (a + c), b / (b + d));
    let lr = p1 / p2;
    let se = libm::sqrt(1.0 / a - 1.0 / (a + c) + 1.0 / b - 1.0 / (b + d));
    (p1, p2, lr, se)
}

/// The Simel log-method likelihood-ratio CI (`epiR::epi.tests`).
pub fn simel_lr(a: u64, c: u64, b: u64, d: u64, conf: f64) -> [f64; 3] {
    let (_, _, lr, se) = simel_parts(a, c, b, d);
    let h = z(conf) * se;
    [lr, lr * libm::exp(-h), lr * libm::exp(h)]
}

/// `kappa`'s intermediates — the explain trace's named values
/// (v1.0 Task 10, spec §6).
pub struct KappaParts {
    /// `(a + d)/n` — observed agreement.
    pub po: f64,
    /// `r1·c1 + r2·c2` — expected agreement.
    pub pe: f64,
    /// `(po − pe)/(1 − pe)` — Cohen's kappa.
    pub k: f64,
    /// Fleiss Equation 18.13's standard error.
    pub se: f64,
}

/// The intermediates of `kappa(tp, fp, fn_, tn, _)` — shared by
/// the result and the explain trace.
pub fn kappa_parts(tp: u64, fp: u64, fn_: u64, tn: u64) -> KappaParts {
    let (a, b, c, d) = (tp as f64, fp as f64, fn_ as f64, tn as f64);
    let n = a + b + c + d;
    let (r1, r2) = ((a + b) / n, (c + d) / n);
    let (c1, c2) = ((a + c) / n, (b + d) / n);
    let po = (a + d) / n;
    let pe = r1 * c1 + r2 * c2;
    let k = (po - pe) / (1.0 - pe);
    let inside = pe + pe * pe - (r1 * c1 * (r1 + c1) + r2 * c2 * (r2 + c2));
    KappaParts {
        po,
        pe,
        k,
        se: libm::sqrt(inside) / ((1.0 - pe) * libm::sqrt(n)),
    }
}

/// `epiR::epi.kappa` (method "fleiss") — Cohen's kappa with the
/// Fleiss standard error, Fleiss Equation 18.13.
pub fn kappa(tp: u64, fp: u64, fn_: u64, tn: u64, conf: f64) -> [f64; 3] {
    let p = kappa_parts(tp, fp, fn_, tn);
    let h = z(conf) * p.se;
    [p.k, p.k - h, p.k + h]
}
