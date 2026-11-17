//! `calc` explain dispatch (v1.0 Task 10, spec §6): runs the
//! calculator through `calc::run` — the leaf a step reports is the
//! serialized result's leaf, bit-identical by construction — then
//! hands the path's remaining segments to the per-calculator
//! explainer, which rebuilds the run's intermediates from the same
//! shared helpers the run used.

mod counts;
mod rates;
mod ss;
mod summary;
mod tables;

use super::path::Segment;
use super::steps::Vals;
use super::Explanation;
use crate::error::EngineError;
use crate::procedures::RunContext;
use serde_json::Value;

/// The `RunContext` explain runs calculators under — the receipt is
/// discarded; the timestamp is a fixed marker.
fn ctx() -> RunContext {
    RunContext {
        timestamp: super::EXPLAIN_TS.to_string(),
        project: None,
    }
}

/// A `vals` map with non-finite intermediates dropped — `values`
/// never carries NaN or infinity; an `Undefined` leaf relays its
/// reason instead.
pub(super) fn finite(pairs: &[(&str, f64)]) -> Vals {
    pairs
        .iter()
        .filter(|(_, v)| v.is_finite())
        .map(|(k, v)| (k.to_string(), *v))
        .collect()
}

/// The cell inputs `{a, b, c, d}` of one 2×2 — a method step's
/// inputs and the `odds_ratio`/`chi_square` values alike.
pub(super) fn cells(t: &crate::stats::Table2x2) -> Vals {
    super::steps::vals(&[
        ("a", t.a as f64),
        ("b", t.b as f64),
        ("c", t.c as f64),
        ("d", t.d as f64),
    ])
}

/// `cells` plus the `conf` input — the exact-CI methods' input set.
pub(super) fn cells_conf(t: &crate::stats::Table2x2, conf: f64) -> Vals {
    let mut v = cells(t);
    v.insert("conf".into(), conf);
    v
}

/// The run half of a calculator explain (v1.0 Task 10 fix round
/// 3): the validated inputs and the serialized result of the one
/// `calc::run` every `calc.<id>.*` statistic traces from.
pub(crate) struct CalcRun {
    id: String,
    canonical: Value,
    result: Value,
}

impl CalcRun {
    pub(crate) fn id(&self) -> &str {
        &self.id
    }
}

/// `definition` + `validate` + `calc::run` once — the public
/// `explain` and the corpus tests' run-once path share this.
pub(crate) fn run(id: &str, inputs: &Value) -> Result<CalcRun, EngineError> {
    let def = crate::calc::definition(id)?;
    let canonical = crate::calc::input::validate(def, inputs)?;
    let out = crate::calc::run(id, inputs, &ctx())?;
    Ok(CalcRun {
        id: id.to_string(),
        canonical,
        result: out.result,
    })
}

impl CalcRun {
    /// The trace half — the same per-calculator dispatch the inline
    /// explainer performed.
    pub(crate) fn trace(
        &self,
        statistic: &str,
        segs: &[Segment],
    ) -> Result<Explanation, EngineError> {
        let (result, canonical) = (&self.result, &self.canonical);
        match self.id.as_str() {
            "two_by_two" => tables::two_by_two(result, canonical, statistic, segs),
            "two_by_n" => tables::two_by_n(result, canonical, statistic, segs),
            "r_by_c" => tables::r_by_c(result, canonical, statistic, segs),
            "dose_response" => tables::dose_response(result, canonical, statistic, segs),
            "binomial" => counts::binomial(result, canonical, statistic, segs),
            "poisson" => counts::poisson(result, canonical, statistic, segs),
            "proportion" => counts::proportion(result, canonical, statistic, segs),
            "screening" => counts::screening(result, canonical, statistic, segs),
            "smr" => counts::smr(result, canonical, statistic, segs),
            "matched_case_control" => {
                counts::matched_case_control(result, canonical, statistic, segs)
            }
            "rate_one" => rates::rate_one(result, canonical, statistic, segs),
            "rate_compare" => rates::rate_compare(result, canonical, statistic, segs),
            "mean_ci" => summary::mean_ci(result, canonical, statistic, segs),
            "median_ci" => summary::median_ci(result, canonical, statistic, segs),
            "t_test_summary" => summary::t_test_summary(result, canonical, statistic, segs),
            "anova_summary" => summary::anova_summary(result, canonical, statistic, segs),
            "ss_proportion"
            | "ss_unmatched_cc"
            | "ss_cohort"
            | "ss_mean_difference"
            | "power_unmatched_cc"
            | "power_cohort"
            | "power_mean_difference"
            | "random_numbers" => ss::explain(self.id.as_str(), result, canonical, statistic, segs),
            _ => Err(super::path::unknown(statistic)),
        }
    }
}
