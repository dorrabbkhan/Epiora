//! `calc` registry coverage for the sample-size calculators
//! `ss_proportion`, `ss_unmatched_cc`, `ss_cohort` and
//! `ss_mean_difference` (v1.0 Task 9): every leaf each emits —
//! rounded and `_real` — mapped to the Task 2 (plus C1 follow-up)
//! fixtures that cover it.

use super::super::ToleranceClass::ClosedForm as CF;
use super::super::{cov, Coverage};

const SS_PROPORTION: &[&str] = &[
    "calc/ss_proportion/clustered",
    "calc/ss_proportion/large",
    "calc/ss_proportion/small_population",
    "calc/ss_proportion/textbook",
];

const SS_UNMATCHED_CC: &[&str] = &[
    "calc/ss_unmatched_cc/odds_ratio_input",
    "calc/ss_unmatched_cc/percent_input",
    "calc/ss_unmatched_cc/power_90",
];

const SS_COHORT: &[&str] = &[
    "calc/ss_cohort/odds_ratio_input",
    "calc/ss_cohort/percent_input",
    "calc/ss_cohort/risk_difference_input",
    "calc/ss_cohort/risk_ratio_input",
];

const SS_MEAN_DIFFERENCE: &[&str] = &[
    "calc/ss_mean_difference/high_power",
    "calc/ss_mean_difference/small_effect",
    "calc/ss_mean_difference/textbook",
];

pub(super) static ENTRIES: &[Coverage] = &[
    cov("calc.ss_proportion.cluster_size.80", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size.90", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size.95", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size.97", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size.99", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size.999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size.9999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size_real.80", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size_real.90", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size_real.95", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size_real.97", SS_PROPORTION, CF),
    cov("calc.ss_proportion.cluster_size_real.99", SS_PROPORTION, CF),
    cov(
        "calc.ss_proportion.cluster_size_real.999",
        SS_PROPORTION,
        CF,
    ),
    cov(
        "calc.ss_proportion.cluster_size_real.9999",
        SS_PROPORTION,
        CF,
    ),
    cov("calc.ss_proportion.n_ei7.80", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7.90", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7.95", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7.97", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7.99", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7.999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7.9999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7_real.80", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7_real.90", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7_real.95", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7_real.97", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7_real.99", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7_real.999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_ei7_real.9999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi.80", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi.90", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi.95", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi.97", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi.99", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi.999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi.9999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi_real.80", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi_real.90", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi_real.95", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi_real.97", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi_real.99", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi_real.999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.n_openepi_real.9999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.total.80", SS_PROPORTION, CF),
    cov("calc.ss_proportion.total.90", SS_PROPORTION, CF),
    cov("calc.ss_proportion.total.95", SS_PROPORTION, CF),
    cov("calc.ss_proportion.total.97", SS_PROPORTION, CF),
    cov("calc.ss_proportion.total.99", SS_PROPORTION, CF),
    cov("calc.ss_proportion.total.999", SS_PROPORTION, CF),
    cov("calc.ss_proportion.total.9999", SS_PROPORTION, CF),
    cov("calc.ss_unmatched_cc.fleiss.cases", SS_UNMATCHED_CC, CF),
    cov(
        "calc.ss_unmatched_cc.fleiss.cases_real",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov("calc.ss_unmatched_cc.fleiss.controls", SS_UNMATCHED_CC, CF),
    cov(
        "calc.ss_unmatched_cc.fleiss.controls_real",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov("calc.ss_unmatched_cc.fleiss.total", SS_UNMATCHED_CC, CF),
    cov(
        "calc.ss_unmatched_cc.fleiss.total_real",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov("calc.ss_unmatched_cc.fleiss_cc.cases", SS_UNMATCHED_CC, CF),
    cov(
        "calc.ss_unmatched_cc.fleiss_cc.cases_real",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov(
        "calc.ss_unmatched_cc.fleiss_cc.controls",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov(
        "calc.ss_unmatched_cc.fleiss_cc.controls_real",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov("calc.ss_unmatched_cc.fleiss_cc.total", SS_UNMATCHED_CC, CF),
    cov(
        "calc.ss_unmatched_cc.fleiss_cc.total_real",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov("calc.ss_unmatched_cc.kelsey.cases", SS_UNMATCHED_CC, CF),
    cov(
        "calc.ss_unmatched_cc.kelsey.cases_real",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov("calc.ss_unmatched_cc.kelsey.controls", SS_UNMATCHED_CC, CF),
    cov(
        "calc.ss_unmatched_cc.kelsey.controls_real",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov("calc.ss_unmatched_cc.kelsey.total", SS_UNMATCHED_CC, CF),
    cov(
        "calc.ss_unmatched_cc.kelsey.total_real",
        SS_UNMATCHED_CC,
        CF,
    ),
    cov("calc.ss_cohort.fleiss.exposed", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss.exposed_real", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss.total", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss.total_real", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss.unexposed", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss.unexposed_real", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss_cc.exposed", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss_cc.exposed_real", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss_cc.total", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss_cc.total_real", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss_cc.unexposed", SS_COHORT, CF),
    cov("calc.ss_cohort.fleiss_cc.unexposed_real", SS_COHORT, CF),
    cov("calc.ss_cohort.kelsey.exposed", SS_COHORT, CF),
    cov("calc.ss_cohort.kelsey.exposed_real", SS_COHORT, CF),
    cov("calc.ss_cohort.kelsey.total", SS_COHORT, CF),
    cov("calc.ss_cohort.kelsey.total_real", SS_COHORT, CF),
    cov("calc.ss_cohort.kelsey.unexposed", SS_COHORT, CF),
    cov("calc.ss_cohort.kelsey.unexposed_real", SS_COHORT, CF),
    cov("calc.ss_mean_difference.n1", SS_MEAN_DIFFERENCE, CF),
    cov("calc.ss_mean_difference.n1_real", SS_MEAN_DIFFERENCE, CF),
    cov("calc.ss_mean_difference.n2", SS_MEAN_DIFFERENCE, CF),
    cov("calc.ss_mean_difference.n2_real", SS_MEAN_DIFFERENCE, CF),
    cov("calc.ss_mean_difference.total", SS_MEAN_DIFFERENCE, CF),
    cov("calc.ss_mean_difference.total_real", SS_MEAN_DIFFERENCE, CF),
];
