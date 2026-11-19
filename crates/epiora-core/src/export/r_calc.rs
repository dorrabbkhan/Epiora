//! `r_calc` — the `<writer>(id, …)` call for every `calc.<dir>`
//! (v1.0 Task 17, spec §8.1): one `match` arm per calculator, inputs
//! embedded as R literals, plus the corpus seed each arm's writer
//! needs. `random_numbers` is the one run the engine refuses: its
//! draws come from a seeded ChaCha stream no R code can replay.

use super::r_emit::{lit_json, lit_matrix, lit_str, lit_tables};
use super::r_script::{invalid, not_exportable, opt, req};
use crate::EngineError;
use serde_json::Value;

// ---------------------------------------------------------------------
// Calculators (spec §8.1): `export_r_calculator` and the corpus driver.

/// The `<writer>(id, …)` call for one `calc.<dir>` run, and the writer
/// seed for the snippet closure.
pub(crate) fn calc_call(
    call_id: &str,
    dir: &str,
    inputs: &Value,
) -> Result<(&'static str, String), EngineError> {
    let id = lit_str(call_id);
    let j = lit_json;
    let num = |k: &str| -> Result<String, EngineError> { Ok(lit_json(req(inputs, k)?)) };
    let conf = |call: &mut String| {
        if let Some(c) = opt(inputs, "conf_level") {
            call.push_str(&format!(", conf = {}", j(c)));
        }
    };
    let (seed, call) = match dir {
        "proportion" => {
            let mut c = format!(
                "proportion_write({id}, {}, {}",
                num("numerator")?,
                num("denominator")?
            );
            conf(&mut c);
            if let Some(v) = opt(inputs, "population_size") {
                c.push_str(&format!(", pop = {}", j(v)));
            }
            if let Some(v) = opt(inputs, "compare_with") {
                c.push_str(&format!(", compare_with = {}", j(v)));
            }
            if let Some(v) = opt(inputs, "multiplier") {
                c.push_str(&format!(", multiplier = {}", j(v)));
            }
            c.push(')');
            ("proportion_write", c)
        }
        "two_by_two" => {
            let mut c = format!(
                "two_by_two_write({id}, {}",
                lit_tables(req(inputs, "strata")?)
            );
            conf(&mut c);
            if let Some(v) = opt(inputs, "study_type") {
                c.push_str(&format!(", study_type = {}", j(v)));
            }
            c.push(')');
            ("two_by_two_write", c)
        }
        "two_by_n" => {
            let mut c = format!(
                "two_by_n_write({id}, {}",
                lit_tables(req(inputs, "strata")?)
            );
            conf(&mut c);
            c.push(')');
            ("two_by_n_write", c)
        }
        "r_by_c" => (
            "r_by_c_write",
            format!("r_by_c_write({id}, {})", lit_matrix(req(inputs, "table")?)),
        ),
        "dose_response" => {
            let mut c = format!(
                "dose_response_write({id}, {}",
                lit_tables(req(inputs, "table")?)
            );
            conf(&mut c);
            c.push(')');
            ("dose_response_write", c)
        }
        "matched_case_control" => {
            let mut c = format!(
                "matched_write({id}, {}",
                lit_matrix(req(inputs, "matched_sets")?)
            );
            if let Some(v) = opt(inputs, "controls_per_case") {
                c.push_str(&format!(", controls_per_case = {}", j(v)));
            }
            conf(&mut c);
            c.push(')');
            ("matched_write", c)
        }
        "screening" => {
            let mut c = format!(
                "screening_write({id}, {}",
                lit_matrix(req(inputs, "table")?)
            );
            conf(&mut c);
            c.push(')');
            ("screening_write", c)
        }
        "smr" => {
            let mut c = format!("smr_write({id}, {}, {}", num("observed")?, num("expected")?);
            conf(&mut c);
            c.push(')');
            ("smr_write", c)
        }
        "binomial" => (
            "binomial_write",
            format!(
                "binomial_write({id}, {}, {}, {})",
                num("numerator")?,
                num("denominator")?,
                num("expected_percent")?
            ),
        ),
        "poisson" => (
            "poisson_write",
            format!(
                "poisson_write({id}, {}, {})",
                num("observed")?,
                num("expected")?
            ),
        ),
        "rate_one" => {
            let mut c = format!(
                "rate_one_write({id}, {}, {}",
                num("cases")?,
                num("person_time")?
            );
            if let Some(v) = opt(inputs, "per_unit") {
                c.push_str(&format!(", per_unit = {}", j(v)));
            }
            conf(&mut c);
            c.push(')');
            ("rate_one_write", c)
        }
        "rate_compare" => {
            let mut c = format!(
                "rate_compare_write({id}, {}",
                lit_tables(req(inputs, "strata")?)
            );
            if let Some(v) = opt(inputs, "per_unit") {
                c.push_str(&format!(", per_unit = {}", j(v)));
            }
            conf(&mut c);
            c.push(')');
            ("rate_compare_write", c)
        }
        "mean_ci" => {
            let mut c = format!(
                "mean_ci_write({id}, {}, {}, {}, {}",
                num("mean")?,
                j(req(inputs, "sd_or")?),
                num("sd_value")?,
                num("n")?
            );
            if let Some(v) = opt(inputs, "population_size") {
                c.push_str(&format!(", population_size = {}", j(v)));
            }
            conf(&mut c);
            c.push(')');
            ("mean_ci_write", c)
        }
        "median_ci" => {
            let mut c = format!(
                "median_ci_write({id}, {}, {}",
                num("n")?,
                num("percentile")?
            );
            conf(&mut c);
            c.push(')');
            ("median_ci_write", c)
        }
        "t_test_summary" => {
            let mut c = format!(
                "t_test_summary_write({id}, {}, {}, {}, {}, {}, {}",
                num("n_1")?,
                num("mean_1")?,
                num("sd_1")?,
                num("n_2")?,
                num("mean_2")?,
                num("sd_2")?
            );
            conf(&mut c);
            c.push(')');
            ("t_test_summary_write", c)
        }
        "anova_summary" => (
            "anova_summary_write",
            format!(
                "anova_summary_write({id}, {})",
                lit_matrix(req(inputs, "table")?)
            ),
        ),
        "ss_proportion" => (
            "ss_proportion_write",
            format!(
                "ss_proportion_write({id}, {}, {}, {}, {}{})",
                num("population_size")?,
                num("expected_freq")?,
                num("margin_error")?,
                num("design_effect")?,
                opt(inputs, "clusters")
                    .map(|v| format!(", clusters = {}", j(v)))
                    .unwrap_or_default()
            ),
        ),
        "ss_unmatched_cc" => {
            let mut c = format!(
                "ss_cc_write({id}, {}, {}, {}, {}",
                num("conf_level")?,
                num("power")?,
                num("ratio_controls_per_case")?,
                num("percent_controls_exposed")?
            );
            if let Some(v) = opt(inputs, "odds_ratio") {
                c.push_str(&format!(", odds_ratio = {}", j(v)));
            } else if let Some(v) = opt(inputs, "percent_cases_exposed") {
                c.push_str(&format!(", p1_pct = {}", j(v)));
            }
            c.push(')');
            ("ss_cc_write", c)
        }
        "ss_cohort" => {
            let mut c = format!(
                "ss_cohort_write({id}, {}, {}, {}, {}",
                num("conf_level")?,
                num("power")?,
                num("ratio_unexposed_per_exposed")?,
                num("percent_unexposed_outcome")?
            );
            for (k, arg) in [
                ("percent_exposed_outcome", "percent_exposed"),
                ("odds_ratio", "odds_ratio"),
                ("risk_ratio", "risk_ratio"),
                ("risk_difference", "risk_difference"),
            ] {
                if let Some(v) = opt(inputs, k) {
                    c.push_str(&format!(", {arg} = {}", j(v)));
                }
            }
            c.push(')');
            ("ss_cohort_write", c)
        }
        "ss_mean_difference" => (
            "ss_mean_difference_write",
            format!(
                "ss_mean_difference_write({id}, {}, {}, {}, {}, {}, {})",
                num("conf_level")?,
                num("power")?,
                num("ratio")?,
                num("difference")?,
                num("sd_1")?,
                num("sd_2")?
            ),
        ),
        "power_unmatched_cc" => (
            "power_unmatched_cc_write",
            format!(
                "power_unmatched_cc_write({id}, {}, {}, {}, {}, {})",
                num("conf_level")?,
                num("n_cases")?,
                num("n_controls")?,
                num("percent_cases_exposed")?,
                num("percent_controls_exposed")?
            ),
        ),
        "power_cohort" => (
            "power_cohort_write",
            format!(
                "power_cohort_write({id}, {}, {}, {}, {}, {}, {})",
                num("conf_level")?,
                num("n_exposed")?,
                num("n_unexposed")?,
                num("percent_exposed_outcome")?,
                num("percent_unexposed_outcome")?,
                opt(inputs, "design")
                    .map(j)
                    .unwrap_or_else(|| "\"cohort\"".into())
            ),
        ),
        "power_mean_difference" => (
            "power_mean_difference_write",
            format!(
                "power_mean_difference_write({id}, {}, {}, {}, {}, {}, {})",
                num("conf_level")?,
                num("n_1")?,
                num("n_2")?,
                num("difference")?,
                num("sd_1")?,
                num("sd_2")?
            ),
        ),
        "random_numbers" => {
            return Err(not_exportable(
                "random_numbers draws come from a seeded ChaCha stream R cannot replay",
            ));
        }
        other => return Err(invalid(format!("no R writer for calc.{other}"))),
    };
    Ok((seed, call))
}
