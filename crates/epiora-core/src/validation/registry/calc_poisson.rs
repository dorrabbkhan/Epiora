//! `calc` registry coverage for `poisson` (v1.0 Task 7): every
//! leaf the calculator emits, mapped to the Task 2 fixtures that
//! cover it. Rows name only the fixtures that actually contain the
//! leaf.

use super::super::ToleranceClass::ClosedForm as CF;
use super::super::{cov, Coverage};

const POISSON_0: &[&str] = &[
    "calc/poisson/large",
    "calc/poisson/textbook",
    "calc/poisson/zero",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.poisson.tail.eq", POISSON_0, CF),
    cov("calc.poisson.tail.ge", POISSON_0, CF),
    cov("calc.poisson.tail.gt", POISSON_0, CF),
    cov("calc.poisson.tail.le", POISSON_0, CF),
    cov("calc.poisson.tail.lt", POISSON_0, CF),
];
