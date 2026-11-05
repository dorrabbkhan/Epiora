//! epiora-core: the Epiora statistics engine (spec v0.2 §3).
//!
//! Deterministic (libm only, no clock or environment reads), never panics:
//! every failure is an [`EngineError`] with a stable `code()`, and every
//! statistically undefined value is `Estimate::Undefined { reason }`.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod dataset;
pub mod error;
pub mod procedures;
pub mod project;
pub mod receipt;
pub mod special;
pub mod stats;
pub mod tabulate;
pub mod validation;

#[cfg(feature = "corpus")]
pub mod corpus;

#[cfg(feature = "fuzzing")]
pub mod fuzz_harness;

pub use error::EngineError;
