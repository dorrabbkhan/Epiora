//! `calc` registry coverage for the power calculators
//! `power_unmatched_cc`, `power_cohort` and `power_mean_difference`
//! (v1.0 Task 9): every leaf each emits, mapped to the Task 2
//! fixtures that cover it.

use super::super::ToleranceClass::ClosedForm as CF;
use super::super::{cov, Coverage};

const POWER_UNMATCHED_CC: &[&str] = &[
    "calc/power_unmatched_cc/large",
    "calc/power_unmatched_cc/rare_exposure",
    "calc/power_unmatched_cc/small_effect",
    "calc/power_unmatched_cc/textbook",
];

const POWER_COHORT: &[&str] = &[
    "calc/power_cohort/cohort",
    "calc/power_cohort/cross_sectional",
    "calc/power_cohort/rct",
];

const POWER_MEAN_DIFFERENCE: &[&str] = &[
    "calc/power_mean_difference/large",
    "calc/power_mean_difference/small_effect",
    "calc/power_mean_difference/textbook",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.power_unmatched_cc.normal", POWER_UNMATCHED_CC, CF),
    cov("calc.power_unmatched_cc.normal_cc", POWER_UNMATCHED_CC, CF),
    cov("calc.power_unmatched_cc.odds_ratio", POWER_UNMATCHED_CC, CF),
    cov("calc.power_cohort.normal", POWER_COHORT, CF),
    cov("calc.power_cohort.normal_cc", POWER_COHORT, CF),
    cov("calc.power_cohort.risk_difference", POWER_COHORT, CF),
    cov("calc.power_cohort.risk_ratio", POWER_COHORT, CF),
    cov(
        "calc.power_mean_difference.power",
        POWER_MEAN_DIFFERENCE,
        CF,
    ),
];
