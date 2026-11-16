//! `ss` — the sample-size, power and `random_numbers` calculators.
//! The Kelsey/Fleiss chains report the real-valued `vn` intermediates
//! (`*_real` leaves) alongside the rounded values; the power leaves
//! report the `powerz` deviate the run computed; `random_numbers` is
//! a seeded method trace.

use super::super::path::Segment;
use super::super::steps::{closed, leaf, method, vals};
use super::super::Explanation;
use super::finite;
use crate::calc::counts::shared::{cnt, conf, num, z};
use crate::error::EngineError;
use serde_json::Value;

mod power;
mod random;
mod sample_size;

pub(super) use power::{power_mean_difference, power_props};
pub(super) use random::random_numbers;
pub(super) use sample_size::{ss_mean_difference, ss_proportion, ss_two_props};

fn unknown(statistic: &str) -> EngineError {
    super::super::path::unknown(statistic)
}

pub(crate) fn explain(
    id: &str,
    result: &Value,
    inputs: &Value,
    statistic: &str,
    segs: &[Segment],
) -> Result<Explanation, EngineError> {
    match id {
        "ss_proportion" => ss_proportion(result, inputs, statistic, segs),
        "ss_unmatched_cc" | "ss_cohort" => ss_two_props(id, result, inputs, statistic, segs),
        "ss_mean_difference" => ss_mean_difference(result, inputs, statistic, segs),
        "power_unmatched_cc" | "power_cohort" => power_props(id, result, inputs, statistic, segs),
        "power_mean_difference" => power_mean_difference(result, inputs, statistic, segs),
        "random_numbers" => random_numbers(result, inputs, statistic, segs),
        _ => Err(unknown(statistic)),
    }
}

/// `ss_proportion`'s level key → the confidence percent.
fn level(key: &str) -> Option<f64> {
    Some(match key {
        "80" => 80.0,
        "90" => 90.0,
        "95" => 95.0,
        "97" => 97.0,
        "99" => 99.0,
        "999" => 99.9,
        "9999" => 99.99,
        _ => return None,
    })
}
