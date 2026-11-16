//! Registry data for the Task 9 calculators, declared as
//! `docs/v1.0-calculators.md` inventory metadata; they are
//! registered but not runnable in Task 7.

use crate::calc::{idf, od, CalculatorDef, DefaultValue, Family, InputDef, InputKind, OutputDef};
use crate::validation::ToleranceClass::ClosedForm as CF;
const POWER_UNMATCHED_CC_INPUTS: &[InputDef] = &[
    idf(
        "conf_level",
        InputKind::ConfLevel,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
    idf(
        "n_cases",
        InputKind::Integer {
            min: 1,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "n_controls",
        InputKind::Integer {
            min: 1,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf("percent_cases_exposed", InputKind::Percent, None, None),
    idf("percent_controls_exposed", InputKind::Percent, None, None),
];

const POWER_UNMATCHED_CC_OUTPUTS: &[OutputDef] = &[
    od("calc.power_unmatched_cc.normal", CF, "`fixtures/R/methods/sample_size.R::power_cc` — OpenEpi `PowerCC.js` (Fleiss §3.18 inverted)"),
    od("calc.power_unmatched_cc.normal_cc", CF, "`fixtures/R/methods/sample_size.R::power_cc_cc` — same"),
    od("calc.power_unmatched_cc.odds_ratio", CF, "`fixtures/R/methods/sample_size.R::implied_or` — OpenEpi `PowerCC.js` (odds of exposure, cases vs controls)"),
];
pub(super) static POWER_UNMATCHED_CC: CalculatorDef = CalculatorDef {
    id: "power_unmatched_cc",
    family: Family::Power,
    inputs: POWER_UNMATCHED_CC_INPUTS,
    groups: &[],
    outputs: POWER_UNMATCHED_CC_OUTPUTS,
};

const POWER_COHORT_INPUTS: &[InputDef] = &[
    idf(
        "design",
        InputKind::Choice {
            options: &["cohort", "rct", "cross_sectional"],
        },
        Some(DefaultValue::Choice("cohort")),
        None,
    ),
    idf(
        "conf_level",
        InputKind::ConfLevel,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
    idf(
        "n_exposed",
        InputKind::Integer {
            min: 1,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "n_unexposed",
        InputKind::Integer {
            min: 1,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf("percent_exposed_outcome", InputKind::Percent, None, None),
    idf("percent_unexposed_outcome", InputKind::Percent, None, None),
];

const POWER_COHORT_OUTPUTS: &[OutputDef] = &[
    od("calc.power_cohort.normal", CF, "`fixtures/R/methods/sample_size.R::power_cohort` — OpenEpi `PowerCohort.js` (same code in RCT/Cross)"),
    od("calc.power_cohort.normal_cc", CF, "`fixtures/R/methods/sample_size.R::power_cohort_cc` — same source"),
    od("calc.power_cohort.risk_difference", CF, "fixtures/R/calc.R"),
    od("calc.power_cohort.risk_ratio", CF, "fixtures/R/calc.R"),
];
pub(super) static POWER_COHORT: CalculatorDef = CalculatorDef {
    id: "power_cohort",
    family: Family::Power,
    inputs: POWER_COHORT_INPUTS,
    groups: &[],
    outputs: POWER_COHORT_OUTPUTS,
};

const POWER_MEAN_DIFFERENCE_INPUTS: &[InputDef] = &[
    idf(
        "conf_level",
        InputKind::ConfLevel,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
    idf(
        "difference",
        InputKind::Real {
            min: f64::MIN,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
    idf(
        "mean_1",
        InputKind::Real {
            min: f64::MIN,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
    idf(
        "mean_2",
        InputKind::Real {
            min: f64::MIN,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
    idf(
        "n_1",
        InputKind::Integer {
            min: 1,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "n_2",
        InputKind::Integer {
            min: 1,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "sd_1",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
    idf(
        "sd_2",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
    idf(
        "var_1",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
    idf(
        "var_2",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
];

const POWER_MEAN_DIFFERENCE_OUTPUTS: &[OutputDef] = &[od(
    "calc.power_mean_difference.power",
    CF,
    "`fixtures/R/methods/sample_size.R::power_means` — OpenEpi `PowerMean.js` z-formula",
)];
pub(super) static POWER_MEAN_DIFFERENCE: CalculatorDef = CalculatorDef {
    id: "power_mean_difference",
    family: Family::Power,
    inputs: POWER_MEAN_DIFFERENCE_INPUTS,
    groups: &[],
    outputs: POWER_MEAN_DIFFERENCE_OUTPUTS,
};

const RANDOM_NUMBERS_INPUTS: &[InputDef] = &[
    idf(
        "lower",
        InputKind::Integer {
            min: i64::MIN,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "upper",
        InputKind::Integer {
            min: i64::MIN,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "count",
        InputKind::Integer { min: 1, max: 10000 },
        None,
        None,
    ),
    idf("seed", InputKind::Count, None, None),
    idf(
        "columns",
        InputKind::Integer { min: 1, max: 20 },
        Some(DefaultValue::Number(1.0)),
        None,
    ),
];

const RANDOM_NUMBERS_OUTPUTS: &[OutputDef] = &[];
pub(super) static RANDOM_NUMBERS: CalculatorDef = CalculatorDef {
    id: "random_numbers",
    family: Family::Other,
    inputs: RANDOM_NUMBERS_INPUTS,
    groups: &[],
    outputs: RANDOM_NUMBERS_OUTPUTS,
};
