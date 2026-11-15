//! `calc` registry coverage for `screening` (v1.0 Task 7): every
//! leaf the calculator emits, mapped to the Task 2 fixtures that
//! cover it. Rows name only the fixtures that actually contain the
//! leaf.

use super::super::ToleranceClass::ClosedForm as CF;
use super::super::{cov, Coverage};

const SCREENING_0: &[&str] = &[
    "calc/screening/boundary",
    "calc/screening/multi_level",
    "calc/screening/textbook",
];
const SCREENING_1: &[&str] = &["calc/screening/multi_level"];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.screening.accuracy.est", SCREENING_0, CF),
    cov("calc.screening.accuracy.lower", SCREENING_0, CF),
    cov("calc.screening.accuracy.upper", SCREENING_0, CF),
    cov("calc.screening.bias_index", SCREENING_0, CF),
    cov("calc.screening.diagnostic_odds.est", SCREENING_0, CF),
    cov("calc.screening.diagnostic_odds.lower", SCREENING_0, CF),
    cov("calc.screening.diagnostic_odds.upper", SCREENING_0, CF),
    cov("calc.screening.entropy.negative", SCREENING_0, CF),
    cov("calc.screening.entropy.positive", SCREENING_0, CF),
    cov("calc.screening.kappa.est", SCREENING_0, CF),
    cov("calc.screening.kappa.lower", SCREENING_0, CF),
    cov("calc.screening.kappa.upper", SCREENING_0, CF),
    cov("calc.screening.level_lr[].est", SCREENING_1, CF),
    cov("calc.screening.level_lr[].lower", SCREENING_1, CF),
    cov("calc.screening.level_lr[].upper", SCREENING_1, CF),
    cov("calc.screening.lr_neg.est", SCREENING_0, CF),
    cov("calc.screening.lr_neg.lower", SCREENING_0, CF),
    cov("calc.screening.lr_neg.upper", SCREENING_0, CF),
    cov("calc.screening.lr_pos.est", SCREENING_0, CF),
    cov("calc.screening.lr_pos.lower", SCREENING_0, CF),
    cov("calc.screening.lr_pos.upper", SCREENING_0, CF),
    cov("calc.screening.npv.est", SCREENING_0, CF),
    cov("calc.screening.npv.lower", SCREENING_0, CF),
    cov("calc.screening.npv.upper", SCREENING_0, CF),
    cov("calc.screening.ppv.est", SCREENING_0, CF),
    cov("calc.screening.ppv.lower", SCREENING_0, CF),
    cov("calc.screening.ppv.upper", SCREENING_0, CF),
    cov("calc.screening.sensitivity.est", SCREENING_0, CF),
    cov("calc.screening.sensitivity.lower", SCREENING_0, CF),
    cov("calc.screening.sensitivity.upper", SCREENING_0, CF),
    cov("calc.screening.specificity.est", SCREENING_0, CF),
    cov("calc.screening.specificity.lower", SCREENING_0, CF),
    cov("calc.screening.specificity.upper", SCREENING_0, CF),
];
