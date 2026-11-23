//! `epiora-legacy`: Epi Info 7 / 3.5.x project readers (v1.1 spec §4).
//! Task 5 shipped the EPTB batch decoder; Task 7 adds the
//! [`LegacySource`] abstraction, the `BatchSource` the worker feeds,
//! the EI7 mapper ([`detect`]/[`map_ei7`]) and the re-import
//! [`refresh_plan`]; Task 8 adds the EI3 mapper ([`map_ei3`]). No
//! input may panic; every failure is a [`LegacyError`] with a stable
//! `code()`.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

pub mod batch;
pub mod ei3;
pub mod ei7;
mod error;
mod project;
mod refresh;
mod report;
mod source;

pub use ei3::map_ei3;
pub use ei7::map_ei7;
pub use error::LegacyError;
pub use project::{CheckCode, LegacyProgram, LegacyProject, LegacyView, SourceFormat};
pub use refresh::{refresh_plan, RefreshOutcome, RefreshPlan, ViewRefresh};
pub use report::{ReportEntry, ReportKind};
pub use source::{BatchSource, LegacySource};

#[cfg(feature = "fuzzing")]
pub use ei7::fuzz::legacy_meta_any;

/// `detect` — layout detection only: the `LegacySource` carries table
/// names, not the backend, so an EI7 layout reports `Ei7Access`; the
/// caller supplies the true backend `SourceFormat` to [`map_ei7`].
/// `view*` tables (and no `metaViews`) mean EI3; anything else is
/// [`LegacyError::LayoutUnknown`] with the plain table list (spec §10).
pub fn detect(source: &dyn LegacySource) -> Result<SourceFormat, LegacyError> {
    let tables = source.tables();
    if tables.iter().any(|t| t.eq_ignore_ascii_case("metaviews")) {
        return Ok(SourceFormat::Ei7Access);
    }
    if tables
        .iter()
        .any(|t| t.get(..4).is_some_and(|p| p.eq_ignore_ascii_case("view")))
    {
        return Ok(SourceFormat::Ei3Access);
    }
    Err(LegacyError::LayoutUnknown {
        tables: plain_tables(&tables),
    })
}

/// The non-metadata tables an unknown-layout error offers for plain
/// import (the golden's `unknownLayoutEntries` filter).
pub(crate) fn plain_tables(tables: &[String]) -> Vec<String> {
    tables
        .iter()
        .filter(|t| {
            let lt = t.to_lowercase();
            !lt.starts_with("meta") && !lt.starts_with("sqlite_")
        })
        .cloned()
        .collect()
}
