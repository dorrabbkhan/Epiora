//! The EI3 field rows → `OutCol`s (ports `buildEi3Project`'s field
//! loop): the `EI3_TYPE` kinds, the NUMBER integer/real split, MIRROR
//! source values, `X(SRC,…)` code tables → label sets, and the
//! verbatim system columns (`UniqueKey`, `RECSTATUS`, `FKEY`).

use std::collections::HashSet;

use epiora_core::dataset::{ColumnMeta, LabelSet, Value};

use crate::ei7::datetime;
use crate::ei7::fields::OutCol;
use crate::ei7::meta::cell;
use crate::ei7::MapCtx;
use crate::report::ReportKind;
use crate::source::{Cell, TableData};

use super::meta::{parse_ei3_list, Ei3Field, ViewDef};

/// `EI3_TYPE[t]` — `None` for types that are form metadata, not data
/// (TEXTONLY, GROUP, …: `field_not_imported`).
fn ei3_kind(ty: &str) -> Option<&'static str> {
    Some(match ty {
        "TEXTBOX" | "PHONENUMBER" => "text",
        "NUMBER" => "number",
        "DATE" => "date",
        "CHECKBOX" | "YES/NO" => "boolean",
        "COMBO" | "LISTBOX" => "coded",
        "MIRROR" => "mirror",
        _ => return None,
    })
}

/// The view's field columns in `defRows` order, then the system
/// columns verbatim (`UniqueKey`, `RECSTATUS`, `FKEY` when present).
pub(crate) fn build_columns(ctx: &mut MapCtx, def: &ViewDef, data: &TableData) -> Vec<OutCol> {
    let mut cols = Vec::new();
    for f in &def.fields {
        if let Some(oc) = field_column(ctx, def, data, f) {
            cols.push(oc);
        }
    }
    for sys in ["UniqueKey", "RECSTATUS", "FKEY"] {
        let Some(c) = ctx.col_of(&def.data_table, sys) else {
            continue;
        };
        let ty: &'static str = if sys == "FKEY" { "text" } else { "integer" };
        let values = data
            .col_ci(&c)
            .map_or_else(|| vec![None; data.rows.len()], |ci| data.column(ci));
        cols.push(OutCol {
            name: c,
            ty,
            values: values
                .iter()
                .map(|v| datetime::convert_cell(v.as_ref(), ty))
                .collect(),
            meta: ColumnMeta {
                legacy_type: Some(sys.to_string()),
                ..ColumnMeta::default()
            },
        });
    }
    cols
}

/// One field row → an [`OutCol`] (`None` when the type is not data:
/// `field_not_imported` is recorded instead).
fn field_column(ctx: &mut MapCtx, def: &ViewDef, data: &TableData, f: &Ei3Field) -> Option<OutCol> {
    let view_name = def.view_name.as_str();
    let data_table = def.data_table.as_str();
    let kind = ei3_kind(&f.ty);
    let mut meta = ColumnMeta {
        prompt: f.prompt.clone(),
        legacy_type: Some(f.ty.clone()),
        ..ColumnMeta::default()
    };
    let Some(kind) = kind else {
        ctx.add(
            ReportKind::FieldNotImported,
            format!("{view_name}/{}", f.name),
            format!(
                "EI3 field type {} is form metadata",
                if f.ty.is_empty() {
                    "(empty)"
                } else {
                    f.ty.as_str()
                }
            ),
        );
        return None;
    };
    let col_name = ctx.col_of(data_table, &f.name);
    if col_name.is_none() && kind != "mirror" {
        ctx.add(
            ReportKind::FieldColumnMissing,
            format!("{view_name}/{}", f.name),
            format!("no column in {data_table}"),
        );
    }
    if let Some(cn) = &col_name {
        if *cn != f.name {
            meta.legacy_name = Some(cn.clone());
        }
    }
    let mut raw: Vec<Option<Cell>> = match &col_name {
        Some(cn) => data
            .col_ci(cn)
            .map_or_else(|| vec![None; data.rows.len()], |ci| data.column(ci)),
        None => vec![None; data.rows.len()],
    };
    let mut ty: &'static str = "text";
    match kind {
        "number" => {
            let has_decimals = f.decimals > 0.0 || f.formatstring.contains('.');
            ty = if has_decimals || !datetime::is_integral_column(&raw) {
                "real"
            } else {
                "integer"
            };
        }
        "boolean" | "date" => ty = kind,
        "mirror" => mirror(ctx, def, data, f, &mut raw),
        "coded" => coded(ctx, def, f, &mut meta),
        _ => {}
    }
    Some(OutCol {
        name: f.name.clone(),
        ty,
        values: raw
            .iter()
            .map(|v| datetime::convert_cell(v.as_ref(), ty))
            .collect(),
        meta,
    })
}

