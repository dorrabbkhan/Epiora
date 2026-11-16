//! `calc` registry coverage for `rate_one` (v1.0 Task 8): every leaf
//! the calculator emits, mapped to the Task 2 fixtures that cover it.

use super::super::ToleranceClass::{ClosedForm as CF, Iterative as IT};
use super::super::{cov, Coverage};

const RATE_ONE: &[&str] = &[
    "calc/rate_one/boundary",
    "calc/rate_one/large",
    "calc/rate_one/textbook",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.rate_one.ci.byar.lower", RATE_ONE, CF),
    cov("calc.rate_one.ci.byar.upper", RATE_ONE, CF),
    cov("calc.rate_one.ci.fisher.lower", RATE_ONE, IT),
    cov("calc.rate_one.ci.fisher.upper", RATE_ONE, IT),
    cov("calc.rate_one.ci.midp.lower", RATE_ONE, IT),
    cov("calc.rate_one.ci.midp.upper", RATE_ONE, IT),
    cov("calc.rate_one.ci.normal.lower", RATE_ONE, CF),
    cov("calc.rate_one.ci.normal.upper", RATE_ONE, CF),
    cov("calc.rate_one.ci.rothman_greenland.lower", RATE_ONE, CF),
    cov("calc.rate_one.ci.rothman_greenland.upper", RATE_ONE, CF),
    cov("calc.rate_one.estimate", RATE_ONE, CF),
];
