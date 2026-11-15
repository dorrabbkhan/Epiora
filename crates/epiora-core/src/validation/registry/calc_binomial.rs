//! `calc` registry coverage for `binomial` (v1.0 Task 7): every
//! leaf the calculator emits, mapped to the Task 2 fixtures that
//! cover it. Rows name only the fixtures that actually contain the
//! leaf.

use super::super::ToleranceClass::ClosedForm as CF;
use super::super::{cov, Coverage};

const BINOMIAL_0: &[&str] = &[
    "calc/binomial/large",
    "calc/binomial/textbook",
    "calc/binomial/zero",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.binomial.ci_95_count.lower", BINOMIAL_0, CF),
    cov("calc.binomial.ci_95_count.upper", BINOMIAL_0, CF),
    cov("calc.binomial.p_2t", BINOMIAL_0, CF),
    cov("calc.binomial.tail.eq", BINOMIAL_0, CF),
    cov("calc.binomial.tail.ge", BINOMIAL_0, CF),
    cov("calc.binomial.tail.gt", BINOMIAL_0, CF),
    cov("calc.binomial.tail.le", BINOMIAL_0, CF),
    cov("calc.binomial.tail.lt", BINOMIAL_0, CF),
];
