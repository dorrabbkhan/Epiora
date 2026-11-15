//! `calc` registry coverage for `r_by_c` (v1.0 Task 7): every
//! leaf the calculator emits, mapped to the Task 2 fixtures that
//! cover it. Rows name only the fixtures that actually contain the
//! leaf.

use super::super::ToleranceClass::ClosedForm as CF;
use super::super::{cov, Coverage};

const R_BY_C_0: &[&str] = &[
    "calc/r_by_c/large",
    "calc/r_by_c/sparse",
    "calc/r_by_c/textbook",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.r_by_c.chisq.df", R_BY_C_0, CF),
    cov("calc.r_by_c.chisq.p", R_BY_C_0, CF),
    cov("calc.r_by_c.chisq.value", R_BY_C_0, CF),
    cov("calc.r_by_c.expected_lt1.count", R_BY_C_0, CF),
    cov("calc.r_by_c.expected_lt5.count", R_BY_C_0, CF),
    cov("calc.r_by_c.expected_lt5.percent", R_BY_C_0, CF),
];
