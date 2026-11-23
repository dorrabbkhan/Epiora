//! The mapped legacy project — the shape Task 11/12's store persists
//! and the golden fixtures pin structurally (spec §4.2, §7.2).

use epiora_core::dataset::{Dataset, DatasetLink};

use crate::report::ReportEntry;

/// Which legacy backend the project came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceFormat {
    /// Epi Info 7 project in an Access `.mdb`/`.accdb`.
    Ei7Access,
    /// Epi Info 7 project in a SQLite `.db`.
    Ei7Sqlite,
    /// Epi Info 7 project in a SQL Server export (Task 13+).
    Ei7SqlServer,
    /// Epi Info 3.5.x project (Task 8).
    Ei3Access,
}

/// A saved EI7 `metaPrograms` program: name + verbatim text. The store
/// keeps the text so Program→Command translation (v1.3) has a lossless
/// corpus (spec §5.1); `text_sha256` is what the golden pins.
#[derive(Clone, Debug)]
pub struct LegacyProgram {
    /// The program name.
    pub name: String,
    /// The program text, verbatim.
    pub text: String,
}

/// Stored check code for one view: `blocks` counts `Form`/`View`/
/// `Record`/`Page`/`Field`/`DefineVariables` block headers over the
/// concatenated blocks (golden `check_code` entries); `text` holds the
/// block texts joined by a blank line so v1.3's translator can parse
/// it (spec §5.2).
#[derive(Clone, Debug)]
pub struct CheckCode {
    /// The view the check code belongs to.
    pub view: String,
    /// The concatenated block texts, in view→page→field order.
    pub text: String,
    /// The block-header count (golden parity).
    pub blocks: u32,
}

/// One imported view.
#[derive(Clone, Debug)]
pub struct LegacyView {
    /// The view name.
    pub name: String,
    /// The imported dataset (deleted records kept; `deleted_records`
    /// counts them, spec §7.2).
    pub dataset: Dataset,
    /// The record-key column the re-import diff uses — `GlobalRecordId`
    /// when present, else `UniqueKey`.
    pub key_column: String,
    /// Rows whose `RecStatus` marks them deleted in the source.
    pub deleted_records: u64,
}

/// A whole mapped legacy project.
#[derive(Clone, Debug)]
pub struct LegacyProject {
    /// `metaDbInfo.ProjectName`, or empty when absent (the caller knows
    /// the file name for the fallback the golden applies).
    pub name: String,
    /// `metaDbInfo.Identify` — the legacy project GUID, else `""`.
    pub legacy_id: String,
    /// Which backend produced this project.
    pub format: SourceFormat,
    /// View datasets, in `metaViews` order.
    pub views: Vec<LegacyView>,
    /// Grid (and relate) child datasets reached through `links`:
    /// datasets that are not views — the golden files carry them under
    /// `links[].dataset`.
    pub datasets: Vec<Dataset>,
    /// Project links — relates and grids.
    pub links: Vec<DatasetLink>,
    /// Saved programs (EI7 `metaPrograms`).
    pub programs: Vec<LegacyProgram>,
    /// Stored check code, one entry per view that has any.
    pub check_code: Vec<CheckCode>,
    /// Everything skipped, degraded or assumed, in emit order.
    pub report: Vec<ReportEntry>,
}
