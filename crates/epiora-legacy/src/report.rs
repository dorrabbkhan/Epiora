//! The structured migration report (spec §6.1/§7.2): everything the
//! mapper skipped, degraded or assumed, as ordered `(kind, item,
//! reason)` entries. `kind` uses the golden fixture vocabulary —
//! `fixtures/schema/legacy-project.schema.json` — with the plan's
//! spec-level kinds kept for report consumers.

/// One skipped/degraded/assumed thing the report records.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReportEntry {
    /// The category, from the golden vocabulary.
    pub kind: ReportKind,
    /// The logical name of what was affected — a project, table,
    /// `table/column` or `view/field` name, never a path.
    pub item: String,
    /// Why it was recorded.
    pub reason: String,
}

/// The report categories. `as_str` gives the golden `kind` spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportKind {
    /// A field the mapper did not import (`field_not_imported`).
    FieldNotImported,
    /// A field imported differently than declared
    /// (`field_imported_changed` — Mirror values, Codes display text).
    FieldChanged,
    /// A physical table no view, field or code table claims
    /// (`table_not_imported`).
    TableNotImported,
    /// A field's physical column is absent (`field_column_missing`).
    FieldColumnMissing,
    /// A view/page/grid data table is absent (`view_data_missing`).
    ViewDataMissing,
    /// A physical column no metaFields entry claims
    /// (`column_not_imported`).
    ColumnNotImported,
    /// A code table a field references is absent
    /// (`code_table_missing`).
    CodeTableMissing,
    /// metaLinks record-level links are not schema links
    /// (`record_links_not_imported`).
    RecordLinksNotImported,
    /// No recognized view-definition tables (`legacy_layout_unknown`).
    LayoutUnknown,
    /// JET3 text assumed windows-1252 (`code_page_assumed`).
    CodePageAssumed,
    /// A program carried over (`program_imported`, spec §6.2).
    ProgramImported,
    /// Check code stored read-only (`check_code_stored`, spec §6.2).
    CheckCodeStored,
    /// Deleted rows kept and counted (`records_deleted`, spec §6.2).
    RecordsDeleted,
    /// Text replaced undecodable bytes (`text_decoding`).
    TextDecoding,
    /// One view failed to map (`view_failed`, spec §6.2).
    ViewFailed,
    /// Views without per-view reference values land under
    /// `unvalidated` (spec §6.2).
    Unvalidated,
}

impl ReportKind {
    /// The report `kind` string (the golden/schema vocabulary where
    /// they overlap).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FieldNotImported => "field_not_imported",
            Self::FieldChanged => "field_imported_changed",
            Self::TableNotImported => "table_not_imported",
            Self::FieldColumnMissing => "field_column_missing",
            Self::ViewDataMissing => "view_data_missing",
            Self::ColumnNotImported => "column_not_imported",
            Self::CodeTableMissing => "code_table_missing",
            Self::RecordLinksNotImported => "record_links_not_imported",
            Self::LayoutUnknown => "legacy_layout_unknown",
            Self::CodePageAssumed => "code_page_assumed",
            Self::ProgramImported => "program_imported",
            Self::CheckCodeStored => "check_code_stored",
            Self::RecordsDeleted => "records_deleted",
            Self::TextDecoding => "text_decoding",
            Self::ViewFailed => "view_failed",
            Self::Unvalidated => "unvalidated",
        }
    }
}
