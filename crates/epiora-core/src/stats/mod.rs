//! `stats` — pure statistics on counts (spec §3.5). `two_by_two` and
//! `exact` cover the 2×2 family including the iterative exact
//! intervals; `rxc` is the Pearson chi-square; `proportion` is the FREQ
//! per-category summary. Every output is an [`Estimate`]: statistically
//! undefined values are `Undefined { reason }`, never NaN.

mod exact;
mod rxc;
mod two_by_two;

pub mod estimate;
pub mod proportion;

pub use estimate::{Ci, Estimate, UndefinedReason};
pub use proportion::{proportions, CategoryStats};
pub use rxc::{rxc, RxcStats};
pub use two_by_two::{two_by_two, Table2x2, TwoByTwoStats};
