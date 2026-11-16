//! Shared helpers for the counts calculators: canonical-input
//! accessors, leaf writers and the closed-form statistics several
//! calculators share (proportion CIs, Poisson CIs, the Simel
//! likelihood-ratio CI and the z/tail evaluations used across the
//! family). Iterative reference values are in `exact_or.rs` and
//! `martin.rs`.

use crate::special::{
    chisq_sf, dbinom_raw, dpois_raw, normal_quantile, reg_inc_beta, reg_inc_beta_upper,
};
use crate::stats::estimate::UndefinedReason;
use serde_json::{json, Value};

/// The `undefined` reason a non-finite result falls back to.
const FALLBACK: UndefinedReason = UndefinedReason::OutOfRange;

/// `x` as a JSON leaf: a number, or `{"undefined": reason}` when not
/// finite — the shape every calculator result tree carries.
pub fn leaf(x: f64) -> Value {
    if x.is_finite() {
        Value::from(x)
    } else {
        undefined(FALLBACK.code())
    }
}

/// An `{"undefined": code}` leaf.
pub fn undefined(code: &str) -> Value {
    json!({ "undefined": code })
}

/// Owner decision (v1.0 Task 7 follow-up, recorded on PR #74): an
/// exact-path leaf turns `{"undefined": "too_large"}` when its
/// stratum's hypergeometric support — `max − min + 1` of cell `a`
/// given the margins — exceeds this. Checked before any allocation
/// or enumeration.
pub const MAX_EXACT_SUPPORT: u64 = 1_000_000;

/// Same owner decision: `two_by_two`'s `adjusted.cmle_or.*` turns
/// `too_large` when `martin`'s convolution work exceeds this. For two
/// informative strata the work is `support₁ · support₂`; for `k > 2`
/// it is `Σ_{i≥2} (support of the polynomial convolved so far) ×
/// support_i`, which reduces to `support₁ · support₂` at `k = 2`.
pub const MAX_EXACT_CONVOLUTION_WORK: u64 = 100_000_000;

/// Owner decision T1 (v1.0 Task 8 follow-up, recorded on PR #77): a
/// person-time stratum's binomial support is `m1 + 1` with
/// `m1 = rint(a + b)` as `poly_strat_pt` computes it; any informative
/// stratum with `m1` above this turns its five `cmle_rr.*` leaves
/// `{"undefined": "too_large"}` (the pooled table's `m1` is the sum,
/// so it crosses with the stratum). Chosen at or above the corpus's
/// `large_counts` fixture (m1 = 1.2e6), which still computes.
/// Checked arithmetically before any allocation; the
/// `MAX_EXACT_CONVOLUTION_WORK` total-work cap applies on top.
pub const MAX_PT_SUPPORT: u64 = 2_000_000;

/// Owner decision T4 (v1.0 Task 8 follow-up, recorded on PR #78):
/// `rate_compare` computes each stratum's `cmle_rr` separately, so
/// the worst admitted call was 9 strata at `MAX_PT_SUPPORT` (~30 s
/// release). The call's total support — `Σ m1 = rint(a + b)` over
/// the informative strata — is therefore capped at the same
/// 2,000,000; above it every stratum's and the crude `cmle_rr.*`
/// leaves are `{"undefined": "too_large"}`. Checked arithmetically
/// before any allocation; `MAX_PT_SUPPORT` and
/// `MAX_EXACT_CONVOLUTION_WORK` stay in force per stratum.
pub const MAX_PT_TOTAL_SUPPORT: u64 = 2_000_000;

/// The hypergeometric support size of a 2×2's `a` cell given its
/// margins — `min(n1, m1) − max(0, n1−m0) + 1` (with `n1` the first
/// row total, `m1`/`m0` the column totals; `n1 − m0 = n1 + m1 − N`),
/// or 0 when no value of `a` is consistent with the margins. Pure
/// arithmetic: the point of the cap is to decide this *without* the
/// `O(support)` allocation.
pub fn hypergeom_support(t: &crate::stats::Table2x2) -> u64 {
    let n1 = t.a.saturating_add(t.b);
    let m0 = t.b.saturating_add(t.d);
    let m1 = t.a.saturating_add(t.c);
    let lo = n1.saturating_sub(m0);
    let hi = n1.min(m1);
    hi.checked_sub(lo).map_or(0, |d| d.saturating_add(1))
}

