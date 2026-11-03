//! `freq` registry entries: the corpus fixtures covering every
//! statistic the `freq` family can emit (spec §4.3).

use crate::validation::{cov, Coverage, ToleranceClass};

const FIXTURES: &[&str] = &[
    "freq/missing_group",
    "freq/missing_group_incl_missing",
    "freq/oswego_age",
    "freq/oswego_sex",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov(
        "freq.categories[].ci_95_clopper_pearson.lower",
        FIXTURES,
        ToleranceClass::ClosedForm,
    ),
    cov(
        "freq.categories[].ci_95_clopper_pearson.upper",
        FIXTURES,
        ToleranceClass::ClosedForm,
    ),
    cov(
        "freq.categories[].ci_95_wilson.lower",
        FIXTURES,
        ToleranceClass::ClosedForm,
    ),
    cov(
        "freq.categories[].ci_95_wilson.upper",
        FIXTURES,
        ToleranceClass::ClosedForm,
    ),
    cov(
        "freq.categories[].count",
        FIXTURES,
        ToleranceClass::ClosedForm,
    ),
    cov(
        "freq.categories[].cumulative_percent",
        FIXTURES,
        ToleranceClass::ClosedForm,
    ),
    cov(
        "freq.categories[].percent",
        FIXTURES,
        ToleranceClass::ClosedForm,
    ),
    cov("freq.missing", FIXTURES, ToleranceClass::ClosedForm),
    cov("freq.total", FIXTURES, ToleranceClass::ClosedForm),
];
