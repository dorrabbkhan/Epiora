//! The EI7 field-type tables and the per-field column builders
//! (ports `ei7-grid.mjs`: `FIELD_TYPE`, `TYPE_KIND`,
//! `buildGridDataset`, and the `labelSetFor` code-table resolution
//! from `ei7-project.mjs`).

use std::collections::HashSet;
use std::rc::Rc;

use epiora_core::dataset::{ColumnMeta, DatasetLink, LabelSet, LinkKind, Value};

use super::meta::{cell, MetaField, MetaGridCol};
use super::{datetime, MapCtx};
use crate::report::ReportKind;
use crate::source::{Cell, TableData};

/// A column being assembled for a view/grid dataset: golden type
/// string + converted values + the meta the golden would emit.
pub(crate) struct OutCol {
    /// Column name.
    pub name: String,
    /// Golden type: integer|real|text|boolean|date|time|date_time.
    pub ty: &'static str,
    /// Converted values.
    pub values: Vec<Option<datetime::Typed>>,
    /// Column meta (empty when nothing recorded).
    pub meta: ColumnMeta,
}

/// `FieldTypeId` → how the field lands (`TYPE_KIND`), `None` for
/// unknown or non-integral ids (JS `TYPE_KIND[t]` object-key lookup).
pub(crate) fn type_kind(t: f64) -> Option<&'static str> {
    if !t.is_finite() || libm::trunc(t) != t {
        return None;
    }
    let t = t as i64;
    Some(match t {
        1 | 3 | 4 | 6 => "text",
        5 => "number",
        7 => "date",
        8 => "time",
        9 => "date_time",
        10 | 11 => "boolean",
        12 => "option",
        15 => "mirror",
        17 | 19 => "coded",
        18 => "codes",
        25 => "text",
        26 | 28 => "system_text",
        22 | 23 => "system_int",
        24 => "system_fkey",
        2 | 13 | 21 => "meta",
        14 => "binary",
        20 => "link",
        16 => "grid",
        _ => return None,
    })
}

/// `FIELD_TYPE[t] || 'type' + t` — the EI7 field-type name.
pub(crate) fn field_type_name(t: f64) -> String {
    if t.is_finite() && libm::trunc(t) == t {
        let name = match t as i64 {
            1 => Some("Text"),
            2 => Some("LabelTitle"),
            3 => Some("TextUppercase"),
            4 => Some("Multiline"),
            5 => Some("Number"),
            6 => Some("PhoneNumber"),
            7 => Some("Date"),
            8 => Some("Time"),
            9 => Some("DateTime"),
            10 => Some("Checkbox"),
            11 => Some("YesNo"),
            12 => Some("Option"),
            13 => Some("CommandButton"),
            14 => Some("Image"),
            15 => Some("Mirror"),
            16 => Some("Grid"),
            17 => Some("LegalValues"),
            18 => Some("Codes"),
            19 => Some("CommentLegal"),
            20 => Some("Relate"),
            21 => Some("Group"),
            22 => Some("RecStatus"),
            23 => Some("UniqueKey"),
            24 => Some("ForeignKey"),
            25 => Some("GUID"),
            26 => Some("GlobalRecordId"),
            27 => Some("List"),
            28 => Some("UniqueRowId"),
            99 => Some("Unknown"),
            _ => None,
        };
        if let Some(n) = name {
            return n.to_string();
        }
    }
    format!("type{t}")
}

/// The field's code-table reference (`SourceTableName`, `List`,
/// `CodeColumnName`, `TextColumnName`, `Name`, kind).
pub(crate) struct CodeRef<'a> {
    /// Field name (for `list:` ids and report items).
    pub name: &'a str,
    /// `SourceTableName` (trimmed — the golden's `src`).
    pub source_table: &'a str,
    /// `CodeColumnName` (raw — `label_set_for` trims like `labelSetFor`).
    pub code_col: &'a str,
    /// `TextColumnName` (raw).
    pub text_col: &'a str,
    /// `List` (inline legal values).
    pub list: &'a str,
    /// The field's `TYPE_KIND`.
    pub kind: Option<&'static str>,
}

