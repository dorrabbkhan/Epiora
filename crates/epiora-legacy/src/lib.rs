//! `epiora-legacy`: Epi Info 7 / 3.5.x project readers (v1.1 spec §4).
//! Task 5 ships the EPTB batch decoder; the mappers land in Tasks 7–8.

pub mod batch;
mod error;

pub use error::LegacyError;
