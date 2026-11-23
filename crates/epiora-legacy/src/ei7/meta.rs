//! The EI7 `meta*` table readers: each row becomes a small struct the
//! mapper uses. Missing meta tables read as empty (the golden treats
//! `db.hasTable(...) ? rows : []`); missing columns read as `None`.

use std::borrow::Borrow;

use crate::source::{Cell, TableData};

/// Case-insensitive cell lookup in one row of a [`TableData`]. `B`
/// lets callers pass `&TableData` or `&Rc<TableData>`.
pub(crate) fn cell<'a, B: Borrow<TableData>>(t: &'a B, row: usize, name: &str) -> Option<&'a Cell> {
    let t = t.borrow();
    t.col_ci(name).and_then(|c| t.get(row, c))
}

/// Cell → `String(v)` with JS semantics (`undefined`/`null` when the
/// column is absent or the cell missing).
pub(crate) fn s_cell(t: &TableData, row: usize, name: &str) -> String {
    match t.col_ci(name) {
        None => "undefined".to_string(),
        Some(c) => t
            .get(row, c)
            .map_or_else(|| "null".to_string(), Cell::as_string),
    }
}

/// Cell → trimmed `String(v)` (`""` when absent — the golden's
/// `String(x || '').trim()`).
pub(crate) fn s_trim(t: &TableData, row: usize, name: &str) -> String {
    match t.col_ci(name) {
        None => String::new(),
        Some(c) => t
            .get(row, c)
            .map_or_else(String::new, |v| v.as_string().trim().to_string()),
    }
}

/// Cell → `f64` (`f64::NAN` when absent — JS `Number(undefined)`).
pub(crate) fn n_cell(t: &TableData, row: usize, name: &str) -> f64 {
    t.col_ci(name)
        .and_then(|c| t.get(row, c))
        .map_or(f64::NAN, Cell::as_number)
}