impl<'a> From<&'a MetaField> for CodeRef<'a> {
    fn from(f: &'a MetaField) -> Self {
        Self {
            name: &f.name,
            source_table: &f.source_table,
            code_col: &f.code_col,
            text_col: &f.text_col,
            list: &f.list,
            kind: type_kind(f.field_type),
        }
    }
}

impl<'a> From<&'a MetaGridCol> for CodeRef<'a> {
    fn from(g: &'a MetaGridCol) -> Self {
        Self {
            name: &g.name,
            source_table: &g.source_table,
            code_col: &g.code_col,
            text_col: &g.text_col,
            list: &g.list,
            kind: type_kind(g.field_type),
        }
    }
}

/// `labelSetFor(f)` — inline `List` entries or a code-table lookup;
/// the label-set id the column meta references, or `None`.
pub(crate) fn label_set_for(ctx: &mut MapCtx, f: &CodeRef) -> Option<String> {
    let src = f.source_table.trim();
    if src.is_empty() {
        let list: Vec<String> = f
            .list
            .split('\n')
            .map(|s| s.trim_end_matches('\r').trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if list.is_empty() {
            return None;
        }
        let id = format!("list:{}", f.name);
        if !ctx.label_sets.contains_key(&id) {
            ctx.label_sets.insert(
                id.clone(),
                LabelSet {
                    id: id.clone(),
                    entries: list
                        .into_iter()
                        .map(|v| (Value::Text(v.clone()), v))
                        .collect(),
                },
            );
        }
        return Some(id);
    }
    let table = match ctx.rows_of(src) {
        Some(t) => t,
        None => {
            ctx.add(
                ReportKind::CodeTableMissing,
                src,
                format!(
                    "field {} references a code table that is not in the file",
                    f.name
                ),
            );
            return None;
        }
    };
    ctx.claimed.insert(src.to_lowercase());
    // `labelSetFor` trims both names; a space-only TextColumnName
    // falls back to the code column like the golden's `||` does.
    let code_col = f.code_col.trim();
    let text_col = {
        let t = f.text_col.trim();
        if t.is_empty() {
            code_col
        } else {
            t
        }
    };
    let use_pair = f.kind == Some("codes")
        && !code_col.is_empty()
        && !text_col.is_empty()
        && code_col != text_col;
    let id = if use_pair {
        format!("{src}#{code_col}->{text_col}")
    } else {
        src.to_string()
    };
    if ctx.label_sets.contains_key(&id) {
        return Some(id);
    }
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    let value_col = if use_pair { code_col } else { text_col };
    for r in 0..table.rows.len() {
        let v = cell(&table, r, value_col);
        let l = cell(&table, r, text_col);
        // `v === null || v === undefined || v === ''` → skip.
        let skip = match v {
            None => true,
            Some(Cell::Text(s)) => s.is_empty(),
            Some(_) => false,
        };
        if skip {
            continue;
        }
        let key = format!("{}→{}", js_str(v), js_str(l));
        if !seen.insert(key) {
            continue;
        }
        let label = match l {
            None => v.map_or_else(String::new, Cell::as_string),
            Some(c) => c.as_string(),
        };
        let value = v.map_or_else(String::new, Cell::as_string);
        entries.push((Value::Text(value), label));
    }
    ctx.label_sets.insert(
        id.clone(),
        LabelSet {
            id: id.clone(),
            entries,
        },
    );
    Some(id)
}

/// JS `${v}` for a cell: `undefined` (column absent), `null` (missing
/// cell) or `String(v)` — used for the golden's dedup keys.
fn js_str(c: Option<&Cell>) -> String {
    match c {
        None => "undefined".to_string(),
        Some(c) => c.as_string(),
    }
}

/// The grid dataset a Grid field produces plus its project link
/// (`buildGridDataset`).
pub(crate) struct GridOut {
    /// The child dataset id (`<tableName><PageId><FieldName>`).
    pub dataset_id: String,
    /// Its columns.
    pub cols: Vec<OutCol>,
    /// The project link to it.
    pub link: DatasetLink,
}

/// `buildGridDataset` — one `kind == 'grid'` field → child dataset +
/// `kind: grid` link.
pub(crate) fn build_grid(
    ctx: &mut MapCtx,
    f: &MetaField,
    view_name: &str,
    table_name: &str,
    grid_cols: &[MetaGridCol],
) -> GridOut {
    let grid_table = format!("{table_name}{}{}", f.page_id, f.name);
    ctx.claimed.insert(grid_table.to_lowercase());
    let mut g_fields: Vec<&MetaGridCol> = grid_cols
        .iter()
        .filter(|g| g.field_id == f.field_id)
        .collect();
    g_fields.sort_by(|a, b| {
        a.grid_col_id
            .partial_cmp(&b.grid_col_id)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let g_table: Option<Rc<TableData>> = ctx.rows_of(&grid_table);
    let n_rows = match &g_table {
        Some(t) => t.rows.len(),
        None => {
            ctx.add(
                ReportKind::ViewDataMissing,
                view_name.to_string(),
                format!("grid table {grid_table} is not in the file"),
            );
            0
        }
    };
    let mut cols = Vec::new();
    if let (Some(t), Some(uk)) = (&g_table, ctx.col_of(&grid_table, "UniqueKey")) {
        if let Some(ci) = t.col_ci(&uk) {
            let values = t
                .column(ci)
                .iter()
                .map(|v| datetime::convert_cell(v.as_ref(), "integer"))
                .collect();
            cols.push(OutCol {
                name: "UniqueKey".to_string(),
                ty: "integer",
                values,
                meta: ColumnMeta {
                    legacy_type: Some("UniqueKey".to_string()),
                    ..ColumnMeta::default()
                },
            });
        }
    }
    for g in g_fields {
        let gk = type_kind(g.field_type);
        let col_name = ctx.col_of(&grid_table, &g.name);
        let gvals: Vec<Option<Cell>> = match (&g_table, &col_name) {
            (Some(t), Some(cn)) => t
                .col_ci(cn)
                .map_or_else(|| vec![None; n_rows], |ci| t.column(ci)),
            _ => vec![None; n_rows],
        };
        let ty: &'static str = match gk {
            Some("system_int") => "integer",
            Some("number") => {
                if g.pattern.contains('.') || !datetime::is_integral_column(&gvals) {
                    "real"
                } else {
                    "integer"
                }
            }
            Some("boolean") => "boolean",
            Some("date") => "date",
            Some("time") => "time",
            Some("date_time") => "date_time",
            _ => "text",
        };
        let mut meta = ColumnMeta {
            prompt: if g.text.is_empty() {
                None
            } else {
                Some(g.text.clone())
            },
            legacy_type: Some(field_type_name(g.field_type)),
            ..ColumnMeta::default()
        };
        if matches!(gk, Some("coded") | Some("codes") | Some("option")) {
            if let Some(lid) = label_set_for(ctx, &CodeRef::from(g)) {
                meta.label_set = Some(lid);
            }
        }
        cols.push(OutCol {
            name: g.name.clone(),
            ty,
            values: gvals
                .iter()
                .map(|v| datetime::convert_cell(v.as_ref(), ty))
                .collect(),
            meta,
        });
    }
    GridOut {
        link: DatasetLink {
            parent: view_name.to_string(),
            child: grid_table.clone(),
            child_key: "FKEY".to_string(),
            parent_key: "GlobalRecordId".to_string(),
            kind: LinkKind::Grid,
        },
        dataset_id: grid_table,
        cols,
    }
}
