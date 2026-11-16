//! Registry data for the Task 8 person-time/continuous
//! calculators, declared as `docs/v1.0-calculators.md` inventory
//! metadata; they are registered but not runnable in Task 7.

use crate::calc::{
    idf, od, CalculatorDef, CellKind, DefaultValue, Family, InputDef, InputGroup, InputKind,
    OutputDef,
};
use crate::validation::ToleranceClass::ClosedForm as CF;
const MEAN_CI_INPUTS: &[InputDef] = &[
    idf(
        "mean",
        InputKind::Real {
            min: f64::MIN,
            max: f64::MAX,
        },
        None,
        None,
    ),
    idf(
        "sd_or",
        InputKind::Choice {
            options: &["sd", "se", "variance"],
        },
        None,
        None,
    ),
    idf(
        "sd_value",
        InputKind::Real {
            min: f64::MIN_POSITIVE,
            max: f64::MAX,
        },
        None,
        None,
    ),
    idf(
        "n",
        InputKind::Integer {
            min: 2,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "population_size",
        InputKind::Count,
        Some(DefaultValue::Number(999999999.0)),
        None,
    ),
    idf(
        "conf_level",
        InputKind::ConfLevel,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
];

const MEAN_CI_OUTPUTS: &[OutputDef] = &[
    od("calc.mean_ci.t.lower", CF, "`stats::qt`"),
    od("calc.mean_ci.t.upper", CF, "`stats::qt`"),
    od("calc.mean_ci.z.lower", CF, "`fixtures/R/methods/means.R::mean_ci` — z-CI with √((N−n)/(N−1)) FPC, Cochran, Sampling Techniques 1977 §2.8, as in `CIMean.js`"),
    od("calc.mean_ci.z.upper", CF, "`fixtures/R/methods/means.R::mean_ci` — z-CI with √((N−n)/(N−1)) FPC, Cochran, Sampling Techniques 1977 §2.8, as in `CIMean.js`"),
];
pub(super) static MEAN_CI: CalculatorDef = CalculatorDef {
    id: "mean_ci",
    family: Family::Continuous,
    inputs: MEAN_CI_INPUTS,
    groups: &[],
    outputs: MEAN_CI_OUTPUTS,
};

const MEDIAN_CI_INPUTS: &[InputDef] = &[
    idf(
        "n",
        InputKind::Integer {
            min: 2,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "percentile",
        InputKind::Percent,
        Some(DefaultValue::Number(50.0)),
        None,
    ),
    idf(
        "conf_level",
        InputKind::ConfLevel,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
];

const MEDIAN_CI_OUTPUTS: &[OutputDef] = &[
    od(
        "calc.median_ci.normal.lower_rank",
        CF,
        "`fixtures/R/methods/means.R::percentile_ci` — Conover, Practical Nonparametric Statistics",
    ),
    od(
        "calc.median_ci.normal.upper_rank",
        CF,
        "`fixtures/R/methods/means.R::percentile_ci` — Conover, Practical Nonparametric Statistics",
    ),
];
pub(super) static MEDIAN_CI: CalculatorDef = CalculatorDef {
    id: "median_ci",
    family: Family::Continuous,
    inputs: MEDIAN_CI_INPUTS,
    groups: &[],
    outputs: MEDIAN_CI_OUTPUTS,
};

const T_TEST_SUMMARY_INPUTS: &[InputDef] = &[
    idf(
        "conf_level",
        InputKind::ConfLevel,
        Some(DefaultValue::Number(95.0)),
        None,
    ),
    idf(
        "n_1",
        InputKind::Integer {
            min: 2,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "mean_1",
        InputKind::Real {
            min: f64::MIN,
            max: f64::MAX,
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
        "se_1",
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
        "n_2",
        InputKind::Integer {
            min: 2,
            max: i64::MAX,
        },
        None,
        None,
    ),
    idf(
        "mean_2",
        InputKind::Real {
            min: f64::MIN,
            max: f64::MAX,
        },
        None,
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
        "se_2",
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

/// Inventory `## t_test_summary`: per group, exactly one of
/// `sd_i`/`se_i`/`var_i` — the canonical `sd` first.
const T_TEST_SUMMARY_GROUPS: &[InputGroup] = &[
    InputGroup {
        members: &["sd_1", "se_1", "var_1"],
    },
    InputGroup {
        members: &["sd_2", "se_2", "var_2"],
    },
];

const T_TEST_SUMMARY_OUTPUTS: &[OutputDef] = &[
    od(
        "calc.t_test_summary.f_variance.df_den",
        CF,
        "`stats::var.test`",
    ),
    od(
        "calc.t_test_summary.f_variance.df_num",
        CF,
        "`stats::var.test`",
    ),
    od("calc.t_test_summary.f_variance.f", CF, "`stats::var.test`"),
    od("calc.t_test_summary.f_variance.p", CF, "`stats::var.test`"),
    od(
        "calc.t_test_summary.pooled.df",
        CF,
        "`stats::t.test(var.equal=TRUE)` equivalent",
    ),
    od(
        "calc.t_test_summary.pooled.diff",
        CF,
        "`stats::t.test(var.equal=TRUE)` equivalent",
    ),
    od(
        "calc.t_test_summary.pooled.lower",
        CF,
        "`stats::t.test(var.equal=TRUE)` equivalent",
    ),
    od(
        "calc.t_test_summary.pooled.p_2t",
        CF,
        "`stats::t.test(var.equal=TRUE)` equivalent",
    ),
    od(
        "calc.t_test_summary.pooled.t",
        CF,
        "`stats::t.test(var.equal=TRUE)` equivalent",
    ),
    od(
        "calc.t_test_summary.pooled.upper",
        CF,
        "`stats::t.test(var.equal=TRUE)` equivalent",
    ),
    od(
        "calc.t_test_summary.welch.df",
        CF,
        "`stats::t.test(var.equal=FALSE)`",
    ),
    od(
        "calc.t_test_summary.welch.diff",
        CF,
        "`stats::t.test(var.equal=FALSE)`",
    ),
    od(
        "calc.t_test_summary.welch.lower",
        CF,
        "`stats::t.test(var.equal=FALSE)`",
    ),
    od(
        "calc.t_test_summary.welch.p_2t",
        CF,
        "`stats::t.test(var.equal=FALSE)`",
    ),
    od(
        "calc.t_test_summary.welch.t",
        CF,
        "`stats::t.test(var.equal=FALSE)`",
    ),
    od(
        "calc.t_test_summary.welch.upper",
        CF,
        "`stats::t.test(var.equal=FALSE)`",
    ),
];
pub(super) static T_TEST_SUMMARY: CalculatorDef = CalculatorDef {
    id: "t_test_summary",
    family: Family::Continuous,
    inputs: T_TEST_SUMMARY_INPUTS,
    groups: T_TEST_SUMMARY_GROUPS,
    outputs: T_TEST_SUMMARY_OUTPUTS,
};

const ANOVA_SUMMARY_INPUTS: &[InputDef] = &[idf(
    "table",
    InputKind::Table {
        cols: 3,
        min_rows: 2,
        max_rows: 10,
        cell: CellKind::Real,
        strata: None,
    },
    None,
    None,
)];

const ANOVA_SUMMARY_OUTPUTS: &[OutputDef] = &[
    od("calc.anova_summary.anova.df_between", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.anova.df_total", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.anova.df_within", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.anova.f", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.anova.ms_between", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.anova.ms_within", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.anova.p", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.anova.ss_between", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.anova.ss_total", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.anova.ss_within", CF, "`fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/mean/sd as in `ANOVA.js`"),
    od("calc.anova_summary.bartlett.chisq", CF, "`fixtures/R/methods/means.R::bartlett_from_summary` — Bartlett 1937 formula as in `ANOVA.js` (`stats::bartlett.test` needs raw data)"),
    od("calc.anova_summary.bartlett.df", CF, "`fixtures/R/methods/means.R::bartlett_from_summary` — Bartlett 1937 formula as in `ANOVA.js` (`stats::bartlett.test` needs raw data)"),
    od("calc.anova_summary.bartlett.p", CF, "`fixtures/R/methods/means.R::bartlett_from_summary` — Bartlett 1937 formula as in `ANOVA.js` (`stats::bartlett.test` needs raw data)"),
    od("calc.anova_summary.group_ci[].lower", CF, "fixtures/R/calc.R"),
    od("calc.anova_summary.group_ci[].upper", CF, "fixtures/R/calc.R"),
    od("calc.anova_summary.group_ci_pooled[].lower", CF, "fixtures/R/calc.R"),
    od("calc.anova_summary.group_ci_pooled[].upper", CF, "fixtures/R/calc.R"),
];
pub(super) static ANOVA_SUMMARY: CalculatorDef = CalculatorDef {
    id: "anova_summary",
    family: Family::Continuous,
    inputs: ANOVA_SUMMARY_INPUTS,
    groups: &[],
    outputs: ANOVA_SUMMARY_OUTPUTS,
};
