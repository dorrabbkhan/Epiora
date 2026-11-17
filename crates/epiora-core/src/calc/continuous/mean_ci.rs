//! `mean_ci` — z- and t-based CIs of a mean from summary data, with
//! the √((N−n)/(N−1)) finite-population correction when
//! `population_size` is a finite population (below the 999999999
//! sentinel), per `fixtures/R/methods/means.R::mean_ci` and the
//! `mean_ci_write` t-row.

use super::super::counts::shared::{conf, leaf, num, z};
use crate::EngineError;
use serde_json::{json, Value};

/// The population-size sentinel: at or above it the population is
/// treated as infinite (no FPC).
const POP_SENTINEL: f64 = 999_999_999.0;

/// The `sd`/`fpc`/`se` derivation of `run` — `sd` resolved from
/// `sd_or`/`sd_value`, `fpc` the finite-population correction,
/// `se = sd/√n·fpc`. Shared by the result and the explain trace
/// (v1.0 Task 10, spec §6).
pub(crate) fn se_parts(inputs: &Value) -> (f64, f64) {
    let n = num(inputs, "n");
    let sd_or = inputs
        .get("sd_or")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let sd_value = num(inputs, "sd_value");
    let sd = match sd_or {
        "se" => sd_value * libm::sqrt(n),
        "variance" => libm::sqrt(sd_value),
        _ => sd_value,
    };
    let pop = num(inputs, "population_size");
    let fpc = if pop > 0.0 && pop < POP_SENTINEL {
        libm::sqrt((pop - n) / (pop - 1.0))
    } else {
        1.0
    };
    (sd / libm::sqrt(n) * fpc, fpc)
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let mean = num(inputs, "mean");
    let n = num(inputs, "n");
    let pop = num(inputs, "population_size");
    // Inventory: `population_size` is `≥ n` or `0` = infinite — a
    // finite population below the sample size is outside the
    // contract (it would take √ of a negative FPC factor).
    if pop != 0.0 && pop < n {
        return Err(EngineError::CalcInputOutOfRange {
            input: "population_size".into(),
            reason: "below_min".into(),
        });
    }
    let c = conf(inputs);
    let (se, _fpc) = se_parts(inputs);
    let z_half = z(c) * se;
    let tc = crate::calc::counts::shared::t_crit(c, n - 1.0);
    let t_half = tc * se;
    Ok(json!({
        "z": { "lower": leaf(mean - z_half), "upper": leaf(mean + z_half) },
        "t": { "lower": leaf(mean - t_half), "upper": leaf(mean + t_half) },
    }))
}
