//! `mean_ci` — z- and t-based CIs of a mean from summary data, with
//! the √((N−n)/(N−1)) finite-population correction when
//! `population_size` is a finite population (below the 999999999
//! sentinel), per `fixtures/R/methods/means.R::mean_ci` and the
//! `mean_ci_write` t-row.

use super::super::counts::shared::{conf, leaf, num, z};
use crate::special::t_quantile;
use crate::EngineError;
use serde_json::{json, Value};

/// The population-size sentinel: at or above it the population is
/// treated as infinite (no FPC).
const POP_SENTINEL: f64 = 999_999_999.0;

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    let mean = num(inputs, "mean");
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
    let fpc = if pop > 0.0 && pop < POP_SENTINEL {
        libm::sqrt((pop - n) / (pop - 1.0))
    } else {
        1.0
    };
    let se = sd / libm::sqrt(n) * fpc;
    let z_half = z(c) * se;
    let tc = t_quantile((1.0 + c / 100.0) / 2.0, n - 1.0).unwrap_or(f64::NAN);
    let t_half = tc * se;
    Ok(json!({
        "z": { "lower": leaf(mean - z_half), "upper": leaf(mean + z_half) },
        "t": { "lower": leaf(mean - t_half), "upper": leaf(mean + t_half) },
    }))
}
