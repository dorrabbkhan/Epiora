//! `stratified_two_by_two` — the `mantel_haenszel.*` and
//! `breslow_day_tarone.*` leaves (v1.0 Task 10, spec §6).

use super::super::path::Segment;
use super::super::steps::{closed, leaf};
use super::super::Explanation;
use super::finite;
use crate::dataset::Dataset;
use crate::error::EngineError;
use crate::procedures::TablesOptions;
use crate::stats::{bdt_parts, stratified_parts, StratifiedStats};

/// The kept strata of a `stratified` run (totals > 0), re-derived as
/// `stats::stratified` filters them.
fn kept_strata(
    ds: &Dataset,
    opts: &TablesOptions,
) -> Result<Vec<crate::stats::Table2x2>, EngineError> {
    Ok(super::strata_of(ds, opts)?
        .into_iter()
        .filter(|t| {
            t.a.saturating_add(t.b)
                .saturating_add(t.c)
                .saturating_add(t.d)
                > 0
        })
        .collect())
}

pub(super) fn mh_leaf(
    stats: &StratifiedStats,
    ds: &Dataset,
    opts: &TablesOptions,
    statistic: &str,
    rest: &[Segment],
) -> Result<Explanation, EngineError> {
    let tree =
        serde_json::to_value(&stats.mantel_haenszel).map_err(|e| EngineError::InvalidOptions {
            reason: format!("result does not serialize: {e}"),
        })?;
    let est = leaf(&tree, rest, statistic)?;
    let u = || super::super::path::unknown(statistic);
    let kept = kept_strata(ds, opts)?;
    let p = stratified_parts(&kept);

    let sd_or = libm::sqrt(
        p.rbg_diag / (2.0 * p.s_diag * p.s_diag)
            + p.rbg_cross / (2.0 * p.s_diag * p.s_offd)
            + p.rbg_offd / (2.0 * p.s_offd * p.s_offd),
    );
    let se_rr = libm::sqrt(p.gr_num / (p.rr_num * p.rr_den));
    let z = crate::special::normal_quantile(0.975).unwrap_or(1.959963984540054);
    let x2 = p.delta * p.delta / p.var_delta;
    let yates = if p.delta.abs() >= 0.5 { 0.5 } else { 0.0 };
    let x2c = (p.delta.abs() - yates) * (p.delta.abs() - yates) / p.var_delta;

    match rest {
        [f, which, l] if f.key == "chi_square" => {
            let (stat_v, fid) = match which.key.as_str() {
                "uncorrected" => (x2, "mh_chi_square"),
                "corrected" => (x2c, "mh_chi_square_cc"),
                _ => return Err(u()),
            };
            match l.key.as_str() {
                "statistic" => Ok(closed(
                    statistic,
                    fid,
                    finite(&[("delta", p.delta), ("var", p.var_delta), ("yates", yates)]),
                    est,
                )),
                "p_value" => Ok(closed(
                    statistic,
                    "chi_sf",
                    finite(&[("x", stat_v), ("df", 1.0)]),
                    est,
                )),
                _ => Err(u()),
            }
        }
        [f, l] if f.key == "odds_ratio" && l.key == "estimate" => Ok(closed(
            statistic,
            "mh_or",
            finite(&[("num", p.s_diag), ("den", p.s_offd)]),
            est,
        )),
        [f, l] if f.key == "risk_ratio" && l.key == "estimate" => Ok(closed(
            statistic,
            "mh_rr",
            finite(&[("num", p.rr_num), ("den", p.rr_den)]),
            est,
        )),
        [f, ci, side] if (f.key == "odds_ratio" || f.key == "risk_ratio") && ci.key == "ci_95" => {
            let fid = match side.key.as_str() {
                "lower" => "wald_exp_lower",
                "upper" => "wald_exp_upper",
                _ => return Err(u()),
            };
            let (est_v, se) = if f.key == "odds_ratio" {
                (p.s_diag / p.s_offd, sd_or)
            } else {
                (p.rr_num / p.rr_den, se_rr)
            };
            Ok(closed(
                statistic,
                fid,
                finite(&[("est", est_v), ("se", se), ("z", z)]),
                est,
            ))
        }
        _ => Err(u()),
    }
}

/// `breslow_day_tarone.*` leaves.
pub(super) fn bdt_leaf(
    stats: &StratifiedStats,
    ds: &Dataset,
    opts: &TablesOptions,
    statistic: &str,
    rest: &[Segment],
) -> Result<Explanation, EngineError> {
    let tree = serde_json::to_value(&stats.breslow_day_tarone).map_err(|e| {
        EngineError::InvalidOptions {
            reason: format!("result does not serialize: {e}"),
        }
    })?;
    let est = leaf(&tree, rest, statistic)?;
    let u = || super::super::path::unknown(statistic);
    let kept = kept_strata(ds, opts)?;
    let parts = bdt_parts(&kept, &stats.mantel_haenszel.odds_ratio.estimate);

    match rest {
        [l] if l.key == "df" => Ok(closed(
            statistic,
            "bdt_df",
            finite(&[("k", kept.len() as f64)]),
            est,
        )),
        [l] if l.key == "statistic" => {
            let (x2, sa, se_, sv) = match parts {
                Some(p) => (p.x2, p.sum_a, p.sum_e, p.sum_var),
                None => (f64::NAN, f64::NAN, f64::NAN, f64::NAN),
            };
            Ok(closed(
                statistic,
                "bdt",
                finite(&[("x2", x2), ("sum_a", sa), ("sum_e", se_), ("sum_var", sv)]),
                est,
            ))
        }
        [l] if l.key == "p_value" => {
            let x2t = match parts {
                Some(p) => p.x2 - (p.sum_a - p.sum_e) * (p.sum_a - p.sum_e) / p.sum_var,
                None => f64::NAN,
            };
            Ok(closed(
                statistic,
                "chi_sf",
                finite(&[("x", x2t), ("df", kept.len().max(1) as f64 - 1.0)]),
                est,
            ))
        }
        _ => Err(u()),
    }
}