/// `kind === 'mirror'`: the column takes the source field's values and
/// reports `field_imported_changed`, or reports the missing source.
fn mirror(
    ctx: &mut MapCtx,
    def: &ViewDef,
    data: &TableData,
    f: &Ei3Field,
    raw: &mut Vec<Option<Cell>>,
) {
    let src_name = f.list_expr.as_deref().unwrap_or("").trim().to_string();
    let src_field = def
        .fields
        .iter()
        .find(|x| x.name_raw.to_lowercase() == src_name.to_lowercase());
    let src_col = src_field.and_then(|sf| ctx.col_of(&def.data_table, &sf.name_raw));
    match src_col {
        None => ctx.add(
            ReportKind::FieldColumnMissing,
            format!("{}/{}", def.view_name, f.name),
            format!(
                "mirror source {} not found",
                if src_name.is_empty() {
                    "(unset)"
                } else {
                    src_name.as_str()
                }
            ),
        ),
        Some(sc) => {
            *raw = data
                .col_ci(&sc)
                .map_or_else(|| vec![None; data.rows.len()], |ci| data.column(ci));
            ctx.add(
                ReportKind::FieldChanged,
                format!("{}/{}", def.view_name, f.name),
                format!("MIRROR field imported as the value of {src_name}"),
            );
        }
    }
}

/// `kind === 'coded'`: parse `X(SRC,…)`, resolve `SRC` through the
/// view's SOURCE map, claim the code table and build (or reuse) the
/// label set the column meta references.
fn coded(ctx: &mut MapCtx, def: &ViewDef, f: &Ei3Field, meta: &mut ColumnMeta) {
    let Some(spec) = parse_ei3_list(f.list_expr.as_deref().unwrap_or("")) else {
        return;
    };
    // `sourceMap.get(spec.source.toUpperCase()) || spec.source` — an
    // empty `Datatable` falls back to the spec text like JS `||`.
    let table = def
        .source_map
        .get(&spec.source.to_uppercase())
        .filter(|v| !v.is_empty())
        .cloned()
        .unwrap_or_else(|| spec.source.clone());
    let Some(rows) = ctx.rows_of(&table) else {
        ctx.add(
            ReportKind::CodeTableMissing,
            table,
            format!(
                "field {} references a code table that is not in the file",
                f.name
            ),
        );
        return;
    };
    ctx.claimed.insert(table.to_lowercase());
    let (stored, display) = spec.cols[0].clone();
    let id = if spec.cols.len() > 1 {
        format!("{table}#{stored}->{display}")
    } else {
        table.clone()
    };
    if !ctx.label_sets.contains_key(&id) {
        ctx.label_sets
            .insert(id.clone(), build_label_set(&id, &rows, &stored, &display));
    }
    meta.label_set = Some(id);
}

/// `stored`/`display` → first-seen `(value, label)` entries; rows
/// whose value is missing or `''` are skipped, dedup on `${v}`.
fn build_label_set(id: &str, t: &TableData, stored: &str, display: &str) -> LabelSet {
    let mut entries = Vec::new();
    let mut seen = HashSet::new();
    for r in 0..t.rows.len() {
        let Some(v) = cell(t, r, stored) else {
            continue;
        };
        if matches!(v, Cell::Text(s) if s.is_empty()) {
            continue;
        }
        let value = v.as_string();
        if !seen.insert(value.clone()) {
            continue;
        }
        // `String(ci(r,display) ?? v)` — a missing/null display cell
        // shows the stored value.
        let label = cell(t, r, display).map_or_else(|| value.clone(), Cell::as_string);
        entries.push((Value::Text(value), label));
    }
    LabelSet {
        id: id.to_string(),
        entries,
    }
}

/// `data.filter(r => Number(ci(r, recCol)) === 0).length` — JS
/// `Number` maps `null` and `''`/whitespace text to 0, so a missing
/// or blank RECSTATUS cell counts as deleted.
pub(crate) fn deleted_count(data: &TableData, rec_col: Option<&str>) -> u64 {
    let Some(cn) = rec_col else { return 0 };
    let Some(ci) = data.col_ci(cn) else { return 0 };
    data.rows
        .iter()
        .filter(|r| {
            r.get(ci).and_then(|c| c.as_ref()).is_none_or(|c| {
                c.as_number() == 0.0 || matches!(c, Cell::Text(s) if s.trim().is_empty())
            })
        })
        .count() as u64
}

/// The `Checkcode` texts of the field rows that have any — the
/// `blocks` count the golden records and the stored `CheckCode` text.
pub(crate) fn checkcode_texts(def: &ViewDef) -> Vec<String> {
    def.fields
        .iter()
        .filter(|f| !f.checkcode.trim().is_empty())
        .map(|f| f.checkcode.clone())
        .collect()
}
