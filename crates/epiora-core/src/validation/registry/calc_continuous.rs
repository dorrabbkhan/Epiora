//! `calc` registry coverage for the continuous-summary calculators
//! `mean_ci`, `median_ci`, `t_test_summary` and `anova_summary`
//! (v1.0 Task 8): every leaf each emits, mapped to the Task 2
//! fixtures that cover it.

use super::super::ToleranceClass::ClosedForm as CF;
use super::super::{cov, Coverage};

const MEAN_CI: &[&str] = &[
    "calc/mean_ci/se_input",
    "calc/mean_ci/textbook",
    "calc/mean_ci/variance_input",
];

const MEDIAN_CI: &[&str] = &[
    "calc/median_ci/boundary",
    "calc/median_ci/large",
    "calc/median_ci/median",
    "calc/median_ci/p25",
];

const T_TEST: &[&str] = &[
    "calc/t_test_summary/boundary",
    "calc/t_test_summary/equal_n",
    "calc/t_test_summary/large",
    "calc/t_test_summary/textbook",
];

const ANOVA: &[&str] = &[
    "calc/anova_summary/large",
    "calc/anova_summary/small_group",
    "calc/anova_summary/textbook",
    "calc/anova_summary/two_groups",
];

const MEAN: &[Coverage] = &[
    cov("calc.mean_ci.t.lower", MEAN_CI, CF),
    cov("calc.mean_ci.t.upper", MEAN_CI, CF),
    cov("calc.mean_ci.z.lower", MEAN_CI, CF),
    cov("calc.mean_ci.z.upper", MEAN_CI, CF),
];

const MEDIAN: &[Coverage] = &[
    cov("calc.median_ci.normal.lower_rank", MEDIAN_CI, CF),
    cov("calc.median_ci.normal.upper_rank", MEDIAN_CI, CF),
];

const T_TEST_ROWS: &[Coverage] = &[
    cov("calc.t_test_summary.f_variance.df_den", T_TEST, CF),
    cov("calc.t_test_summary.f_variance.df_num", T_TEST, CF),
    cov("calc.t_test_summary.f_variance.f", T_TEST, CF),
    cov("calc.t_test_summary.f_variance.p", T_TEST, CF),
    cov("calc.t_test_summary.pooled.df", T_TEST, CF),
    cov("calc.t_test_summary.pooled.diff", T_TEST, CF),
    cov("calc.t_test_summary.pooled.lower", T_TEST, CF),
    cov("calc.t_test_summary.pooled.p_2t", T_TEST, CF),
    cov("calc.t_test_summary.pooled.t", T_TEST, CF),
    cov("calc.t_test_summary.pooled.upper", T_TEST, CF),
    cov("calc.t_test_summary.welch.df", T_TEST, CF),
    cov("calc.t_test_summary.welch.diff", T_TEST, CF),
    cov("calc.t_test_summary.welch.lower", T_TEST, CF),
    cov("calc.t_test_summary.welch.p_2t", T_TEST, CF),
    cov("calc.t_test_summary.welch.upper", T_TEST, CF),
    cov("calc.t_test_summary.welch.t", T_TEST, CF),
];

const ANOVA_ROWS: &[Coverage] = &[
    cov("calc.anova_summary.anova.df_between", ANOVA, CF),
    cov("calc.anova_summary.anova.df_total", ANOVA, CF),
    cov("calc.anova_summary.anova.df_within", ANOVA, CF),
    cov("calc.anova_summary.anova.f", ANOVA, CF),
    cov("calc.anova_summary.anova.ms_between", ANOVA, CF),
    cov("calc.anova_summary.anova.ms_within", ANOVA, CF),
    cov("calc.anova_summary.anova.p", ANOVA, CF),
    cov("calc.anova_summary.anova.ss_between", ANOVA, CF),
    cov("calc.anova_summary.anova.ss_total", ANOVA, CF),
    cov("calc.anova_summary.anova.ss_within", ANOVA, CF),
    cov("calc.anova_summary.bartlett.chisq", ANOVA, CF),
    cov("calc.anova_summary.bartlett.df", ANOVA, CF),
    cov("calc.anova_summary.bartlett.p", ANOVA, CF),
    cov("calc.anova_summary.group_ci[].lower", ANOVA, CF),
    cov("calc.anova_summary.group_ci[].upper", ANOVA, CF),
    cov("calc.anova_summary.group_ci_pooled[].lower", ANOVA, CF),
    cov("calc.anova_summary.group_ci_pooled[].upper", ANOVA, CF),
];

const LEN: usize = MEAN.len() + MEDIAN.len() + T_TEST_ROWS.len() + ANOVA_ROWS.len();
const fn join<const N: usize>(parts: &[&'static [Coverage]]) -> [Coverage; N] {
    let mut out = [Coverage {
        statistic: "",
        fixtures: &[],
        class: CF,
    }; N];
    let (mut i, mut f) = (0, 0);
    while f < parts.len() {
        let mut j = 0;
        while j < parts[f].len() {
            out[i] = parts[f][j];
            i += 1;
            j += 1;
        }
        f += 1;
    }
    out
}

pub(super) static ENTRIES: &[Coverage] = &join::<LEN>(&[MEAN, MEDIAN, T_TEST_ROWS, ANOVA_ROWS]);