/// `martin`'s per-stratum informativeness: a stratum carries no
/// conditional-likelihood term when both `a·d` and `b·c` are 0.
/// Written cell-wise — `a * d` overflows u64 at counts near 2^53.
pub fn informative(t: &crate::stats::Table2x2) -> bool {
    (t.a != 0 && t.d != 0) || (t.b != 0 && t.c != 0)
}

/// `{est, lower, upper}` from a `[f64; 3]`.
pub fn stat_ci(t: [f64; 3]) -> Value {
    json!({ "est": leaf(t[0]), "lower": leaf(t[1]), "upper": leaf(t[2]) })
}

// --- canonical input access ------------------------------------------------

/// Number input `key`.
pub fn num(inputs: &Value, key: &str) -> f64 {
    inputs.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

/// Count input `key` as `u64`.
pub fn cnt(inputs: &Value, key: &str) -> u64 {
    inputs.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// A table input as `Vec<Vec<f64>>`.
pub fn table(inputs: &Value, key: &str) -> Vec<Vec<f64>> {
    inputs
        .get(key)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|r| {
                    r.as_array()
                        .map(|c| c.iter().filter_map(Value::as_f64).collect())
                        .unwrap_or_default()
                })
                .collect()
        })
        .unwrap_or_default()
}

/// A table of count inputs as `Vec<Vec<u64>>`.
pub fn count_table(inputs: &Value, key: &str) -> Vec<Vec<u64>> {
    table(inputs, key)
        .iter()
        .map(|r| r.iter().map(|x| *x as u64).collect())
        .collect()
}

/// The `strata` input — `[[[a,b],[c,d]], ...]` — as `Table2x2`s.
pub fn strata2x2(inputs: &Value) -> Vec<crate::stats::Table2x2> {
    use crate::stats::Table2x2;
    let cell = |v: Option<&Value>| v.and_then(Value::as_u64).unwrap_or(0);
    let one = |t: &Value| Table2x2 {
        a: cell(t.get(0).and_then(|r| r.get(0))),
        b: cell(t.get(0).and_then(|r| r.get(1))),
        c: cell(t.get(1).and_then(|r| r.get(0))),
        d: cell(t.get(1).and_then(|r| r.get(1))),
    };
    inputs
        .get("strata")
        .and_then(Value::as_array)
        .map(|items| {
            if !items.is_empty()
                && items
                    .iter()
                    .all(|e| e.as_array().is_some_and(|r| r.iter().all(Value::is_array)))
            {
                items.iter().map(one).collect()
            } else {
                vec![one(&Value::Array(items.clone()))]
            }
        })
        .unwrap_or_default()
}

/// The `conf_level` input in percent.
pub fn conf(inputs: &Value) -> f64 {
    num(inputs, "conf_level")
}

/// `qnorm((1 + conf/100)/2)` — the two-sided normal deviate for a
/// confidence level given in percent (`oe_z` in the R methods).
pub fn z(conf: f64) -> f64 {
    normal_quantile((1.0 + conf / 100.0) / 2.0).unwrap_or(f64::NAN)
}

// --- distribution leaves ----------------------------------------------------

/// `dbinom(x, n, p)` — the binomial point mass, through the ported
/// `dbinom_raw` (Loader's `stirlerr`/`bd0` saddle-point, as R's
/// `dbinom`): composing `ln_gamma` differences lost all accuracy once
/// `n` swamped the result (rel. err ~1.7e-6 at n = 1e9, garbage by
/// 1e15 — Review Focus 3, fix round 2).
pub fn dbinom(x: u64, n: u64, p: f64) -> f64 {
    if p <= 0.0 {
        return f64::from(x == 0);
    }
    if p >= 1.0 {
        return f64::from(x == n);
    }
    if x > n {
        return 0.0;
    }
    dbinom_raw(x as f64, n as f64, p, 1.0 - p, false)
}

/// `dpois(x, lambda)` — `dpois_raw`, R `dpois.c`'s `ebd0`-accurate
/// port, for the same reason (`ln_gamma` cancellation at large `x`).
pub fn dpois(x: u64, lambda: f64) -> f64 {
    if lambda <= 0.0 {
        return f64::from(x == 0);
    }
    if !lambda.is_finite() {
        return f64::NAN;
    }
    dpois_raw(x as f64, lambda)
}

