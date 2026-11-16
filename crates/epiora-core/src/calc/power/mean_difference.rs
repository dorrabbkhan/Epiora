//! `power_mean_difference` — OpenEpi `PowerMean.js`: the two-sample
//! z-formula inverted, `powerz = −zα + √n1·d/√(s1² + s2²/r)` with
//! `r = n2/n1`, emitted as `Φ(powerz)·100`
//! (`fixtures/R/methods/sample_size.R::power_means`).

use crate::calc::counts::shared::{cnt, conf, leaf, z};
use crate::calc::mean_diff;
use crate::special::normal_cdf;
use crate::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let (s1, s2) = mean_diff::sds(inputs)?;
    let n1 = cnt(inputs, "n_1") as f64;
    let n2 = cnt(inputs, "n_2") as f64;
    let r = n2 / n1;
    let d = mean_diff::difference(inputs);

    let powerz = -z(conf(inputs)) + libm::sqrt(n1) * d / libm::sqrt(s1 * s1 + s2 * s2 / r);

    Ok(json!({
        "power": leaf(normal_cdf(powerz) * 100.0),
    }))
}
