//! `matched_case_control` — 1:1 pair matching (McNemar, Taylor OR,
//! exact2x2 mid-p/central exacts on the discordant pairs) and the
//! Epiora-owned 1:n extension: one 2×2 stratum per matched set under
//! Mantel–Haenszel. The 1:n `cmle_or` is undefined in the corpus —
//! its `survival::clogit` reference errors, so the null leaves are
//! authoritative.

use super::shared::*;
use crate::error::EngineError;
use serde_json::{json, Value};

/// The 1:1 block: `matched_expected` with `controls_per_case == 1`.
/// `(b/c, √(1/b + 1/c))` — the discordant-pairs OR and its
/// log-scale SE, shared by `pair` and the explain trace (v1.0
/// Task 10, spec §6).
pub(crate) fn pair_or(db: u64, dc: u64) -> (f64, f64) {
    let (dbcf, dcc) = (db as f64, dc as f64);
    (dbcf / dcc, libm::sqrt(1.0 / dbcf + 1.0 / dcc))
}

/// McNemar's chi-square on `n = db + dc` discordant pairs;
/// `correct` applies `(|db−dc|−1)²/n` — 0 when `db == dc`, as
/// `mcnemar.test(correct=TRUE)`.
pub(crate) fn mcnemar_chi(db: u64, dc: u64, correct: bool) -> f64 {
    let (dbcf, dcc) = (db as f64, dc as f64);
    let nf = dbcf + dcc;
    if nf == 0.0 {
        return f64::NAN;
    }
    let d = if !correct {
        dbcf - dcc
    } else if db == dc {
        0.0
    } else {
        (dbcf - dcc).abs() - 1.0
    };
    d * d / nf
}

fn pair(inputs: &Value, conf: f64) -> Value {
    let t = count_table(inputs, "matched_sets");
    let db = t[0][1];
    let dc = t[1][0];
    let n = db + dc;
    let zz = z(conf);
    // `mcnemar.test(correct=TRUE)` subtracts the 1 only when the
    // table differs from its transpose (`any(x - t(x) != 0)`);
    // `db == dc` therefore gives statistic 0 (p = 1), not 1/n.
    let chi = mcnemar_chi(db, dc, false);
    let chi_cc = mcnemar_chi(db, dc, true);
    let (or, se) = pair_or(db, dc);
    let [ml, mu] = {
        let [l, u] = binom_midp_ci(db, n, conf);
        [l / (1.0 - l), u / (1.0 - u)]
    };
    let [fl, fu] = {
        let [l, u] = binom_cp_ci(db, n, conf);
        [l / (1.0 - l), u / (1.0 - u)]
    };
    let tails = binom_tails(db, n, 0.5);
    let pm = 0.5 * dbinom(db, n, 0.5);
    let fisher_1t = tails.le.min(tails.ge);
    // binom.test's two-sided p: mass at most 1+1e-7 times observed.
    // Binom(n, 0.5) is symmetric and unimodal, so the qualifying set
    // is [0, L] ∪ [n−L, n] — two equal tails. L is the largest index
    // in the lower half whose mass passes the same `1 + 1e-7` band
    // the literal pass used; binary search finds it in O(log n)
    // `dbinom` calls and `pbinom` sums both tails exactly.
    let d_obs = dbinom(db, n, 0.5);
    let threshold = d_obs * (1.0 + 1e-7);
    let fisher_2t = if dbinom(0, n, 0.5) > threshold {
        0.0
    } else {
        let (mut lo, mut hi) = (0u64, n / 2);
        while lo < hi {
            let mid = lo + (hi - lo).div_ceil(2);
            if dbinom(mid, n, 0.5) <= threshold {
                lo = mid;
            } else {
                hi = mid - 1;
            }
        }
        (2.0 * pbinom(lo, n, 0.5, true)).min(1.0)
    };
    let midp_p = (2.0 * (tails.le - pm).min(tails.ge - pm)).min(1.0);
    json!({
        "mcnemar": { "value": leaf(chi), "p": leaf(chi_p(chi, 1.0)) },
        "mcnemar_cc": { "value": leaf(chi_cc), "p": leaf(chi_p(chi_cc, 1.0)) },
        "or": stat_ci([or, or * libm::exp(-zz * se), or * libm::exp(zz * se)]),
        "cmle_or": {
            "midp": { "est": leaf(or), "lower": leaf(ml), "upper": leaf(mu) },
            "fisher": { "lower": leaf(fl), "upper": leaf(fu) },
        },
        "exact": {
            "midp_p": leaf(midp_p),
            "fisher_p_1t": leaf(fisher_1t),
            "fisher_p_2t": leaf(fisher_2t),
        },
        "discordant_pairs": leaf(n as f64),
    })
}

