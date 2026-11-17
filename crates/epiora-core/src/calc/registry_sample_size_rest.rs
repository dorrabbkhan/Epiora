//! Registry data for the remaining Task 9 sample-size calculators
//! (`ss_cohort`, `ss_mean_difference`), declared as
//! `docs/v1.0-calculators.md` inventory metadata; registered and
//! runnable since Task 9.

use crate::calc::{
    idf, od, CalculatorDef, DefaultValue, Family, InputDef, InputGroup, InputKind, OutputDef,
};
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
        "`fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc`; OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles — alternative display (default: kelsey)",
    ),
    od(
        "calc.ss_cohort.fleiss.exposed_real",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss.total",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc`; OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles — alternative display (default: kelsey)",
    ),
    od(
        "calc.ss_cohort.fleiss.total_real",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss.unexposed",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc`; OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles — alternative display (default: kelsey)",
    ),
    od(
        "calc.ss_cohort.fleiss.unexposed_real",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss_cc.exposed",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc`; OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles — alternative display (default: kelsey)",
    ),
    od(
        "calc.ss_cohort.fleiss_cc.exposed_real",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss_cc.total",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc`; OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles — alternative display (default: kelsey)",
    ),
    od(
        "calc.ss_cohort.fleiss_cc.total_real",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.fleiss_cc.unexposed",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc`; OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles — alternative display (default: kelsey)",
    ),
    od(
        "calc.ss_cohort.fleiss_cc.unexposed_real",
        CF,
        "`fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.kelsey.exposed",
        CF,
        "`fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc`; OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles — default display",
    ),
    od(
        "calc.ss_cohort.kelsey.exposed_real",
        CF,
        "`fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.kelsey.total",
        CF,
        "`fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc`; OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles — default display",
    ),
    od(
        "calc.ss_cohort.kelsey.total_real",
        CF,
        "`fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc`",
    ),
    od(
        "calc.ss_cohort.kelsey.unexposed",
        CF,
        "`fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc`; OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles — default display",
    ),
    od(
        "calc.ss_cohort.kelsey.unexposed_real",
        CF,
        "`fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc`",
    ),
];
const SS_COHORT_GROUPS: &[InputGroup] = &[InputGroup {
    members: &[
        "odds_ratio",
        "percent_exposed_outcome",
        "risk_ratio",
        "risk_difference",
    ],
}];
pub(super) static SS_COHORT: CalculatorDef = CalculatorDef {
    id: "ss_cohort",
    family: Family::SampleSize,
    inputs: SS_COHORT_INPUTS,
    groups: SS_COHORT_GROUPS,
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
        "`fixtures/R/methods/sample_size.R::ss_means` — OpenEpi `SSMean.js` formula; OpenEpi shows n₁ = ⌈(s₁²+s₂²/r)(z_α+z_β)²/Δ²⌉ — a z formula, not power.t.test — default display",
    ),
    od(
        "calc.ss_mean_difference.n2",
        CF,
        "`fixtures/R/methods/sample_size.R::ss_means` — OpenEpi `SSMean.js` formula; OpenEpi shows n₁ = ⌈(s₁²+s₂²/r)(z_α+z_β)²/Δ²⌉ — a z formula, not power.t.test — default display",
    ),
    od(
        "calc.ss_mean_difference.n1_real",
        CF,
        "`fixtures/R/methods/sample_size.R::ss_means` — same source as `n1`/`n2`/`total`",
    ),
    od(
        "calc.ss_mean_difference.n2_real",
        CF,
        "`fixtures/R/methods/sample_size.R::ss_means` — same source as `n1`/`n2`/`total`",
    ),
    od(
        "calc.ss_mean_difference.total",
        CF,
        "`fixtures/R/methods/sample_size.R::ss_means` — OpenEpi `SSMean.js` formula; OpenEpi shows n₁ = ⌈(s₁²+s₂²/r)(z_α+z_β)²/Δ²⌉ — a z formula, not power.t.test — default display",
    ),
    od(
        "calc.ss_mean_difference.total_real",
        CF,
        "`fixtures/R/methods/sample_size.R::ss_means` — same source as `n1`/`n2`/`total`",
    ),
];
const SS_MEAN_DIFFERENCE_GROUPS: &[InputGroup] = &[
    // `mean_1`/`mean_2` together XOR `difference` — each mean is one
    // alternative to it, so giving all three, or one mean with
    // `difference`, is `bad_choice`.
    InputGroup {
        members: &["mean_1", "difference"],
    },
    InputGroup {
        members: &["mean_2", "difference"],
    },
    InputGroup {
        members: &["sd_1", "var_1"],
    },
    InputGroup {
        members: &["sd_2", "var_2"],
    },
];
pub(super) static SS_MEAN_DIFFERENCE: CalculatorDef = CalculatorDef {
    id: "ss_mean_difference",
    family: Family::SampleSize,
    inputs: SS_MEAN_DIFFERENCE_INPUTS,
    groups: SS_MEAN_DIFFERENCE_GROUPS,
    outputs: SS_MEAN_DIFFERENCE_OUTPUTS,
};
