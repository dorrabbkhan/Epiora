//! `anova_summary` — one-way ANOVA and Bartlett's test from per-group
//! `[n, mean, sd]` rows, sharing `stats::means_summary`'s
//! `anova_from_summary`/`bartlett_from_summary` with MEANS, plus the
//! per-group individual and pooled CIs (`qt(0.975, nᵢ−1)` and
//! `qt(0.975, df_within)·√(ms_w/nᵢ)`) of `anova_summary_write`.

use super::super::counts::shared::{leaf, table, undefined};
use super::est_leaf;
use crate::calc::input::MAX_COUNT;
use crate::special::t_quantile;
use crate::stats::{anova_from_summary, bartlett_from_summary, Estimate, GroupSummary};
use crate::EngineError;
use serde_json::{json, Value};

/// Inventory `## anova_summary` row contract, which the uniform
/// `CellKind::Real` table cannot express (the
/// `rate_compare::check_strata` pattern): each row is one group's
/// `[n, mean, sd]` — `n` an integer `≥ 2` and `< 2^53`, `mean` finite,
/// `sd` finite `> 0`. Typed `calc_input_out_of_range` on `table`;
/// without it `n = 0` underflows `g.n − 1` in
/// `stats::means_summary` and `n` near `u64::MAX` overflows `n_tot`.
fn check_rows(inputs: &Value) -> Result<(), EngineError> {
    let err = |reason: &str| EngineError::CalcInputOutOfRange {
        input: "table".into(),
        reason: reason.into(),
    };
    let rows = inputs
        .get("table")
        .and_then(Value::as_array)
        .ok_or_else(|| err("bad_table_shape"))?;
    for r in rows {
        let cells = r
            .as_array()
            .filter(|c| c.len() == 3)
            .ok_or_else(|| err("bad_table_shape"))?;
        let at = |i: usize| cells[i].as_f64().unwrap_or(f64::NAN);
        let (n, mean, sd) = (at(0), at(1), at(2));
        if !n.is_finite() || n.fract() != 0.0 {
            return Err(err("not_an_integer"));
        }
        if n < 2.0 {
            return Err(err("below_min"));
        }
        if n >= MAX_COUNT as f64 {
            return Err(err("above_max"));
        }
        if !mean.is_finite() || !sd.is_finite() {
            return Err(err("not_finite"));
        }
        if sd <= 0.0 {
            return Err(err("below_min"));
        }
    }
    Ok(())
}

pub(super) fn run(inputs: &Value) -> Result<Value, EngineError> {
    check_rows(inputs)?;
    let rows = table(inputs, "table");
    let groups: Vec<GroupSummary> = rows
        .iter()
        .map(|r| GroupSummary {
            n: *r.first().unwrap_or(&0.0) as u64,
            mean: *r.get(1).unwrap_or(&0.0),
            variance: Some({
                let s = *r.get(2).unwrap_or(&0.0);
                s * s
            }),
        })
        .collect();
    let an = anova_from_summary(&groups);
    let ms_w = match an.ms_within {
        Estimate::Value(v) => v,
        Estimate::Undefined { .. } => f64::NAN,
    };
    let df_w = match an.df_within {
        Estimate::Value(v) => v,
        Estimate::Undefined { .. } => f64::NAN,
    };
    let tc_p = t_quantile(0.975, df_w).unwrap_or(f64::NAN);
    let group_ci: Vec<Value> = groups
        .iter()
        .map(|g| {
            let sd = libm::sqrt(g.variance.unwrap_or(f64::NAN));
            let tc = t_quantile(0.975, (g.n - 1) as f64).unwrap_or(f64::NAN);
            let h = tc * sd / libm::sqrt(g.n as f64);
            json!({ "lower": leaf(g.mean - h), "upper": leaf(g.mean + h) })
        })
        .collect();
    let group_ci_pooled: Vec<Value> = groups
        .iter()
        .map(|g| {
            let h = tc_p * libm::sqrt(ms_w / g.n as f64);
            json!({ "lower": leaf(g.mean - h), "upper": leaf(g.mean + h) })
        })
        .collect();
    let bartlett = match bartlett_from_summary(&groups) {
        Some(b) => json!({
            "chisq": est_leaf(&b.statistic),
            "df": est_leaf(&b.df),
            "p": est_leaf(&b.p_value),
        }),
        // R's bartlett.test errors below two observations per group.
        None => {
            let u = undefined("too_few");
            json!({ "chisq": u, "df": u, "p": u })
        }
    };
    Ok(json!({
        "anova": {
            "ss_between": est_leaf(&an.ss_between),
            "df_between": est_leaf(&an.df_between),
            "ms_between": est_leaf(&an.ms_between),
            "ss_within": est_leaf(&an.ss_within),
            "df_within": est_leaf(&an.df_within),
            "ms_within": est_leaf(&an.ms_within),
            "f": est_leaf(&an.f),
            "p": est_leaf(&an.p_value),
            "ss_total": est_leaf(&an.ss_total),
            "df_total": est_leaf(&an.df_total),
        },
        "bartlett": bartlett,
        "group_ci": group_ci,
        "group_ci_pooled": group_ci_pooled,
    }))
}
