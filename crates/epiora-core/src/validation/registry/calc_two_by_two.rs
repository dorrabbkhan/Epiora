//! `calc` registry coverage for `two_by_two` `crude` output
//! (v1.0 Task 7): every leaf the calculator emits, mapped to the
//! Task 2 fixtures that cover it.

use super::super::ToleranceClass::{ClosedForm as CF, Iterative as IT};
use super::super::{cov, Coverage};

const TWO_BY_TWO_0: &[&str] = &[
    "calc/two_by_two/case_control",
    "calc/two_by_two/large",
    "calc/two_by_two/large_counts",
    "calc/two_by_two/one_inside_boundary",
    "calc/two_by_two/stratified",
    "calc/two_by_two/support_max",
    "calc/two_by_two/textbook",
    "calc/two_by_two/zero_cell",
    "calc/two_by_two/zero_event",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov(
        "calc.two_by_two.crude.chisq.mantel_haenszel.p_1t",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.chisq.mantel_haenszel.p_2t",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.chisq.mantel_haenszel.value",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.chisq.uncorrected.p_1t",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.chisq.uncorrected.p_2t",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.chisq.uncorrected.value",
        TWO_BY_TWO_0,
        CF,
    ),
    cov("calc.two_by_two.crude.chisq.yates.p_1t", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.chisq.yates.p_2t", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.chisq.yates.value", TWO_BY_TWO_0, CF),
    cov(
        "calc.two_by_two.crude.cmle_or.fisher.lower",
        TWO_BY_TWO_0,
        IT,
    ),
    cov(
        "calc.two_by_two.crude.cmle_or.fisher.upper",
        TWO_BY_TWO_0,
        IT,
    ),
    cov("calc.two_by_two.crude.cmle_or.midp.est", TWO_BY_TWO_0, IT),
    cov("calc.two_by_two.crude.cmle_or.midp.lower", TWO_BY_TWO_0, IT),
    cov("calc.two_by_two.crude.cmle_or.midp.upper", TWO_BY_TWO_0, IT),
    cov("calc.two_by_two.crude.etiologic.efe.est", TWO_BY_TWO_0, CF),
    cov(
        "calc.two_by_two.crude.etiologic.efe.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic.efe.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov("calc.two_by_two.crude.etiologic.efp.est", TWO_BY_TWO_0, CF),
    cov(
        "calc.two_by_two.crude.etiologic.efp.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic.efp.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov("calc.two_by_two.crude.etiologic.pfe.est", TWO_BY_TWO_0, CF),
    cov(
        "calc.two_by_two.crude.etiologic.pfe.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic.pfe.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov("calc.two_by_two.crude.etiologic.pfp.est", TWO_BY_TWO_0, CF),
    cov(
        "calc.two_by_two.crude.etiologic.pfp.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic.pfp.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.efe_or.est",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.efe_or.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.efe_or.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.efp_or.est",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.efp_or.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.efp_or.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.pfe_or.est",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.pfe_or.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.pfe_or.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.pfp_or.est",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.pfp_or.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.etiologic_or.pfp_or.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov("calc.two_by_two.crude.fisher_exact.p_1t", TWO_BY_TWO_0, IT),
    cov("calc.two_by_two.crude.fisher_exact.p_2t", TWO_BY_TWO_0, IT),
    cov("calc.two_by_two.crude.midp_exact.p_1t", TWO_BY_TWO_0, IT),
    cov("calc.two_by_two.crude.midp_exact.p_2t", TWO_BY_TWO_0, IT),
    cov("calc.two_by_two.crude.odds_ratio.est", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.odds_ratio.lower", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.odds_ratio.upper", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.risk.exposed.est", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.risk.exposed.lower", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.risk.exposed.upper", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.risk.overall.est", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.risk.overall.lower", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.risk.overall.upper", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.risk.unexposed.est", TWO_BY_TWO_0, CF),
    cov(
        "calc.two_by_two.crude.risk.unexposed.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.risk.unexposed.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.risk_difference.est",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.risk_difference.lower",
        TWO_BY_TWO_0,
        CF,
    ),
    cov(
        "calc.two_by_two.crude.risk_difference.upper",
        TWO_BY_TWO_0,
        CF,
    ),
    cov("calc.two_by_two.crude.risk_ratio.est", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.risk_ratio.lower", TWO_BY_TWO_0, CF),
    cov("calc.two_by_two.crude.risk_ratio.upper", TWO_BY_TWO_0, CF),
];
