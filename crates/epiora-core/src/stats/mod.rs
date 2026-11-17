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
mod two_by_two_parts;

pub mod estimate;
pub mod proportion;

pub(crate) use breslow_day::bdt_parts;
pub use breslow_day::BreslowDayTarone;
pub use estimate::{Ci, Estimate, UndefinedReason};
pub use exact::fisher_p_two_sided;
pub(crate) use means::{
    anova_from_summary, bartlett_from_summary, group_stats, summarize, t_tests_from_summary,
    t_tests_from_summary_at, GroupSummary,
};
pub use means::{Anova, Bartlett, GroupStats, TTest, TwoGroupTests};
pub(crate) use means_summary::{anova_parts, bartlett_parts, t_parts, TParts};
pub use proportion::{proportions, CategoryStats};
pub use rank::KruskalWallis;
pub(crate) use rank::{kruskal_wallis, kw_parts};
pub(crate) use rates::{
    adj_stratum, adjusted_rate_parts, adjusted_rates, ef_person_time, ef_pt_parts, pois_approx,
    pt_zscore, rate_ratio, rate_ratio_parts, rd_byar, rd_byar_parts, rg_ci,
};
pub(crate) use rxc::rxc_parts;
pub use rxc::{rxc, RxcStats};
pub use stratified::{stratified, MantelHaenszel, MhChiSquare, MhEstimate, StratifiedStats};
pub(crate) use stratified::{stratified_derived, stratified_parts};
pub use summation::neumaier_sum;
pub(crate) use two_by_two::parts as two_by_two_parts;
pub use two_by_two::{two_by_two, ChiSquare, Table2x2, TwoByTwoStats};
