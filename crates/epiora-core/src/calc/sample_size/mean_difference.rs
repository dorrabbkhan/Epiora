//! `ss_mean_difference` — the OpenEpi `SSMean.js` two-sample
//! z-formula: `vn = (s1² + s2²/ratio)·(zα + zβ)²/d²`, then
//! `n1 = ceil(vn)`, `n2 = ceil(n1·ratio)` — `n2` is computed from
//! the already-rounded `n1`, which is why `n2_real` in the fixtures
//! is `n1·ratio`, not `vn·ratio` (`fixtures/R/methods/sample_size.R`).
//! A zero difference divides by it — the required size is infinite,
//! so every leaf turns `infinite_sample_size` (Review Focus 3).

use crate::calc::counts::shared::{conf, leaf, num, undefined, z};
use crate::calc::mean_diff;
use crate::special::normal_quantile;
use crate::EngineError;
use serde_json::{json, Value};

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let (s1, s2) = mean_diff::sds(inputs)?;
    let ratio = num(inputs, "ratio");
    let diff = mean_diff::difference(inputs);

    if diff == 0.0 {
        let inf = || undefined("infinite_sample_size");
        return Ok(json!({
            "n1": inf(), "n1_real": inf(),
            "n2": inf(), "n2_real": inf(),
            "total": inf(), "total_real": inf(),
        }));
    }

    let za = z(conf(inputs));
    // zb is one-sided: qnorm(power/100), not the two-sided z().
    let zb = normal_quantile(num(inputs, "power") / 100.0).unwrap_or(f64::NAN);
    let vn = (s1 * s1 + s2 * s2 / ratio) * (za + zb) * (za + zb) / (diff * diff);
    let n1 = vn.ceil();
    let vn2 = n1 * ratio;
    let n2 = vn2.ceil();

    Ok(json!({
        "n1": leaf(n1),
        "n1_real": leaf(vn),
        "n2": leaf(n2),
        "n2_real": leaf(vn2),
        "total": leaf(n1 + n2),
        "total_real": leaf(vn + vn2),
    }))
}
