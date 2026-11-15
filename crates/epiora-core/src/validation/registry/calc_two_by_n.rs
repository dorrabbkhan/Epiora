//! `calc` registry coverage for `two_by_n` (v1.0 Task 7): every
//! leaf the calculator emits, mapped to the Task 2 fixtures that
//! cover it. Rows name only the fixtures that actually contain the
//! leaf.

use super::super::ToleranceClass::{ClosedForm as CF, Iterative as IT};
use super::super::{cov, Coverage};

const TWO_BY_N_0: &[&str] = &[
    "calc/two_by_n/stratified",
    "calc/two_by_n/textbook",
    "calc/two_by_n/zero_cell",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.two_by_n.crude.chisq.mantel.p_2t", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.chisq.mantel.value", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.chisq.uncorrected.p_2t", TWO_BY_N_0, CF),
    cov(
        "calc.two_by_n.crude.chisq.uncorrected.value",
        TWO_BY_N_0,
        CF,
    ),
    cov("calc.two_by_n.crude.chisq.yates.p_2t", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.chisq.yates.value", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.exact.fisher_1t", TWO_BY_N_0, IT),
    cov("calc.two_by_n.crude.exact.fisher_2t", TWO_BY_N_0, IT),
    cov("calc.two_by_n.crude.exact.midp_2t", TWO_BY_N_0, IT),
    cov("calc.two_by_n.crude.odds_ratio.est", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.odds_ratio.lower", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.odds_ratio.upper", TWO_BY_N_0, CF),
    cov(
        "calc.two_by_n.crude.odds_ratio_mle.fisher.lower",
        TWO_BY_N_0,
        IT,
    ),
    cov(
        "calc.two_by_n.crude.odds_ratio_mle.fisher.upper",
        TWO_BY_N_0,
        IT,
    ),
    cov(
        "calc.two_by_n.crude.odds_ratio_mle.midp.est",
        TWO_BY_N_0,
        IT,
    ),
    cov(
        "calc.two_by_n.crude.odds_ratio_mle.midp.lower",
        TWO_BY_N_0,
        IT,
    ),
    cov(
        "calc.two_by_n.crude.odds_ratio_mle.midp.upper",
        TWO_BY_N_0,
        IT,
    ),
    cov("calc.two_by_n.crude.risk_difference.est", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.risk_difference.lower", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.risk_difference.upper", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.risk_ratio.est", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.risk_ratio.lower", TWO_BY_N_0, CF),
    cov("calc.two_by_n.crude.risk_ratio.upper", TWO_BY_N_0, CF),
    cov("calc.two_by_n.strata[].chisq.mantel.p_2t", TWO_BY_N_0, CF),
    cov("calc.two_by_n.strata[].chisq.mantel.value", TWO_BY_N_0, CF),
    cov(
        "calc.two_by_n.strata[].chisq.uncorrected.p_2t",
        TWO_BY_N_0,
        CF,
    ),
    cov(
        "calc.two_by_n.strata[].chisq.uncorrected.value",
        TWO_BY_N_0,
        CF,
    ),
    cov("calc.two_by_n.strata[].chisq.yates.p_2t", TWO_BY_N_0, CF),
    cov("calc.two_by_n.strata[].chisq.yates.value", TWO_BY_N_0, CF),
    cov("calc.two_by_n.strata[].exact.fisher_1t", TWO_BY_N_0, IT),
    cov("calc.two_by_n.strata[].exact.fisher_2t", TWO_BY_N_0, IT),
    cov("calc.two_by_n.strata[].exact.midp_2t", TWO_BY_N_0, IT),
    cov("calc.two_by_n.strata[].odds_ratio.est", TWO_BY_N_0, CF),
    cov("calc.two_by_n.strata[].odds_ratio.lower", TWO_BY_N_0, CF),
    cov("calc.two_by_n.strata[].odds_ratio.upper", TWO_BY_N_0, CF),
    cov(
        "calc.two_by_n.strata[].odds_ratio_mle.fisher.lower",
        TWO_BY_N_0,
        IT,
    ),
    cov(
        "calc.two_by_n.strata[].odds_ratio_mle.fisher.upper",
        TWO_BY_N_0,
        IT,
    ),
    cov(
        "calc.two_by_n.strata[].odds_ratio_mle.midp.est",
        TWO_BY_N_0,
        IT,
    ),
    cov(
        "calc.two_by_n.strata[].odds_ratio_mle.midp.lower",
        TWO_BY_N_0,
        IT,
    ),
    cov(
        "calc.two_by_n.strata[].odds_ratio_mle.midp.upper",
        TWO_BY_N_0,
        IT,
    ),
    cov("calc.two_by_n.strata[].risk_difference.est", TWO_BY_N_0, CF),
    cov(
        "calc.two_by_n.strata[].risk_difference.lower",
        TWO_BY_N_0,
        CF,
    ),
    cov(
        "calc.two_by_n.strata[].risk_difference.upper",
        TWO_BY_N_0,
        CF,
    ),
    cov("calc.two_by_n.strata[].risk_ratio.est", TWO_BY_N_0, CF),
    cov("calc.two_by_n.strata[].risk_ratio.lower", TWO_BY_N_0, CF),
    cov("calc.two_by_n.strata[].risk_ratio.upper", TWO_BY_N_0, CF),
];
