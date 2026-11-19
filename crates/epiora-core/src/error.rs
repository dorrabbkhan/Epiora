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
    /// A MEANS variable is not Integer, Real or Date.
    #[error("not a numeric variable: {name}")]
    NotNumericVariable { name: String },
    /// A MEANS cross-tabulation has more than `levels` categories.
    #[error("too many groups for {name}: {levels}")]
    TooManyGroups { name: String, levels: u64 },
    /// A fixture does not follow the corpus format (spec §6.2).
    #[error("corpus format: {reason}")]
    CorpusFormat { reason: String },
    /// Snapshot bytes are malformed or non-canonical (v0.3 spec §5.1).
    #[error("snapshot decode: {reason}")]
    SnapshotDecode { reason: String },
    /// A calculator id is not in the calculator registry (v1.0 spec
    /// §5.3).
    #[error("unknown calculator: {id}")]
    CalcUnknown { id: String },
    /// A required calculator input is missing (v1.0 spec §5.3).
    #[error("calculator input missing: {input}")]
    CalcInputMissing { input: String },
    /// A calculator input fails its declared kind, range or table
    /// shape — `reason` is the stable sub-code (`unknown_input`,
    /// `not_an_integer`, `below_min`, `above_max`, `bad_choice`,
    /// `bad_table_shape`, `bad_table_cell`, `not_finite`,
    /// `bad_conf_level`; v1.0 spec §5.3).
    #[error("calculator input out of range: {input} ({reason})")]
    CalcInputOutOfRange { input: String, reason: String },
    /// A statistic path no procedure or calculator run can produce
    /// (v1.0 spec §6): an unknown family, an absent leaf, or a path
    /// that is not a statistic.
    #[error("unknown statistic for explain: {statistic}")]
    ExplainUnknown { statistic: String },
    /// This run has no reproducible R reference (v1.0 spec §8.1) —
    /// currently only `random_numbers`, whose engine draws come from
    /// a seeded ChaCha stream R cannot replay.
    #[error("R export not available: {reason}")]
    ExportNotAvailable { reason: String },
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
            Self::NotNumericVariable { .. } => "not_numeric_variable",
            Self::TooManyGroups { .. } => "too_many_groups",
            Self::CorpusFormat { .. } => "corpus_format",
            Self::SnapshotDecode { .. } => "snapshot_decode",
            Self::CalcUnknown { .. } => "calc_unknown",
            Self::CalcInputMissing { .. } => "calc_input_missing",
            Self::CalcInputOutOfRange { .. } => "calc_input_out_of_range",
            Self::ExplainUnknown { .. } => "explain_unknown",
            Self::ExportNotAvailable { .. } => "export_not_available",
        }
    }
}
