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

/// The group-1 variance-based size — `(s1² + s2²/ratio)·(zα+zβ)²/d²`
/// — shared by the run and the explain trace (v1.0 Task 10, spec §6).
pub(crate) fn vn(s1: f64, s2: f64, ratio: f64, za: f64, zb: f64, diff: f64) -> f64 {
    (s1 * s1 + s2 * s2 / ratio) * (za + zb) * (za + zb) / (diff * diff)
}

/// `n2_real = n1·ratio` — the group-2 size from the already-rounded
/// `n1`, shared by the run and the explain trace.
pub(crate) fn n2_real(n1: f64, ratio: f64) -> f64 {
    n1 * ratio
}

/// `(n1, n2_real, n2)` — `ceil(vn)`, then `n2_real = n1·ratio`, then
/// `ceil(n2_real)`; `n2` comes from the already-rounded `n1`, which
/// is why `n2_real` in the fixtures is not `vn·ratio` — shared by
/// the run and the explain trace (v1.0 Task 10, spec §6).
pub(crate) fn n1_n2(vn: f64, ratio: f64) -> (f64, f64, f64) {
    let n1 = vn.ceil();
    let vn2 = n2_real(n1, ratio);
    (n1, vn2, vn2.ceil())
}

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
    let vn = vn(s1, s2, ratio, za, zb, diff);
    let (n1, vn2, n2) = n1_n2(vn, ratio);

    Ok(json!({
        "n1": leaf(n1),
        "n1_real": leaf(vn),
        "n2": leaf(n2),
        "n2_real": leaf(vn2),
        "total": leaf(n1 + n2),
        "total_real": leaf(vn + vn2),
    }))
}