/// `pchisq(x, 1, lower.tail = FALSE)` clamped to [0, 1].
pub fn chi_p(x: f64, df: f64) -> f64 {
    if !x.is_finite() {
        return f64::NAN;
    }
    chisq_sf(x, df).clamp(0.0, 1.0)
}

/// `pbinom(x, n, p, lower.tail)` — the calc-only O(1) binomial tail,
/// R's own `pbeta(p, x + 1, n − x, lower.tail = !lower.tail)`
/// identity (pbinom.c L44-47) on the regularised incomplete beta.
/// `special::binomial_tail`'s literal log-space sum is O(n) — it
/// stays untouched for the non-calc corpus and the native/wasm
/// parity output it is bound to; the calculators take this path so
/// valid counts below 2^53 terminate (Review Focus 1/3). The edge
/// semantics are `binomial_tail`'s: mass at 0 for `p <= 0`, at `n`
/// for `p >= 1`, full/empty range for `x >= n`.
pub fn pbinom(x: u64, n: u64, p: f64, lower: bool) -> f64 {
    if p.is_nan() {
        return f64::NAN;
    }
    if p <= 0.0 {
        return if lower { 1.0 } else { 0.0 };
    }
    if p >= 1.0 {
        return if lower {
            f64::from(x >= n)
        } else {
            f64::from(x < n)
        };
    }
    if x >= n {
        return if lower { 1.0 } else { 0.0 };
    }
    let (a, b) = ((x + 1) as f64, (n - x) as f64);
    if lower {
        reg_inc_beta_upper(p, a, b)
    } else {
        reg_inc_beta(p, a, b)
    }
}

/// `pbinom` tails in the R spellings used by the corpus.
pub struct BinomTails {
    /// `P(X < x)`.
    pub lt: f64,
    /// `P(X <= x)`.
    pub le: f64,
    /// `P(X >= x)`.
    pub ge: f64,
    /// `P(X > x)`.
    pub gt: f64,
}

/// All four binomial tails of `x` under `Binom(n, p)`.
pub fn binom_tails(x: u64, n: u64, p: f64) -> BinomTails {
    BinomTails {
        lt: if x == 0 {
            0.0
        } else {
            pbinom(x - 1, n, p, true)
        },
        le: pbinom(x, n, p, true),
        ge: if x == 0 {
            1.0
        } else {
            pbinom(x - 1, n, p, false)
        },
        gt: pbinom(x, n, p, false),
    }
}

// --- exactci binomial/Poisson machinery -------------------------------------

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

/// The Simel log-method likelihood-ratio CI (`epiR::epi.tests`):
/// `se = sqrt(1/a - 1/(a+c) + 1/b - 1/(b+d))` with `a`/`b` the row
/// counts of the positive cell in each column.
pub fn simel_lr(a: u64, c: u64, b: u64, d: u64, conf: f64) -> [f64; 3] {
    let (a, c, b, d) = (a as f64, c as f64, b as f64, d as f64);
    let lr = (a / (a + c)) / (b / (b + d));
    let se = libm::sqrt(1.0 / a - 1.0 / (a + c) + 1.0 / b - 1.0 / (b + d));
    let h = z(conf) * se;
    [lr, lr * libm::exp(-h), lr * libm::exp(h)]
}

/// `epiR::epi.kappa` (method "fleiss") — Cohen's kappa with the
/// Fleiss standard error, Fleiss Equation 18.13.
pub fn kappa(tp: u64, fp: u64, fn_: u64, tn: u64, conf: f64) -> [f64; 3] {
    let (a, b, c, d) = (tp as f64, fp as f64, fn_ as f64, tn as f64);
    let n = a + b + c + d;
    let (r1, r2) = ((a + b) / n, (c + d) / n);
    let (c1, c2) = ((a + c) / n, (b + d) / n);
    let po = (a + d) / n;
    let pe = r1 * c1 + r2 * c2;
    let k = (po - pe) / (1.0 - pe);
    let inside = pe + pe * pe - (r1 * c1 * (r1 + c1) + r2 * c2 * (r2 + c2));
    let se = libm::sqrt(inside) / ((1.0 - pe) * libm::sqrt(n));
    let h = z(conf) * se;
    [k, k - h, k + h]
}

pub(crate) use super::shared_exact::*;