/// `matched_n.cmle_or` — the `survival::clogit` conditional MLE: one
/// stratum per matched set, Wald CI on the coefficient. A set with
/// case exposure `e` and `k` of `M` controls exposed has `t = e + k`
/// exposed members and contributes `e − p` to the score and `p(1−p)`
/// to the observed information, `p = t·ψ/(t·ψ + M+1−t)`, `ψ = e^β` —
/// the set's conditional probability that the case is exposed. The
/// shapes carry multiplicities, so the fit is O(M) in memory (Review
/// Focus 1/3). Boundary inputs mirror `matched_cmle_direct`: sets
/// pushing only up (`e = 1, k < M`), only down (`e = 0, k > 0`) or
/// none at all put the MLE on the boundary or leave it unidentified —
/// `unbounded`, the reason `stats::exact` uses for an MLE at the
/// support edge (clogit itself reports NA coef/se 0 on a flat
/// likelihood and diverges with "beta may be infinite" on one-way
/// discordance).
fn matched_n_cmle(t: &[Vec<u64>], m: u64, conf: f64) -> [f64; 3] {
    // `p(β)` for one shape without overflow: ψ = e^β over/underflows
    // at |β| ≈ 709/745, so the two one-sided forms avoid 0/0 and ∞/∞.
    let p_of = |e: u64, k: u64, beta: f64| -> f64 {
        let t = (e + k) as f64;
        let c = (m + 1 - e - k) as f64;
        if beta <= 0.0 {
            let w = t * libm::exp(beta);
            w / (w + c)
        } else {
            t / (t + c * libm::exp(-beta))
        }
    };
    // `matched_sets` rows: row 0 = exposed case (e = 1), row 1 =
    // unexposed (e = 0) — the same orientation `matched_n`'s MH sums
    // use (`case_exp == 0` → `a = 1` there).
    let mut up = 0u64;
    let mut down = 0u64;
    for &mult in t[0].iter().take(m as usize) {
        up += mult;
    }
    for &mult in t[1].iter().skip(1) {
        down += mult;
    }
    if up == 0 || down == 0 {
        return [f64::NAN; 3];
    }
    // Score and observed information over the informative shapes only
    // (t = 0 and c = 0 sets contribute e − p ≡ 0 and p(1−p) ≡ 0).
    let score_info = |beta: f64| -> (f64, f64) {
        let (mut s, mut info) = (0.0, 0.0);
        for (row, cells) in t.iter().enumerate().take(2) {
            let e = 1u64 - row as u64;
            for (k, &mult) in cells.iter().enumerate() {
                if mult == 0 || (e == 0 && k == 0) || (e == 1 && k == m as usize) {
                    continue;
                }
                let p = p_of(e, k as u64, beta);
                s += mult as f64 * (e as f64 - p);
                info += mult as f64 * p * (1.0 - p);
            }
        }
        (s, info)
    };
    // The score is non-increasing in β (∂/∂β = −info ≤ 0), so a sign
    // change bounds the unique root; β = 0 is bracketed to start.
    let (mut lo, mut hi) = (-1.0, 1.0);
    while score_info(lo).0 <= 0.0 {
        lo *= 2.0;
    }
    while score_info(hi).0 >= 0.0 {
        hi *= 2.0;
    }
    // Safeguarded Newton: β + score/info (Newton on the score root,
    // whose slope is −info), bisection when the step leaves the
    // bracket — the same observed information clogit's var comes from.
    let mut beta = 0.0;
    for _ in 0..200 {
        let (s, info) = score_info(beta);
        if s > 0.0 {
            lo = beta;
        } else {
            hi = beta;
        }
        let newton = beta + s / info;
        beta = if newton.is_finite() && newton > lo && newton < hi {
            newton
        } else {
            0.5 * (lo + hi)
        };
        if hi - lo <= 1e-13 * beta.abs().max(1.0) {
            break;
        }
    }
    let (_, info) = score_info(beta);
    let se = 1.0 / libm::sqrt(info);
    let h = z(conf) * se;
    [libm::exp(beta), libm::exp(beta - h), libm::exp(beta + h)]
}

/// The multiplicity-weighted MH sums of `matched_n` — the explain
/// trace's named values (v1.0 Task 10, spec §6). The run path calls
/// `matched_n_parts` and the explain path reuses it.
pub(crate) struct MatchedNParts {
    /// `sum(mult·(a − N1·M1/n))` over the distinct set shapes.
    pub delta: f64,
    /// `sum(mult·N1·N0·M1·M0 / (n²·(n−1)))`.
    pub var: f64,
    /// `sum(mult·a·d/n)`.
    pub sd_: f64,
    /// `sum(mult·b·c/n)`.
    pub so_: f64,
    /// `sum(mult·(a+d)·a·d/n²)`.
    pub rg_a: f64,
    /// `sum(mult·((a+d)·b·c + (b+c)·a·d)/n²)`.
    pub rg_b: f64,
    /// `sum(mult·(b+c)·b·c/n²)`.
    pub rg_c: f64,
}

