//! `calc` registry coverage for `dose_response` (v1.0 Task 7): every
//! leaf the calculator emits, mapped to the Task 2 fixtures that
//! cover it. Rows name only the fixtures that actually contain the
//! leaf.

use super::super::ToleranceClass::ClosedForm as CF;
use super::super::{cov, Coverage};

const DOSE_RESPONSE_0: &[&str] = &[
    "calc/dose_response/decreasing",
    "calc/dose_response/large",
    "calc/dose_response/stratified",
    "calc/dose_response/textbook",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.dose_response.crude.odds_ratio[]", DOSE_RESPONSE_0, CF),
    cov("calc.dose_response.mh_or[]", DOSE_RESPONSE_0, CF),
    cov(
        "calc.dose_response.strata[].odds_ratio[]",
        DOSE_RESPONSE_0,
        CF,
    ),
    cov("calc.dose_response.xmh.cc.p", DOSE_RESPONSE_0, CF),
    cov("calc.dose_response.xmh.cc.value", DOSE_RESPONSE_0, CF),
    cov("calc.dose_response.xmh.uncorrected.p", DOSE_RESPONSE_0, CF),
    cov(
        "calc.dose_response.xmh.uncorrected.value",
        DOSE_RESPONSE_0,
        CF,
    ),
];
