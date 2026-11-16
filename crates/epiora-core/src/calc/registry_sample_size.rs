//! Registry data for the Task 9 sample-size calculators
//! (`ss_proportion`, `ss_unmatched_cc`), declared as
//! `docs/v1.0-calculators.md` inventory metadata; registered and
//! runnable since Task 9.

use crate::calc::{
    idf, od, CalculatorDef, DefaultValue, Family, InputDef, InputGroup, InputKind, OutputDef,
};
use crate::validation::ToleranceClass::ClosedForm as CF;
const SS_PROPORTION_INPUTS: &[InputDef] = &[
    idf(
        "population_size",
        InputKind::Count,
        Some(DefaultValue::Number(1000000.0)),
        None,
    ),
    idf(
        "expected_freq",
        InputKind::Percent,
        Some(DefaultValue::Number(50.0)),
        None,
    ),
    idf(
        "margin_error",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: 100.0,
        },
        Some(DefaultValue::Number(5.0)),
        None,
    ),
    idf(
        "design_effect",
        InputKind::Real {
            min: 1.0,
            max: f64::MAX,
        },
        Some(DefaultValue::Number(1.0)),
        None,
    ),
    idf(
        "clusters",
        InputKind::Integer {
            min: 1,
            max: i64::MAX,
        },
        Some(DefaultValue::Number(1.0)),
        None,
    ),
];

