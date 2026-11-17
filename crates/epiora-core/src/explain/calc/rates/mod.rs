//! `rates` — the person-time calculators `rate_one` and
//! `rate_compare`. The closed leaves report the shared
//! `stats::rates` intermediates; the CMLE rate ratios are one-step
//! `Method` traces.

use super::super::path::Segment;
use super::super::steps::{closed, leaf, method, vals, Vals};
use super::super::Explanation;
use super::finite;
use crate::calc::counts::shared::{cnt, conf, z};
use crate::error::EngineError;
use crate::stats::{self as rates, Estimate};
use serde_json::Value;

mod adjusted;
mod compare;
mod leaf_pt;
mod one;

pub(super) use compare::rate_compare;
pub(super) use one::rate_one;

fn unknown(statistic: &str) -> EngineError {
    super::super::path::unknown(statistic)
}
