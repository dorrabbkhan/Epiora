//! `rxc` registry entries: the corpus fixtures covering every
//! statistic the `rxc` family can emit (spec §4.3).

use crate::validation::{cov, Coverage, ToleranceClass};

const FIXTURES: &[&str] = &["rxc/table_3x2", "rxc/table_4x3"];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("rxc.chi_square.df", FIXTURES, ToleranceClass::ClosedForm),
    cov(
        "rxc.chi_square.p_value",
        FIXTURES,
        ToleranceClass::ClosedForm,
    ),
    cov(
        "rxc.chi_square.statistic",
        FIXTURES,
        ToleranceClass::ClosedForm,
    ),
];
