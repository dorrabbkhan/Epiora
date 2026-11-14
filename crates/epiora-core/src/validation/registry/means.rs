//! `means` registry entries: the corpus fixtures covering every
//! statistic the `means` family can emit (spec §4.3). All means
//! leaves are `closed_form` — the generator assigns no `iterative`
//! leaf.
//!
//! Fixture sets:
//! - `ALL`: every means fixture (top-level `total`/`missing` and the
//!   `overall` block).
//! - `GROUPED`: every fixture with ≥ 1 non-empty group (everything
//!   but `means_all_missing`, whose `groups` is empty).
//! - `TWO_GROUP`: the fixtures whose run emits `t_test` — exactly two
//!   non-empty groups, both variance-defined, not essentially
//!   constant (`means_ties`, `means_dates`, `means_oswego_age_sex`;
//!   `means_constant` is excluded — its t-test is absent by R's
//!   "essentially constant" error).
//! - `ANOVA`/`KRUSKAL` (one set): every grouped fixture with ≥ 2
//!   groups (everything but the ungrouped and all-missing fixtures).
//! - `BARTLETT`: the same minus `means_one_obs_group` — R's
//!   `bartlett.test` errors when a group has one observation, so the
//!   block is absent there.

use crate::validation::{cov, Coverage, ToleranceClass};

const ALL: &[&str] = &[
    "means/means_all_missing",
    "means/means_constant",
    "means/means_dates",
    "means/means_grouped",
    "means/means_include_missing",
    "means/means_one_obs_group",
    "means/means_oswego_age",
    "means/means_oswego_age_sex",
    "means/means_ties",
    "means/means_ungrouped",
];

const GROUPED: &[&str] = &[
    "means/means_constant",
    "means/means_dates",
    "means/means_grouped",
    "means/means_include_missing",
    "means/means_one_obs_group",
    "means/means_oswego_age",
    "means/means_oswego_age_sex",
    "means/means_ties",
    "means/means_ungrouped",
];

const TWO_GROUP: &[&str] = &[
    "means/means_dates",
    "means/means_oswego_age_sex",
    "means/means_ties",
];

const ANOVA: &[&str] = &[
    "means/means_constant",
    "means/means_dates",
    "means/means_grouped",
    "means/means_include_missing",
    "means/means_one_obs_group",
    "means/means_oswego_age_sex",
    "means/means_ties",
];

const BARTLETT: &[&str] = &[
    "means/means_constant",
    "means/means_dates",
    "means/means_grouped",
    "means/means_include_missing",
    "means/means_oswego_age_sex",
    "means/means_ties",
];

macro_rules! stat_entries {
    ($($id:literal => $fixtures:expr),+ $(,)?) => {
        &[$(cov($id, $fixtures, ToleranceClass::ClosedForm)),+]
    };
}

pub(super) static ENTRIES: &[Coverage] = stat_entries! {
    "means.total" => ALL,
    "means.missing" => ALL,
    "means.groups[].observations" => GROUPED,
    "means.groups[].total" => GROUPED,
    "means.groups[].mean" => GROUPED,
    "means.groups[].variance" => GROUPED,
    "means.groups[].std_dev" => GROUPED,
    "means.groups[].min" => GROUPED,
    "means.groups[].p25" => GROUPED,
    "means.groups[].median" => GROUPED,
    "means.groups[].p75" => GROUPED,
    "means.groups[].max" => GROUPED,
    "means.groups[].mode" => GROUPED,
    "means.overall.observations" => ALL,
    "means.overall.total" => ALL,
    "means.overall.mean" => ALL,
    "means.overall.variance" => ALL,
    "means.overall.std_dev" => ALL,
    "means.overall.min" => ALL,
    "means.overall.p25" => ALL,
    "means.overall.median" => ALL,
    "means.overall.p75" => ALL,
    "means.overall.max" => ALL,
    "means.overall.mode" => ALL,
    "means.t_test.pooled.t" => TWO_GROUP,
    "means.t_test.pooled.df" => TWO_GROUP,
    "means.t_test.pooled.p_value" => TWO_GROUP,
    "means.t_test.pooled.diff" => TWO_GROUP,
    "means.t_test.pooled.diff_ci_95.lower" => TWO_GROUP,
    "means.t_test.pooled.diff_ci_95.upper" => TWO_GROUP,
    "means.t_test.welch.t" => TWO_GROUP,
    "means.t_test.welch.df" => TWO_GROUP,
    "means.t_test.welch.p_value" => TWO_GROUP,
    "means.t_test.welch.diff" => TWO_GROUP,
    "means.t_test.welch.diff_ci_95.lower" => TWO_GROUP,
    "means.t_test.welch.diff_ci_95.upper" => TWO_GROUP,
    "means.anova.ss_between" => ANOVA,
    "means.anova.ss_within" => ANOVA,
    "means.anova.ss_total" => ANOVA,
    "means.anova.df_between" => ANOVA,
    "means.anova.df_within" => ANOVA,
    "means.anova.df_total" => ANOVA,
    "means.anova.ms_between" => ANOVA,
    "means.anova.ms_within" => ANOVA,
    "means.anova.f" => ANOVA,
    "means.anova.p_value" => ANOVA,
    "means.bartlett.statistic" => BARTLETT,
    "means.bartlett.df" => BARTLETT,
    "means.bartlett.p_value" => BARTLETT,
    "means.kruskal_wallis.h" => ANOVA,
    "means.kruskal_wallis.df" => ANOVA,
    "means.kruskal_wallis.p_value" => ANOVA,
};
