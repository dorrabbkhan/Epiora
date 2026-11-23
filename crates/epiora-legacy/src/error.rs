//! The single error type of the legacy readers (v1.1 spec §10). No
//! input can cause a panic; every failure surfaces as a `LegacyError`
//! whose `code()` is stable — callers match on `code()`, never on the
//! message.

/// A legacy-read failure with a stable machine-readable `code()`.
#[derive(Debug, thiserror::Error)]
pub enum LegacyError {
    /// An EPTB batch is malformed or truncated (spec §10:
    /// `legacy_batch_invalid`).
    #[error("invalid EPTB batch: {reason}")]
    BatchInvalid {
        /// What is wrong with the batch.
        reason: String,
    },
    /// The file has no recognized view-definition tables — not an Epi
    /// Info project (spec §10: `legacy_layout_unknown`). Carries the
    /// plain table list so the app can offer a plain table import.
    #[error("unrecognised project layout ({} table(s))", tables.len())]
    LayoutUnknown {
        /// Every table the source listed, in source order.
        tables: Vec<String>,
    },
    /// One view failed to map; the other views still import and the
    /// report says why (spec §10: `legacy_view_broken`).
    #[error("view {view} could not be mapped: {reason}")]
    ViewBroken {
        /// The view that failed.
        view: String,
        /// What went wrong.
        reason: String,
    },
    /// A failure the `LegacySource` implementation reported with its
    /// own stable code — `mdb_*` reader errors pass through unchanged
    /// (plan Task 7).
    #[error("{reason}")]
    Source {
        /// The reader's stable code (`mdb_truncated`, …).
        code: &'static str,
        /// What went wrong.
        reason: String,
    },
    /// The re-import diff rejected a key column or id set
    /// (`epiora-import`'s stable codes pass through).
    #[error("re-import diff failed: {0}")]
    Import(#[from] epiora_import::ImportError),
    /// The dataset model rejected mapped data (`epiora-core`'s stable
    /// codes pass through).
    #[error("dataset rejected: {0}")]
    Engine(#[from] epiora_core::EngineError),
}

impl LegacyError {
    /// The stable machine-readable code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Self::BatchInvalid { .. } => "legacy_batch_invalid",
            Self::LayoutUnknown { .. } => "legacy_layout_unknown",
            Self::ViewBroken { .. } => "legacy_view_broken",
            Self::Source { code, .. } => code,
            Self::Import(e) => e.code(),
            Self::Engine(e) => e.code(),
        }
    }
}