const SS_PROPORTION_OUTPUTS: &[OutputDef] = &[
    od("calc.ss_proportion.cluster_size.80", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.cluster_size.90", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.cluster_size.95", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.cluster_size.97", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.cluster_size.99", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.cluster_size.999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.cluster_size.9999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.cluster_size_real.80", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.cluster_size_real.90", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.cluster_size_real.95", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.cluster_size_real.97", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.cluster_size_real.99", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.cluster_size_real.999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.cluster_size_real.9999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.n_ei7.80", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `PopulationSurvey.xaml.cs`"),
    od("calc.ss_proportion.n_ei7.90", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `PopulationSurvey.xaml.cs`"),
    od("calc.ss_proportion.n_ei7.95", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `PopulationSurvey.xaml.cs`"),
    od("calc.ss_proportion.n_ei7.97", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `PopulationSurvey.xaml.cs`"),
    od("calc.ss_proportion.n_ei7.99", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `PopulationSurvey.xaml.cs`"),
    od("calc.ss_proportion.n_ei7.999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `PopulationSurvey.xaml.cs`"),
    od("calc.ss_proportion.n_ei7.9999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `PopulationSurvey.xaml.cs`"),
    od("calc.ss_proportion.n_ei7_real.80", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.n_ei7_real.90", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.n_ei7_real.95", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.n_ei7_real.97", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.n_ei7_real.99", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.n_ei7_real.999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.n_ei7_real.9999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7`"),
    od("calc.ss_proportion.n_openepi.80", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPropor.js`"),
    od("calc.ss_proportion.n_openepi.90", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPropor.js`"),
    od("calc.ss_proportion.n_openepi.95", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPropor.js`"),
    od("calc.ss_proportion.n_openepi.97", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPropor.js`"),
    od("calc.ss_proportion.n_openepi.99", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPropor.js`"),
    od("calc.ss_proportion.n_openepi.999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPropor.js`"),
    od("calc.ss_proportion.n_openepi.9999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPropor.js`"),
    od("calc.ss_proportion.n_openepi_real.80", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi`"),
    od("calc.ss_proportion.n_openepi_real.90", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi`"),
    od("calc.ss_proportion.n_openepi_real.95", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi`"),
    od("calc.ss_proportion.n_openepi_real.97", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi`"),
    od("calc.ss_proportion.n_openepi_real.99", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi`"),
    od("calc.ss_proportion.n_openepi_real.999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi`"),
    od("calc.ss_proportion.n_openepi_real.9999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi`"),
    od("calc.ss_proportion.total.80", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.total.90", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.total.95", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.total.97", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.total.99", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.total.999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
    od("calc.ss_proportion.total.9999", CF, "`fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size`"),
];
pub(super) static SS_PROPORTION: CalculatorDef = CalculatorDef {
    id: "ss_proportion",
    family: Family::SampleSize,
    inputs: SS_PROPORTION_INPUTS,
    groups: &[],
    outputs: SS_PROPORTION_OUTPUTS,
};

const SS_UNMATCHED_CC_INPUTS: &[InputDef] = &[
    idf(
        "conf_level",
        InputKind::ConfLevel,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
    idf(
        "power",
        InputKind::Percent,
        Some(DefaultValue::Number(80.0)),
        None,
    ),
    idf(
        "ratio_controls_per_case",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        Some(DefaultValue::Number(1.0)),
        None,
    ),
    idf(
        "percent_controls_exposed",
        InputKind::Real {
            min: 0.0,
            max: 99.99,
        },
        Some(DefaultValue::Number(40.0)),
        None,
    ),
    idf(
        "odds_ratio",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
    idf(
        "percent_cases_exposed",
        InputKind::Percent,
        Some(DefaultValue::Omit),
        None,
    ),
];

const SS_UNMATCHED_CC_OUTPUTS: &[OutputDef] = &[
    od("calc.ss_unmatched_cc.fleiss.cases", CF, "`fixtures/R/methods/sample_size.R::fleiss` — Fleiss, Statistical Methods for Rates and Proportions, formulas 3.18–3.19"),
    od("calc.ss_unmatched_cc.fleiss.cases_real", CF, "`fixtures/R/methods/sample_size.R::fleiss` — Fleiss, Statistical Methods for Rates and Proportions, formulas 3.18–3.19"),
    od("calc.ss_unmatched_cc.fleiss.controls", CF, "`fixtures/R/methods/sample_size.R::fleiss` — Fleiss, Statistical Methods for Rates and Proportions, formulas 3.18–3.19"),
    od("calc.ss_unmatched_cc.fleiss.controls_real", CF, "`fixtures/R/methods/sample_size.R::fleiss` — Fleiss, Statistical Methods for Rates and Proportions, formulas 3.18–3.19"),
    od("calc.ss_unmatched_cc.fleiss.total", CF, "`fixtures/R/methods/sample_size.R::fleiss` — Fleiss, Statistical Methods for Rates and Proportions, formulas 3.18–3.19"),
    od("calc.ss_unmatched_cc.fleiss.total_real", CF, "`fixtures/R/methods/sample_size.R::fleiss` — Fleiss, Statistical Methods for Rates and Proportions, formulas 3.18–3.19"),
    od("calc.ss_unmatched_cc.fleiss_cc.cases", CF, "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source"),
    od("calc.ss_unmatched_cc.fleiss_cc.cases_real", CF, "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source"),
    od("calc.ss_unmatched_cc.fleiss_cc.controls", CF, "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source"),
    od("calc.ss_unmatched_cc.fleiss_cc.controls_real", CF, "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source"),
    od("calc.ss_unmatched_cc.fleiss_cc.total", CF, "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source"),
    od("calc.ss_unmatched_cc.fleiss_cc.total_real", CF, "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source"),
    od("calc.ss_unmatched_cc.kelsey.cases", CF, "`fixtures/R/methods/sample_size.R::kelsey` — Kelsey et al., Methods in Observational Epidemiology 2nd ed., Table 12-15"),
    od("calc.ss_unmatched_cc.kelsey.cases_real", CF, "`fixtures/R/methods/sample_size.R::kelsey` — Kelsey et al., Methods in Observational Epidemiology 2nd ed., Table 12-15"),
    od("calc.ss_unmatched_cc.kelsey.controls", CF, "`fixtures/R/methods/sample_size.R::kelsey` — Kelsey et al., Methods in Observational Epidemiology 2nd ed., Table 12-15"),
    od("calc.ss_unmatched_cc.kelsey.controls_real", CF, "`fixtures/R/methods/sample_size.R::kelsey` — Kelsey et al., Methods in Observational Epidemiology 2nd ed., Table 12-15"),
    od("calc.ss_unmatched_cc.kelsey.total", CF, "`fixtures/R/methods/sample_size.R::kelsey` — Kelsey et al., Methods in Observational Epidemiology 2nd ed., Table 12-15"),
    od("calc.ss_unmatched_cc.kelsey.total_real", CF, "`fixtures/R/methods/sample_size.R::kelsey` — Kelsey et al., Methods in Observational Epidemiology 2nd ed., Table 12-15"),
];
const SS_UNMATCHED_CC_GROUPS: &[InputGroup] = &[InputGroup {
    members: &["odds_ratio", "percent_cases_exposed"],
}];
pub(super) static SS_UNMATCHED_CC: CalculatorDef = CalculatorDef {
    id: "ss_unmatched_cc",
    family: Family::SampleSize,
    inputs: SS_UNMATCHED_CC_INPUTS,
    groups: SS_UNMATCHED_CC_GROUPS,
    outputs: SS_UNMATCHED_CC_OUTPUTS,
};
