//! `stats` — pure statistics on counts (spec §3.5). `two_by_two` and
//! `exact` cover the 2×2 family including the iterative exact
//! intervals; `stratified` is the Mantel–Haenszel and Breslow–Day–
//! Tarone family over k 2×2 strata; `rxc` is the Pearson chi-square;
//! `proportion` is the FREQ per-category summary. Every output is an
//! [`Estimate`]: statistically undefined values are
//! `Undefined { reason }`, never NaN.

mod breslow_day;
mod exact;
mod means;
mod means_summary;
mod rank;
mod rates;
mod rxc;
mod stratified;
mod summation;
mod two_by_two;

pub mod estimate;
pub mod proportion;

pub use breslow_day::BreslowDayTarone;
pub use estimate::{Ci, Estimate, UndefinedReason};
pub use exact::fisher_p_two_sided;
pub(crate) use means::{
    anova_from_summary, bartlett_from_summary, group_stats, summarize, t_tests_from_summary,
    t_tests_from_summary_at, GroupSummary,
};
pub use means::{Anova, Bartlett, GroupStats, TTest, TwoGroupTests};
pub use proportion::{proportions, CategoryStats};
pub(crate) use rank::kruskal_wallis;
pub use rank::KruskalWallis;
pub(crate) use rates::{
    adjusted_rates, ef_person_time, pois_approx, pt_zscore, rate_ratio, rd_byar, rg_ci,
};
pub use rxc::{rxc, RxcStats};
pub use stratified::{stratified, MantelHaenszel, MhChiSquare, MhEstimate, StratifiedStats};
pub use summation::neumaier_sum;
pub use two_by_two::{two_by_two, ChiSquare, Table2x2, TwoByTwoStats};