impl MatchedNParts {
    /// `delta²/var` — the uncorrected MH chi-square.
    pub(crate) fn chi(&self) -> f64 {
        self.delta * self.delta / self.var
    }

    /// `(|delta|−0.5)²/var` clamped at 0 — the corrected chi-square.
    pub(crate) fn chi_cc(&self) -> f64 {
        let s = (self.delta.abs() - 0.5).max(0.0);
        s * s / self.var
    }

    /// `sd_/so_` — the MH odds ratio.
    pub(crate) fn or(&self) -> f64 {
        self.sd_ / self.so_
    }

    /// The Robins–Breslow–Greenland SD of `log or`.
    pub(crate) fn sd_or(&self) -> f64 {
        libm::sqrt(
            self.rg_a / (2.0 * self.sd_ * self.sd_)
                + self.rg_b / (2.0 * self.sd_ * self.so_)
                + self.rg_c / (2.0 * self.so_ * self.so_),
        )
    }
}

/// The `matched_n` sums — shared by the result and the explain
/// trace.
pub(crate) fn matched_n_parts(t: &[Vec<u64>], m: u64) -> MatchedNParts {
    let (mut delta, mut var, mut sd_, mut so_): (f64, f64, f64, f64) = (0.0, 0.0, 0.0, 0.0);
    let (mut rg_a, mut rg_b, mut rg_c) = (0.0, 0.0, 0.0);
    for (case_exp, row) in t.iter().enumerate().take(2) {
        for (k, &mult) in row.iter().enumerate() {
            if mult == 0 {
                continue;
            }
            let (a, b, c, d) = if case_exp == 0 {
                (1.0, k as f64, 0.0, (m - k as u64) as f64)
            } else {
                (0.0, k as f64, 1.0, (m - k as u64) as f64)
            };
            let mult = mult as f64;
            let (n1, n0, m1, m0, n) = (a + b, c + d, a + c, b + d, a + b + c + d);
            delta += mult * (a - n1 * m1 / n);
            var += mult * (n1 * n0 * m1 * m0 / (n * n * (n - 1.0)));
            sd_ += mult * (a * d / n);
            so_ += mult * (b * c / n);
            rg_a += mult * ((a + d) * a * d / (n * n));
            rg_b += mult * (((a + d) * b * c + (b + c) * a * d) / (n * n));
            rg_c += mult * ((b + c) * b * c / (n * n));
        }
    }
    MatchedNParts {
        delta,
        var,
        sd_,
        so_,
        rg_a,
        rg_b,
        rg_c,
    }
}

/// The 1:n block: each matched set is one 2×2 stratum
/// `[[case_exp, k], [case_unexp, M-k]]` under `mantelhaen.test`. The
/// sums are linear over identical strata, so the 2·(M+1) ≤ 10
/// distinct (case_exp, k) shapes carry each accumulated term times
/// its multiplicity — O(M) work and memory for any valid counts,
/// where enumerating one stratum per set was O(Σ cells) (Review
/// Focus 1/3).
fn matched_n(inputs: &Value, conf: f64) -> Value {
    let m = cnt(inputs, "controls_per_case");
    let t = count_table(inputs, "matched_sets");
    let zz = z(conf);
    let p = matched_n_parts(&t, m);
    let chi = p.chi();
    let chi_cc = p.chi_cc();
    let or = p.or();
    let sd = p.sd_or();
    let [ce, cl, cu] = matched_n_cmle(&t, m, conf);
    // A boundary MLE (NaN from the guard) is `unbounded`, not the
    // generic non-finite fallback.
    let cmle = |v: f64| {
        if v.is_nan() {
            undefined("unbounded")
        } else {
            leaf(v)
        }
    };
    json!({
        "matched_n": {
            "mh_or": stat_ci([or, or * libm::exp(-zz * sd), or * libm::exp(zz * sd)]),
            "mh_chisq": { "value": leaf(chi), "p": leaf(chi_p(chi, 1.0)) },
            "mh_chisq_cc": { "value": leaf(chi_cc), "p": leaf(chi_p(chi_cc, 1.0)) },
            "cmle_or": {
                "est": cmle(ce),
                "lower": cmle(cl),
                "upper": cmle(cu),
            },
        },
    })
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let m = cnt(inputs, "controls_per_case");
    let conf = conf(inputs);
    let t = count_table(inputs, "matched_sets");
    if t.len() != 2 || t.iter().any(|r| r.len() != (m + 1) as usize) {
        return Err(EngineError::CalcInputOutOfRange {
            input: "matched_sets".into(),
            reason: "bad_table_shape".into(),
        });
    }
    let mut out = if m == 1 {
        pair(inputs, conf)
    } else {
        json!({})
    };
    if m >= 2 {
        let Value::Object(map) = matched_n(inputs, conf) else {
            return Err(EngineError::CalcUnknown {
                id: "matched".into(),
            });
        };
        if let Some(o) = out.as_object_mut() {
            o.extend(map);
        }
    }
    Ok(out)
}
