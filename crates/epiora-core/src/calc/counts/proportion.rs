//! `proportion` — one proportion `x/n` with eight confidence
//! intervals and an optional one-sample z-test
//! (`fixtures/R/methods/proportion.R`, `exactci::binom.exact`,
//! `PropCIs::{wald2ci,addz2ci,scoreci}`). `wilson_fpc` is emitted
//! only when the FPC condition `2 < N < 1e6` holds, as the corpus
//! records.

use super::shared::{
    agresti_coull_ci, binom_cp_ci, binom_midp_ci, cnt, conf, leaf, num, score_cc_ci, scoreci,
    stat_ci, table, wald_ci, wilson_fpc_ci, z,
};
use crate::error::EngineError;
use crate::special::normal_cdf;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let x = cnt(inputs, "numerator");
    let n = cnt(inputs, "denominator");
    let conf = conf(inputs);
    let pop = cnt(inputs, "population_size");
    if x > n {
        return Err(EngineError::CalcInputOutOfRange {
            input: "numerator".into(),
            reason: "above_max".into(),
        });
    }
    if pop != 0 && pop < n {
        return Err(EngineError::CalcInputOutOfRange {
            input: "population_size".into(),
            reason: "below_min".into(),
        });
    }
    let _ = table;
    let _ = num;

    let p = x as f64 / n as f64;
    let [ml, mu] = binom_midp_ci(x, n, conf);
    let [cl, cu] = binom_cp_ci(x, n, conf);
    let [wl, wu] = wald_ci(x, n, conf);
    let [al, au] = agresti_coull_ci(x, n, conf);
    let [sl, su] = scoreci(x, n, conf);
    let [scl, scu] = score_cc_ci(x, n, conf);

    let mut ci = serde_json::Map::new();
    ci.insert(
        "midp".into(),
        json!({ "lower": leaf(ml), "upper": leaf(mu) }),
    );
    ci.insert(
        "clopper_pearson".into(),
        json!({ "lower": leaf(cl), "upper": leaf(cu) }),
    );
    ci.insert(
        "wald".into(),
        json!({ "lower": leaf(wl), "upper": leaf(wu) }),
    );
    ci.insert(
        "agresti_coull".into(),
        json!({ "lower": leaf(al), "upper": leaf(au) }),
    );
    ci.insert(
        "wilson".into(),
        json!({ "lower": leaf(sl), "upper": leaf(su) }),
    );
    ci.insert(
        "score_cc".into(),
        json!({ "lower": leaf(scl), "upper": leaf(scu) }),
    );
    if pop > 2 && pop < 1_000_000 {
        let [fl, fu] = wilson_fpc_ci(x, n, conf, pop as f64);
        ci.insert(
            "wilson_fpc".into(),
            json!({ "lower": leaf(fl), "upper": leaf(fu) }),
        );
    }

    let mut out = json!({ "estimate": leaf(p), "ci": Value::Object(ci) });
    if let Some(cw) = inputs.get("compare_with").and_then(Value::as_f64) {
        let p0 = cw / 100.0;
        let zv = (p - p0) / libm::sqrt(p0 * (1.0 - p0) / n as f64);
        out["ztest"] = json!({ "z": leaf(zv), "p": leaf(2.0 * normal_cdf(-zv.abs())) });
    }
    let _ = stat_ci;
    let _ = z;
    Ok(out)
}