/// JS truthiness of a raw cell: `0`, `0.0`, `NaN`, `""`, `false` and
/// missing are falsy.
pub(crate) fn js_truthy(c: Option<&Cell>) -> bool {
    match c {
        None => false,
        Some(Cell::Int(0)) | Some(Cell::Bool(false)) => false,
        Some(Cell::Real(x)) => *x != 0.0 && x.is_finite(),
        Some(Cell::Text(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

/// A `metaViews` row.
pub(crate) struct MetaView {
    /// `ViewId` as text.
    pub view_id: String,
    /// `Name`.
    pub name: String,
    /// The six view-level check-code columns, in `CHECK_COLS` order.
    pub check: Vec<String>,
}

/// A `metaPages` row.
pub(crate) struct MetaPage {
    /// `PageId` as text.
    pub page_id: String,
    /// `ViewId` as text.
    pub view_id: String,
    /// `Position`.
    pub position: f64,
    /// `CheckCodeBefore` text.
    pub check_before: String,
    /// `CheckCodeAfter` text.
    pub check_after: String,
}

/// A `metaFields` row.
pub(crate) struct MetaField {
    /// `FieldId` as text.
    pub field_id: String,
    /// `ViewId` as text.
    pub view_id: String,
    /// `PageId` as text.
    pub page_id: String,
    /// JS `f.PageId && String(f.PageId) !== ''` — falsy raw PageId
    /// (`0`, `""`, missing) means a system/spine field.
    pub has_page: bool,
    /// `FieldTypeId` — kept `f64` so non-integral ids miss the type
    /// table like the golden's object-key lookup does.
    pub field_type: f64,
    /// `Name` — also the physical column's name.
    pub name: String,
    /// `PromptText`.
    pub prompt: String,
    /// `Pattern`.
    pub pattern: String,
    /// `DataTableName`.
    pub data_table: String,
    /// `SourceTableName` (code tables).
    pub source_table: String,
    /// `CodeColumnName`.
    pub code_col: String,
    /// `TextColumnName`.
    pub text_col: String,
    /// `List` (inline legal values).
    pub list: String,
    /// `SourceFieldId` (Mirror).
    pub source_field_id: String,
    /// `RelatedViewId` (Relate).
    pub related_view_id: String,
    /// `CheckCodeBefore` text.
    pub check_before: String,
    /// `CheckCodeAfter` text.
    pub check_after: String,
}

/// A `metaGridColumns` row.
pub(crate) struct MetaGridCol {
    /// `FieldId` (the owning Grid field) as text.
    pub field_id: String,
    /// `GridColumnId` — the column order.
    pub grid_col_id: f64,
    /// `Name` — the physical column's name.
    pub name: String,
    /// `Text` (the column prompt).
    pub text: String,
    /// `Pattern`.
    pub pattern: String,
    /// `FieldTypeId` (`f64` like `MetaField`).
    pub field_type: f64,
    /// `SourceTableName`.
    pub source_table: String,
    /// `CodeColumnName`.
    pub code_col: String,
    /// `TextColumnName`.
    pub text_col: String,
    /// `List`.
    pub list: String,
}

/// The six view-level check-code columns (`CHECK_COLS`).
pub(crate) const CHECK_COLS: [&str; 6] = [
    "CheckCode",
    "CheckCodeBefore",
    "CheckCodeAfter",
    "RecordCheckCodeBefore",
    "RecordCheckCodeAfter",
    "CheckCodeVariableDefinitions",
];

/// `meta*` tables the unclaimed pass always skips (`KNOWN_META`,
/// lowercase).
pub(crate) const KNOWN_META: [&str; 17] = [
    "metaviews",
    "metapages",
    "metafields",
    "metafieldtypes",
    "metadatatypes",
    "metadbinfo",
    "metaprograms",
    "metagridcolumns",
    "metalinks",
    "metabackgrounds",
    "metaimages",
    "metalayers",
    "metamaps",
    "metamaplayers",
    "metamappoints",
    "metapatterns",
    "metalayerrendertypes",
];

/// `metaDbInfo` first row → (`ProjectName`, `ProjectId` with braces
/// stripped). Both empty when the table or row is absent.
pub(crate) fn read_db_info(t: Option<&TableData>) -> (String, String) {
    let Some(t) = t else {
        return (String::new(), String::new());
    };
    if t.rows.is_empty() {
        return (String::new(), String::new());
    }
    let name = cell(t, 0, "ProjectName").map_or_else(String::new, Cell::as_string);
    let id = cell(t, 0, "ProjectId")
        .map_or_else(String::new, Cell::as_string)
        .replace(['{', '}'], "");
    (name, id)
}

/// All `metaViews` rows.
pub(crate) fn read_views(t: Option<&TableData>) -> Vec<MetaView> {
    let Some(t) = t else { return Vec::new() };
    (0..t.rows.len())
        .map(|r| MetaView {
            view_id: s_cell(t, r, "ViewId"),
            name: s_cell(t, r, "Name"),
            check: CHECK_COLS
                .iter()
                .map(|c| cell(t, r, c).map_or_else(String::new, Cell::as_string))
                .collect(),
        })
        .collect()
}

/// All `metaPages` rows.
pub(crate) fn read_pages(t: Option<&TableData>) -> Vec<MetaPage> {
    let Some(t) = t else { return Vec::new() };
    (0..t.rows.len())
        .map(|r| MetaPage {
            page_id: s_cell(t, r, "PageId"),
            view_id: s_cell(t, r, "ViewId"),
            position: n_cell(t, r, "Position"),
            check_before: cell(t, r, "CheckCodeBefore").map_or_else(String::new, Cell::as_string),
            check_after: cell(t, r, "CheckCodeAfter").map_or_else(String::new, Cell::as_string),
        })
        .collect()
}

/// All `metaFields` rows, in file order.
pub(crate) fn read_fields(t: Option<&TableData>) -> Vec<MetaField> {
    let Some(t) = t else { return Vec::new() };
    let text = |r: usize, n: &str| cell(t, r, n).map_or_else(String::new, Cell::as_string);
    (0..t.rows.len())
        .map(|r| MetaField {
            field_id: s_cell(t, r, "FieldId"),
            view_id: s_cell(t, r, "ViewId"),
            page_id: s_cell(t, r, "PageId"),
            has_page: js_truthy(cell(t, r, "PageId")) && !s_cell(t, r, "PageId").is_empty(),
            field_type: n_cell(t, r, "FieldTypeId"),
            name: text(r, "Name"),
            prompt: text(r, "PromptText"),
            pattern: text(r, "Pattern"),
            data_table: text(r, "DataTableName"),
            source_table: s_trim(t, r, "SourceTableName"),
            // Raw cell text: the golden interpolates the untrimmed
            // `f.CodeColumnName`/`f.TextColumnName` into its report
            // (`"the  COUNTYCODE code"`), while `labelSetFor` trims.
            code_col: text(r, "CodeColumnName"),
            text_col: text(r, "TextColumnName"),
            list: text(r, "List"),
            source_field_id: s_cell(t, r, "SourceFieldId"),
            related_view_id: s_cell(t, r, "RelatedViewId"),
            check_before: text(r, "CheckCodeBefore"),
            check_after: text(r, "CheckCodeAfter"),
        })
        .collect()
}

/// All `metaGridColumns` rows.
pub(crate) fn read_grid_cols(t: Option<&TableData>) -> Vec<MetaGridCol> {
    let Some(t) = t else { return Vec::new() };
    (0..t.rows.len())
        .map(|r| MetaGridCol {
            field_id: s_cell(t, r, "FieldId"),
            grid_col_id: n_cell(t, r, "GridColumnId"),
            name: cell(t, r, "Name").map_or_else(String::new, Cell::as_string),
            text: cell(t, r, "Text").map_or_else(String::new, Cell::as_string),
            pattern: cell(t, r, "Pattern").map_or_else(String::new, Cell::as_string),
            field_type: n_cell(t, r, "FieldTypeId"),
            source_table: s_trim(t, r, "SourceTableName"),
            code_col: cell(t, r, "CodeColumnName").map_or_else(String::new, Cell::as_string),
            text_col: cell(t, r, "TextColumnName").map_or_else(String::new, Cell::as_string),
            list: cell(t, r, "List").map_or_else(String::new, Cell::as_string),
        })
        .collect()
}

/// `metaPrograms` rows → (`Name`, `Content`) verbatim (`String(v)`,
/// `Content ?? ''`).
pub(crate) fn read_programs(t: Option<&TableData>) -> Vec<(String, String)> {
    let Some(t) = t else { return Vec::new() };
    (0..t.rows.len())
        .map(|r| {
            (
                s_cell(t, r, "Name"),
                cell(t, r, "Content").map_or_else(String::new, Cell::as_string),
            )
        })
        .collect()
}

/// `metaLinks` rows → `{FromViewId}→{ToViewId}` items.
pub(crate) fn read_link_items(t: Option<&TableData>) -> Vec<String> {
    let Some(t) = t else { return Vec::new() };
    (0..t.rows.len())
        .map(|r| {
            format!(
                "{}→{}",
                s_cell(t, r, "FromViewId"),
                s_cell(t, r, "ToViewId")
            )
        })
        .collect()
}
