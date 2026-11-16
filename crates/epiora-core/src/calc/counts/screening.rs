//! `screening` — the diagnostic-test battery of
//! `epiR::epi.tests`/`epi.kappa` plus the OpenEpi-only measures in
//! `fixtures/R/methods/screening.R`. Wilson CIs are
//! `PropCIs::scoreci` — rounded to four decimals as the package
//! reports.

use super::shared::*;
use crate::error::EngineError;
use serde_json::{json, Value};

fn wilson_stat(x: u64, n: u64, conf: f64) -> Value {
    let [l, u] = scoreci(x, n, conf);
    json!({ "est": leaf(x as f64 / n as f64), "lower": leaf(l), "upper": leaf(u) })
}

/// `entropy_reduction` — H(prev) − H(posterior) per test sign;
/// `post` is PPV for a positive test, NPV for a negative one (the
/// OpenEpi `DiagnosticTest.js` convention).
pub(crate) fn entropy(prev: f64, ppv: f64, npv: f64) -> Value {
    let h = |p: f64| -(p * libm::log(p) + (1.0 - p) * libm::log(1.0 - p));
    json!({
        "positive": leaf(h(prev) - h(ppv)),
        "negative": leaf(h(prev) - h(npv)),
    })
}

/// `level_lr` — per-level likelihood ratio, `DiagnosticTest.js` log
/// method.
pub(crate) fn level_lr(ld: f64, ln: f64, d: f64, nd: f64, conf: f64) -> Value {
    let (p1, p2) = (ld / d, ln / nd);
    let lr = p1 / p2;
    let half = z(conf) * ((1.0 - p1) / (d * p1) + (1.0 - p2) / (nd * p2));
    stat_ci([lr, lr * libm::exp(-half), lr * libm::exp(half)])
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let t = count_table(inputs, "table");
    let conf = conf(inputs);
    let tp = t[0][0];
    let fp = t[0][1];
    let fn_: u64 = t[1..].iter().map(|r| r[0]).sum();
    let tn: u64 = t[1..].iter().map(|r| r[1]).sum();
    let (d, nd) = (tp + fn_, fp + tn);
    let (tpos, tneg, n) = (tp + fp, fn_ + tn, d + nd);
    let zz = z(conf);

    let dod = {
        let odds = (tp as f64 * tn as f64) / (fp as f64 * fn_ as f64);
        let se = libm::sqrt(1.0 / tp as f64 + 1.0 / fp as f64 + 1.0 / fn_ as f64 + 1.0 / tn as f64);
        stat_ci([odds, odds * libm::exp(-zz * se), odds * libm::exp(zz * se)])
    };
    let ppv = tp as f64 / tpos as f64;
    let npv = tn as f64 / tneg as f64;

    let mut out = json!({
        "sensitivity": wilson_stat(tp, d, conf),
        "specificity": wilson_stat(tn, nd, conf),
        "ppv": wilson_stat(tp, tpos, conf),
        "npv": wilson_stat(tn, tneg, conf),
        "accuracy": wilson_stat(tp + tn, n, conf),
        "lr_pos": stat_ci(simel_lr(tp, fn_, fp, tn, conf)),
        "lr_neg": stat_ci(simel_lr(fn_, tp, tn, fp, conf)),
        "diagnostic_odds": dod,
        "kappa": stat_ci(kappa(tp, fp, fn_, tn, conf)),
        "entropy": entropy(d as f64 / n as f64, ppv, npv),
        "bias_index": leaf((fp as f64 - fn_ as f64) / n as f64),
    });
    if t.len() > 2 {
        out["level_lr"] = Value::Array(
            t.iter()
                .map(|r| level_lr(r[0] as f64, r[1] as f64, d as f64, nd as f64, conf))
                .collect(),
        );
    }
    Ok(out)
}
