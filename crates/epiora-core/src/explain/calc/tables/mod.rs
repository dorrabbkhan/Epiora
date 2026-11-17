//! `tables` — the table-input counts calculators (`two_by_two`,
//! `two_by_n`, `r_by_c`, `dose_response`). Every `values` entry is
//! an intermediate the run computed through the shared helpers
//! (`wald_parts`, `chi_parts`, `adjusted_parts`, `woolf`,
//! `ef_risk`/`ef_or`, `or_levels`/`mh_or`/`xmh`); exact leaves are
//! one-step `Method` traces carrying their cell inputs.

use super::super::path::Segment;
use super::super::steps::{closed, leaf, method, vals, Vals};
use super::super::Explanation;
use super::{cells, cells_conf, finite};
use crate::calc::counts::shared::{self, conf, z};
use crate::error::EngineError;
use crate::stats::Table2x2;
use serde_json::Value;

mod adjusted;
mod dose;
mod ef;
mod rxc;
mod table;

pub(super) use adjusted::{two_by_n, two_by_two};
pub(super) use dose::dose_response;
pub(super) use ef::ef_leaf;
pub(super) use rxc::r_by_c;
pub(super) use table::table_leaf;

fn unknown(statistic: &str) -> EngineError {
    super::super::path::unknown(statistic)
}

/// The `strata` input as `Table2x2`s — `shared::strata2x2` as the
/// run parses it.
fn strata(inputs: &Value) -> Vec<Table2x2> {
    shared::strata2x2(inputs)
}

/// The pooled (crude) table — the run's same fold.
fn crude(strata: &[Table2x2]) -> Table2x2 {
    strata.iter().fold(
        Table2x2 {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
        },
        |m, t| Table2x2 {
            a: m.a + t.a,
            b: m.b + t.b,
            c: m.c + t.c,
            d: m.d + t.d,
        },
    )
}

// --- shared leaf pieces -----------------------------------------------------

/// A `{value, p_1t, p_2t}` chi-square leaf of one table block.
fn chi_leaf(
    t: &Table2x2,
    statistic: &str,
    which: &str,
    side: &str,
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let [unc, yat, mh] = shared::chi_parts(t);
    let x = match which {
        "uncorrected" => unc,
        "yates" => yat,
        "mantel_haenszel" | "mantel" => mh,
        _ => return Err(unknown(statistic)),
    };
    let (fid, v) = match (which, side) {
        (_, "p_1t") => ("chi_sf_half", finite(&[("x", x), ("df", 1.0)])),
        (_, "p_2t") | (_, "p") => ("chi_sf", finite(&[("x", x), ("df", 1.0)])),
        ("uncorrected", "value") => ("chi_square_cells", oe_cells(t)),
        ("yates", "value") => ("chi_square_yates_cc", oe_cells(t)),
        ("mantel_haenszel", "value") | ("mantel", "value") => ("chi_square_mh", cells(t)),
        _ => return Err(unknown(statistic)),
    };
    Ok(closed(statistic, fid, v, est))
}

/// The observed/expected cell intermediates of `chi_parts` — the
/// cells `shared::oe_cells` returns.
fn oe_cells(t: &Table2x2) -> Vals {
    let c = shared::oe_cells(t);
    finite(&[
        ("o_a", c[0][0]),
        ("o_b", c[1][0]),
        ("o_c", c[2][0]),
        ("o_d", c[3][0]),
        ("e_a", c[0][1]),
        ("e_b", c[1][1]),
        ("e_c", c[2][1]),
        ("e_d", c[3][1]),
    ])
}

/// A `stat_ci` `{est, lower, upper}` leaf on the log scale —
/// `est·exp(∓z·se)`.
fn stat_ci_wald_exp(
    statistic: &str,
    side: &str,
    est_v: f64,
    se: f64,
    conf: f64,
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let fid = match side {
        "est" => return Err(unknown(statistic)),
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

/// `risk.{exposed,unexposed,overall}` — `prop_taylor` ×100.
fn risk_leaf(
    x: u64,
    n: u64,
    conf: f64,
    statistic: &str,
    side: &str,
    est: crate::stats::Estimate,
) -> Result<Explanation, EngineError> {
    let fid = match side {
        "est" => "proportion_pct",
        "lower" => "prop_wald_lower_pct",
        "upper" => "prop_wald_upper_pct",
        _ => return Err(unknown(statistic)),
    };
    Ok(closed(
        statistic,
        fid,
        finite(&[("x", x as f64), ("n", n as f64), ("z", z(conf))]),
        est,
    ))
}
