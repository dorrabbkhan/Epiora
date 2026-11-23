//! `epiora-legacy`: Epi Info 7 / 3.5.x project readers (v1.1 spec §4).
//! Task 5 shipped the EPTB batch decoder; Task 7 adds the
//! [`LegacySource`] abstraction, the `BatchSource` the worker feeds,
//! the EI7 mapper ([`detect`]/[`map_ei7`]) and the re-import
//! [`refresh_plan`]. No input may panic; every failure is a
//! [`LegacyError`] with a stable `code()`.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod batch;
pub mod ei7;
mod error;
mod project;
mod refresh;
mod report;
mod source;

pub use ei7::{detect, map_ei7};
pub use error::LegacyError;
pub use project::{CheckCode, LegacyProgram, LegacyProject, LegacyView, SourceFormat};
pub use refresh::{refresh_plan, RefreshOutcome, RefreshPlan, ViewRefresh};
pub use report::{ReportEntry, ReportKind};
pub use source::{BatchSource, LegacySource};

#[cfg(feature = "fuzzing")]
pub use ei7::fuzz::legacy_meta_any;
