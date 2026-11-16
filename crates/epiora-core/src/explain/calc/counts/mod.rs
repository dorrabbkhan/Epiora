//! `counts` — the scalar-input counts calculators: `binomial`,
//! `poisson`, `proportion`, `screening`, `smr` and
//! `matched_case_control`. Exact tails and exact/CMLE intervals are
//! one-step `Method` traces carrying their inputs; everything else
//! reports the intermediates the run's shared helpers computed.

use super::super::path::Segment;
use super::super::steps::{closed, leaf, method, vals, Vals};
use super::super::Explanation;
use super::finite;
use crate::calc::counts::shared::{binom_cp_ci, binom_tails, cnt, conf, count_table, num, z};
use crate::error::EngineError;
use serde_json::Value;

mod exact;
mod matched;
mod screening;
mod smr;

pub(super) use exact::{binomial, poisson, proportion};
pub(super) use matched::matched_case_control;
pub(super) use screening::screening;
pub(super) use smr::smr;

fn unknown(statistic: &str) -> EngineError {
    super::super::path::unknown(statistic)
}

/// `{x, n, z}` — the proportion-CI inputs every closed proportion
/// method shares.
fn xnz(x: u64, n: u64, conf: f64) -> Vals {
    vals(&[("x", x as f64), ("n", n as f64), ("z", z(conf))])
}

/// `screening`'s table as `(tp, fp, fn_, tn)` — the run's `count_table`
/// fold (row 0 positive, remaining rows negative).
fn screening_cells(inputs: &Value) -> (u64, u64, u64, u64) {
    let t = count_table(inputs, "table");
    let tp = t[0][0];
    let fp = t[0][1];
    let fn_: u64 = t[1..].iter().map(|r| r[0]).sum();
    let tn: u64 = t[1..].iter().map(|r| r[1]).sum();
    (tp, fp, fn_, tn)
}

/// One `wilson_stat` block (`sensitivity`, `ppv`, …): `x/n` with
/// `scoreci` bounds — the `proportion` fid set.
fn wilson_block(
    x: u64,
    n: u64,
    conf: f64,
    statistic: &str,
    side: &str,
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let fid = match side {
        "est" => "proportion",
        "lower" => "prop_scoreci_lower",
        "upper" => "prop_scoreci_upper",
        _ => return Err(unknown(statistic)),
    };
    let v = if side == "est" {
        vals(&[("x", x as f64), ("n", n as f64)])
    } else {
        xnz(x, n, conf)
    };
    Ok(closed(statistic, fid, v, est))
}

/// A `simel_lr`/`kappa`-style `{est, lower, upper}` block whose bounds
/// are `est·exp(∓z·se)` on the log scale.
fn wald_exp_block(
    est_v: f64,
    se: f64,
    conf: f64,
    statistic: &str,
    side: &str,
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let fid = match side {
        "lower" => "wald_exp_lower",
        "upper" => "wald_exp_upper",
        _ => return Err(unknown(statistic)),
    };
    Ok(closed(
        statistic,
        fid,
        finite(&[("est", est_v), ("se", se), ("z", z(conf))]),
        est,
    ))
}
