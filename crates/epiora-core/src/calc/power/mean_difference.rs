//! `power_mean_difference` — OpenEpi `PowerMean.js`: the two-sample
//! z-formula inverted, `powerz = −zα + √n1·d/√(s1² + s2²/r)` with
//! `r = n2/n1`, emitted as `Φ(powerz)·100`
//! (`fixtures/R/methods/sample_size.R::power_means`).

use super::parts::{group_ratio, powerz_mean};
use crate::calc::counts::shared::{cnt, conf, leaf, z};
use crate::calc::mean_diff;
use crate::special::normal_cdf;
use crate::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let (s1, s2) = mean_diff::sds(inputs)?;
    let n1 = cnt(inputs, "n_1") as f64;
    let n2 = cnt(inputs, "n_2") as f64;
    let r = group_ratio(n1, n2);
    let d = mean_diff::difference(inputs);

    let powerz = powerz_mean(n1, r, d, s1, s2, z(conf(inputs)));

    Ok(json!({
        "power": leaf(normal_cdf(powerz) * 100.0),
    }))
}
