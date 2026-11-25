//! The command register (Task 1's `register.json`, embedded): every
//! command, expression function, field type and module feature with
//! its support status, `affects_data` flag and target version. It
//! drives the planner's `may_differ` marking, `Unsupported`
//! reporting and the migration report (spec §6.1).

use serde::Deserialize;
use std::sync::OnceLock;

static REGISTER_JSON: &str = include_str!("../../register.json");

static REGISTER: OnceLock<Register> = OnceLock::new();

/// The raw register JSON — `register()`'s parsed form, emitted
/// verbatim so callers can return it without reserializing.
pub fn register_json() -> &'static str {
    REGISTER_JSON
}

/// The embedded register — parsed once, shared for the process.
pub fn register() -> &'static Register {
    REGISTER.get_or_init(|| {
        // The file is embedded at build time; a parse failure is a
        // build-time defect, not a runtime input error.
        #[allow(clippy::expect_used)]
        serde_json::from_str(REGISTER_JSON).expect("register.json is valid JSON")
    })
}

/// The register: command, function, field-type and module tables.
#[derive(Debug, Deserialize)]
pub struct Register {
    /// Every command the grammar and the register additions name.
    pub commands: Vec<Entry>,
    /// Every expression function.
    pub functions: Vec<Entry>,
    /// Epi Info field types and their Epiora mapping.
    pub field_types: Vec<FieldType>,
    /// Feature modules and their target versions.
    pub modules: Vec<Module>,
}

/// One command or function row.
#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    /// `UPPER-CASE` name (two words where the command is two —
    /// `CANCEL SELECT`).
    pub name: String,
    /// Which Epi Info grammar defines it (`analysis`, `enter`,
    /// `both`).
    pub grammar: String,
    /// Support status.
    pub status: Status,
    /// Whether running it can change the working view's data — the
    /// `may_differ` driver.
    pub affects_data: bool,
    /// The version it lands at (`planned`), or `null` (`not_planned`).
    pub version: Option<String>,
    /// Supported option words / forms, as Task 1 recorded them.
    pub options: Vec<String>,
    /// Why this status.
    pub reason: String,
}

/// Support status.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// Shipped or shipping now.
    Supported,
    /// On the roadmap at `version`.
    Planned,
    /// Explicitly not planned.
    NotPlanned,
}

/// A field type row.
#[derive(Clone, Debug, Deserialize)]
pub struct FieldType {
    /// The Epi Info name.
    pub name: String,
    /// The Epiora storage type it maps to.
    pub epiora_type: String,
    /// Support status.
    pub status: Status,
    /// Target version.
    pub version: Option<String>,
}

/// A module feature row.
#[derive(Clone, Debug, Deserialize)]
pub struct Module {
    /// The module (Enter, MakeView, …).
    pub module: String,
    /// The feature.
    pub feature: String,
    /// Target version.
    pub version: Option<String>,
}

impl Register {
    /// The command row named `name` (case-insensitive), if any.
    pub fn command(&self, name: &str) -> Option<&Entry> {
        self.commands
            .iter()
            .find(|e| e.name.eq_ignore_ascii_case(name))
    }
}
