//! `stats` — pure statistics on counts (spec §3.5). `two_by_two` and
//! `exact` cover the 2×2 family including the iterative exact
//! intervals; `stratified` is the Mantel–Haenszel and Breslow–Day–
//! Tarone family over k 2×2 strata; `rxc` is the Pearson chi-square;
//! `proportion` is the FREQ per-category summary. Every output is an
//! [`Estimate`]: statistically undefined values are
//! `Undefined { reason }`, never NaN.

mod breslow_day;
mod exact;
mod rxc;
mod stratified;
mod two_by_two;

pub mod estimate;
pub mod proportion;

pub use breslow_day::BreslowDayTarone;
pub use estimate::{Ci, Estimate, UndefinedReason};
pub use proportion::{proportions, CategoryStats};
pub use rxc::{rxc, RxcStats};
pub use stratified::{stratified, MantelHaenszel, MhChiSquare, MhEstimate, StratifiedStats};
pub use two_by_two::{two_by_two, ChiSquare, Table2x2, TwoByTwoStats};
