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

/// The closed-form Wald intermediates of one 2×2 block —
/// `[pe, pu, rr, rr_se, rd, rd_se, or, or_se]` — shared by
/// `two_by_two`'s and `two_by_n`'s `table_block` and by the explain
/// trace (v1.0 Task 10, spec §6).
pub fn wald_parts(t: &crate::stats::Table2x2) -> [f64; 8] {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0) = (a + b, c + d);
    let pe = a / n1;
    let pu = c / n0;
    let rr = pe / pu;
    let rr_se = libm::sqrt(b / (a * n1) + d / (c * n0));
    let rd = pe - pu;
    let rd_se = libm::sqrt(pe * (1.0 - pe) / n1 + pu * (1.0 - pu) / n0);
    let or_ = (a * d) / (b * c);
    let or_se = libm::sqrt(1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d);
    [pe, pu, rr, rr_se, rd, rd_se, or_, or_se]
}

/// The three chi-square statistics of one 2×2 —
/// `[uncorrected, yates, mantel_haenszel]` (`stats::chisq.test`
/// Pearson/Yates from the (O−E)²/E sums, Yates clamped at 0, plus the
/// single-stratum `mantelhaen.test` N−1 chi-square). Shared by both
/// counts-table calculators and the explain trace (spec §6).
/// The `[[o, e] × 4]` observed/expected pairs of one 2×2 — the cells
/// `chi_parts` sums over. Shared by `chi_parts` and the explain
/// trace's `chi_square_cells` intermediates (v1.0 Task 10, spec §6).
pub fn oe_cells(t: &crate::stats::Table2x2) -> [[f64; 2]; 4] {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0, m1, m0) = (a + b, c + d, a + c, b + d);
    let n = n1 + n0;
    [
        [a, n1 * m1 / n],
        [b, n1 * m0 / n],
        [c, n0 * m1 / n],
        [d, n0 * m0 / n],
    ]
}

pub fn chi_parts(t: &crate::stats::Table2x2) -> [f64; 3] {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let (n1, n0, m1, m0) = (a + b, c + d, a + c, b + d);
    let n = n1 + n0;
    let cells = oe_cells(t);
    let unc: f64 = cells.iter().map(|&[o, e]| (o - e) * (o - e) / e).sum();
    let yat: f64 = cells
        .iter()
        .map(|&[o, e]| {
            let s = (o - e).abs() - 0.5;
            s.max(0.0) * s.max(0.0) / e
        })
        .sum();
    let delta = a - n1 * m1 / n;
    let mh = delta * delta / (n1 * n0 * m1 * m0 / (n * n * (n - 1.0)));
    [unc, yat, mh]
}

/// `x/n` — a count proportion. Shared by the runs and the explain
/// traces so a trace's count-ratio input is the run's own division
/// (v1.0 Task 10, spec §6).
pub fn prop(x: u64, n: u64) -> f64 {
    x as f64 / n as f64
}

/// `qnorm((1 + conf/100)/2)` — the two-sided normal deviate for a
/// confidence level given in percent (`oe_z` in the R methods).
/// `t_quantile((1 + conf/100)/2, df)` — the two-sided t critical
/// value at `conf` percent. Shared by the runs and the explain
/// trace (v1.0 Task 10, spec §6).
pub fn t_crit(conf: f64, df: f64) -> f64 {
    crate::special::t_quantile((1.0 + conf / 100.0) / 2.0, df).unwrap_or(f64::NAN)
}

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

pub(crate) use super::shared_ci::*;
pub(crate) use super::shared_exact::*;
