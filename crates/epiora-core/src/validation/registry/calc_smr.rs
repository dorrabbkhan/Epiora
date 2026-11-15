//! `calc` registry coverage for `smr` (v1.0 Task 7): every
//! leaf the calculator emits, mapped to the Task 2 fixtures that
//! cover it. Rows name only the fixtures that actually contain the
//! leaf.

use super::super::ToleranceClass::{ClosedForm as CF, Iterative as IT};
use super::super::{cov, Coverage};

const SMR_0: &[&str] = &[
    "calc/smr/boundary_low",
    "calc/smr/conf_97",
    "calc/smr/large",
    "calc/smr/textbook",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.smr.ci.byar.lower", SMR_0, CF),
    cov("calc.smr.ci.byar.upper", SMR_0, CF),
    cov("calc.smr.ci.fisher.lower", SMR_0, IT),
    cov("calc.smr.ci.fisher.upper", SMR_0, IT),
    cov("calc.smr.ci.midp.lower", SMR_0, IT),
    cov("calc.smr.ci.midp.upper", SMR_0, IT),
    cov("calc.smr.ci.rothman_greenland.lower", SMR_0, CF),
    cov("calc.smr.ci.rothman_greenland.upper", SMR_0, CF),
    cov("calc.smr.ci.ury_wiggins.lower", SMR_0, CF),
    cov("calc.smr.ci.ury_wiggins.upper", SMR_0, CF),
    cov("calc.smr.ci.vandenbroucke.lower", SMR_0, CF),
    cov("calc.smr.ci.vandenbroucke.upper", SMR_0, CF),
    cov("calc.smr.estimate", SMR_0, CF),
    cov("calc.smr.test.byar_chisq", SMR_0, CF),
    cov("calc.smr.test.byar_p", SMR_0, CF),
    cov("calc.smr.test.chisq_p", SMR_0, CF),
    cov("calc.smr.test.chisq_value", SMR_0, CF),
    cov("calc.smr.test.fisher_p", SMR_0, IT),
    cov("calc.smr.test.midp_p", SMR_0, IT),
];
