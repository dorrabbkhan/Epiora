//! The single error type of the legacy readers (v1.1 spec §10). No
//! input can cause a panic; every failure is a `LegacyError` whose
//! `code()` is stable — callers match on `code()`, never on the message.

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
}

impl LegacyError {
    /// The stable machine-readable code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Self::BatchInvalid { .. } => "legacy_batch_invalid",
        }
    }
}
