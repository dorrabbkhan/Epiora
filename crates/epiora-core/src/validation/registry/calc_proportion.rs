//! `calc` registry coverage for `proportion` (v1.0 Task 7): every
//! leaf the calculator emits, mapped to the Task 2 fixtures that
//! cover it. Rows name only the fixtures that actually contain the
//! leaf.

use super::super::ToleranceClass::{ClosedForm as CF, Iterative as IT};
use super::super::{cov, Coverage};

const PROPORTION_0: &[&str] = &[
    "calc/proportion/all_positive",
    "calc/proportion/finite_population",
    "calc/proportion/large",
    "calc/proportion/textbook",
    "calc/proportion/zero_numerator",
];
const PROPORTION_2: &[&str] = &["calc/proportion/finite_population"];
const PROPORTION_1: &[&str] = &[
    "calc/proportion/all_positive",
    "calc/proportion/finite_population",
    "calc/proportion/textbook",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.proportion.ci.agresti_coull.lower", PROPORTION_0, CF),
    cov("calc.proportion.ci.agresti_coull.upper", PROPORTION_0, CF),
    cov("calc.proportion.ci.clopper_pearson.lower", PROPORTION_0, IT),
    cov("calc.proportion.ci.clopper_pearson.upper", PROPORTION_0, IT),
    cov("calc.proportion.ci.midp.lower", PROPORTION_0, IT),
    cov("calc.proportion.ci.midp.upper", PROPORTION_0, IT),
    cov("calc.proportion.ci.score_cc.lower", PROPORTION_0, CF),
    cov("calc.proportion.ci.score_cc.upper", PROPORTION_0, CF),
    cov("calc.proportion.ci.wald.lower", PROPORTION_0, CF),
    cov("calc.proportion.ci.wald.upper", PROPORTION_0, CF),
    cov("calc.proportion.ci.wilson.lower", PROPORTION_0, CF),
    cov("calc.proportion.ci.wilson.upper", PROPORTION_0, CF),
    cov("calc.proportion.ci.wilson_fpc.lower", PROPORTION_2, CF),
    cov("calc.proportion.ci.wilson_fpc.upper", PROPORTION_2, CF),
    cov("calc.proportion.estimate", PROPORTION_0, CF),
    cov("calc.proportion.ztest.p", PROPORTION_1, CF),
    cov("calc.proportion.ztest.z", PROPORTION_1, CF),
];
