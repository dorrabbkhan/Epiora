//! Registry data for the Task 9 calculators, declared as
//! `docs/v1.0-calculators.md` inventory metadata; they are
//! registered but not runnable in Task 7.

use crate::calc::{idf, od, CalculatorDef, DefaultValue, Family, InputDef, InputKind, OutputDef};
use crate::validation::ToleranceClass::ClosedForm as CF;
const SS_COHORT_INPUTS: &[InputDef] = &[
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
        "ratio_unexposed_per_exposed",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        Some(DefaultValue::Number(1.0)),
        None,
    ),
    idf(
        "percent_unexposed_outcome",
        InputKind::Real {
            min: 0.0,
            max: 99.9,
        },
        Some(DefaultValue::Number(5.0)),
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
        "percent_exposed_outcome",
        InputKind::Percent,
        Some(DefaultValue::Omit),
        None,
    ),
    idf(
        "risk_ratio",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
    idf(
        "risk_difference",
        InputKind::Real {
            min: f64::MIN,
            max: f64::MAX,
        },
        Some(DefaultValue::Omit),
        None,
    ),
];

const SS_COHORT_OUTPUTS: &[OutputDef] = &[
    od(
        "calc.ss_cohort.fleiss.exposed",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss.total",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss.unexposed",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss_cc.exposed",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss_cc.total",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss_cc.unexposed",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.kelsey.exposed",
        CF,
        "`fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.kelsey.total",
        CF,
        "`fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.kelsey.unexposed",
        CF,
        "`fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc`",
    ),
];
pub(super) static SS_COHORT: CalculatorDef = CalculatorDef {
    id: "ss_cohort",
    family: Family::SampleSize,
    inputs: SS_COHORT_INPUTS,
    groups: &[],
    outputs: SS_COHORT_OUTPUTS,
};

const SS_MEAN_DIFFERENCE_INPUTS: &[InputDef] = &[
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
        "ratio",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        Some(DefaultValue::Number(1.0)),
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

const SS_MEAN_DIFFERENCE_OUTPUTS: &[OutputDef] = &[
    od(
        "calc.ss_mean_difference.n1",
        CF,
        "`fixtures/R/methods/sample_size.R::ss_means` — OpenEpi `SSMean.js` formula",
    ),
    od(
        "calc.ss_mean_difference.n2",
        CF,
        "`fixtures/R/methods/sample_size.R::ss_means` — OpenEpi `SSMean.js` formula",
    ),
    od(
        "calc.ss_mean_difference.total",
        CF,
        "`fixtures/R/methods/sample_size.R::ss_means` — OpenEpi `SSMean.js` formula",
    ),
];
pub(super) static SS_MEAN_DIFFERENCE: CalculatorDef = CalculatorDef {
    id: "ss_mean_difference",
    family: Family::SampleSize,
    inputs: SS_MEAN_DIFFERENCE_INPUTS,
    groups: &[],
    outputs: SS_MEAN_DIFFERENCE_OUTPUTS,
};
