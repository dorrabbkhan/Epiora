//! The single error type of the engine (spec §8). No input can cause a
//! panic; every failure is an `EngineError` whose `code()` is stable
//! across versions — callers match on `code()`, never on the message.

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum EngineError {
    /// A variable named in the options is not in the dataset.
    #[error("unknown variable: {name}")]
    UnknownVariable { name: String },
    /// A two-level variable without explicit levels whose orientation
    /// cannot be derived (spec §3.3): an error, never a guess.
    #[error("ambiguous levels for variable: {variable}")]
    AmbiguousLevels { variable: String },
    /// The dataset has no rows.
    #[error("empty dataset")]
    EmptyDataset,
    /// A column failed `Dataset::new` validation (spec §3.3).
    #[error("invalid column {column}: {reason}")]
    InvalidColumn { column: String, reason: String },
    /// A variable's type does not match what the option requires.
    #[error("type mismatch for {variable}: expected {expected}, found {found}")]
    TypeMismatch {
        variable: String,
        expected: String,
        found: String,
    },
    /// The options passed to a procedure are inconsistent.
    #[error("invalid options: {reason}")]
    InvalidOptions { reason: String },
    /// A fixture does not follow the corpus format (spec §6.2).
    #[error("corpus format: {reason}")]
    CorpusFormat { reason: String },
    /// Snapshot bytes are malformed or non-canonical (v0.3 spec §5.1).
    #[error("snapshot decode: {reason}")]
    SnapshotDecode { reason: String },
}

impl EngineError {
    /// The stable machine-readable code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Self::UnknownVariable { .. } => "unknown_variable",
            Self::AmbiguousLevels { .. } => "ambiguous_levels",
            Self::EmptyDataset => "empty_dataset",
            Self::InvalidColumn { .. } => "invalid_column",
            Self::TypeMismatch { .. } => "type_mismatch",
            Self::InvalidOptions { .. } => "invalid_options",
            Self::CorpusFormat { .. } => "corpus_format",
            Self::SnapshotDecode { .. } => "snapshot_decode",
        }
    }
}
