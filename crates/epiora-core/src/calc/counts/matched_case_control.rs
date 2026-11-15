//! `matched_case_control` — 1:1 pair matching (McNemar, Taylor OR,
//! exact2x2 mid-p/central exacts on the discordant pairs) and the
//! Epiora-owned 1:n extension: one 2×2 stratum per matched set under
//! Mantel–Haenszel. The 1:n `cmle_or` is undefined in the corpus —
//! its `survival::clogit` reference errors, so the null leaves are
//! authoritative.

use super::shared::*;
use crate::error::EngineError;
use crate::stats::Table2x2;
use serde_json::{json, Value};

/// The 1:1 block: `matched_expected` with `controls_per_case == 1`.
fn pair(inputs: &Value, conf: f64) -> Value {
    let t = count_table(inputs, "matched_sets");
    let db = t[0][1];
    let dc = t[1][0];
    let n = db + dc;
    let zz = z(conf);
    let chi = if n == 0 {
        f64::NAN
    } else {
        let d = db as f64 - dc as f64;
        d * d / n as f64
    };
    let chi_cc = if n == 0 {
        f64::NAN
    } else {
        let d = (db as f64 - dc as f64).abs() - 1.0;
        d * d / n as f64
    };
    let or = db as f64 / dc as f64;
    let se = libm::sqrt(1.0 / db as f64 + 1.0 / dc as f64);
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
    let d_obs = dbinom(db, n, 0.5);
    let fisher_2t = (0..=n)
        .map(|i| dbinom(i, n, 0.5))
        .filter(|&p| p <= d_obs * (1.0 + 1e-7))
        .sum::<f64>()
        .min(1.0);
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

/// The 1:n block: each matched set is one 2×2 stratum
/// `[[case_exp, k], [case_unexp, M-k]]` under `mantelhaen.test`.
fn matched_n(inputs: &Value, conf: f64) -> Value {
    let m = cnt(inputs, "controls_per_case");
    let t = count_table(inputs, "matched_sets");
    let zz = z(conf);
    let mut strata: Vec<Table2x2> = Vec::new();
    for (case_exp, row) in t.iter().enumerate().take(2) {
        for (k, &cnt_) in row.iter().enumerate() {
            for _ in 0..cnt_ {
                strata.push(if case_exp == 0 {
                    Table2x2 {
                        a: 1,
                        b: k as u64,
                        c: 0,
                        d: m - k as u64,
                    }
                } else {
                    Table2x2 {
                        a: 0,
                        b: k as u64,
                        c: 1,
                        d: m - k as u64,
                    }
                });
            }
        }
    }
    let (mut delta, mut var, mut sd_, mut so_): (f64, f64, f64, f64) = (0.0, 0.0, 0.0, 0.0);
    let (mut rg_a, mut rg_b, mut rg_c) = (0.0, 0.0, 0.0);
    for t in &strata {
        let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
        let (n1, n0, m1, m0, n) = (a + b, c + d, a + c, b + d, a + b + c + d);
        delta += a - n1 * m1 / n;
        var += n1 * n0 * m1 * m0 / (n * n * (n - 1.0));
        sd_ += a * d / n;
        so_ += b * c / n;
        rg_a += (a + d) * a * d / (n * n);
        rg_b += ((a + d) * b * c + (b + c) * a * d) / (n * n);
        rg_c += (b + c) * b * c / (n * n);
    }
    let chi = delta * delta / var;
    let chi_cc = {
        let s = (delta.abs() - 0.5).max(0.0);
        s * s / var
    };
    let or = sd_ / so_;
    let sd =
        libm::sqrt(rg_a / (2.0 * sd_ * sd_) + rg_b / (2.0 * sd_ * so_) + rg_c / (2.0 * so_ * so_));
    json!({
        "matched_n": {
            "mh_or": stat_ci([or, or * libm::exp(-zz * sd), or * libm::exp(zz * sd)]),
            "mh_chisq": { "value": leaf(chi), "p": leaf(chi_p(chi, 1.0)) },
            "mh_chisq_cc": { "value": leaf(chi_cc), "p": leaf(chi_p(chi_cc, 1.0)) },
            "cmle_or": {
                "est": undefined("no_convergence"),
                "lower": undefined("no_convergence"),
                "upper": undefined("no_convergence"),
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
