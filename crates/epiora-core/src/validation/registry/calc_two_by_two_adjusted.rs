//! `calc` registry coverage for `two_by_two` `adjusted` output
//! (v1.0 Task 7): every leaf the calculator emits, mapped to the
//! Task 2 fixtures that cover it.

use super::super::ToleranceClass::{ClosedForm as CF, Iterative as IT};
use super::super::{cov, Coverage};

const TWO_BY_TWO_1: &[&str] = &[
    "calc/two_by_two/large_counts",
    "calc/two_by_two/one_inside_boundary",
    "calc/two_by_two/stratified",
    "calc/two_by_two/support_max",
    "calc/two_by_two/zero_event",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov(
        "calc.two_by_two.adjusted.breslow_day.or_chisq",
        TWO_BY_TWO_1,
        CF,
    ),
    cov(
        "calc.two_by_two.adjusted.breslow_day.or_p",
        TWO_BY_TWO_1,
        CF,
    ),
    cov(
        "calc.two_by_two.adjusted.breslow_day.rd_chisq",
        TWO_BY_TWO_1,
        CF,
    ),
    cov(
        "calc.two_by_two.adjusted.breslow_day.rd_p",
        TWO_BY_TWO_1,
        CF,
    ),
    cov(
        "calc.two_by_two.adjusted.breslow_day.rr_chisq",
        TWO_BY_TWO_1,
        CF,
    ),
    cov(
        "calc.two_by_two.adjusted.breslow_day.rr_p",
        TWO_BY_TWO_1,
        CF,
    ),
    cov("calc.two_by_two.adjusted.cmle_or.est", TWO_BY_TWO_1, IT),
    cov(
        "calc.two_by_two.adjusted.cmle_or.fisher.lower",
        TWO_BY_TWO_1,
        IT,
    ),
    cov(
        "calc.two_by_two.adjusted.cmle_or.fisher.upper",
        TWO_BY_TWO_1,
        IT,
    ),
    cov(
        "calc.two_by_two.adjusted.cmle_or.midp.lower",
        TWO_BY_TWO_1,
        IT,
    ),
    cov(
        "calc.two_by_two.adjusted.cmle_or.midp.upper",
        TWO_BY_TWO_1,
        IT,
    ),
    cov("calc.two_by_two.adjusted.direct_or.est", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.direct_or.lower", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.direct_or.upper", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.direct_rd.est", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.direct_rd.lower", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.direct_rd.upper", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.direct_rr.est", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.direct_rr.lower", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.direct_rr.upper", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.mh_chisq.p", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.mh_chisq.value", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.mh_or.est", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.mh_or.lower", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.mh_or.upper", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.mh_rr.est", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.mh_rr.lower", TWO_BY_TWO_1, CF),
    cov("calc.two_by_two.adjusted.mh_rr.upper", TWO_BY_TWO_1, CF),
];
