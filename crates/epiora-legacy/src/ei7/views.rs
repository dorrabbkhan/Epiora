//! Per-view dataset assembly (ports `buildEi7Project`'s view loop):
//! spine + page-table join on `GlobalRecordId`, system/data/grid/
//! mirror/relate field handling, the unclaimed-column pass, deleted
//! records, and check-code block counting.

use std::collections::HashMap;
use std::rc::Rc;

use epiora_core::dataset::{ColumnMeta, DatasetLink, LinkKind};

use super::datetime;
use super::fields::{self, OutCol};
use super::meta::{MetaField, MetaPage, MetaView};
use super::page::{self, PageIdx};
use super::MapCtx;
use crate::error::LegacyError;
use crate::project::{CheckCode, LegacyView};
use crate::report::ReportKind;
use crate::source::{Cell, TableData};

/// What one view contributes to the project.
pub(crate) struct ViewOut {
    /// The view dataset + metadata.
    pub view: LegacyView,
    /// Relate/grid links this view declared, in field order.
    pub links: Vec<DatasetLink>,
    /// Grid child datasets (`(id, columns)`), in field order.
    pub grids: Vec<(String, Vec<OutCol>)>,
    /// Stored check code when the view has any.
    pub check: Option<CheckCode>,
}

/// One view → `ViewOut` (ports the `for (const v of views)` body).
pub(crate) fn build_view(
    ctx: &mut MapCtx,
    v: &MetaView,
    fields: &[MetaField],
    pages: &[MetaPage],
    grid_cols: &[super::meta::MetaGridCol],
) -> Result<ViewOut, LegacyError> {
    let v_fields: Vec<&MetaField> = fields.iter().filter(|f| f.view_id == v.view_id).collect();
    let mut v_pages: Vec<&MetaPage> = pages.iter().filter(|p| p.view_id == v.view_id).collect();
    v_pages.sort_by(|a, b| {
        a.position
            .partial_cmp(&b.position)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let page_by_id: HashMap<&str, &MetaPage> =
        v_pages.iter().map(|p| (p.page_id.as_str(), *p)).collect();

    // view.TableName: first non-empty DataTableName in metaFields order.
    let table_name = v_fields
        .iter()
        .map(|f| f.data_table.as_str())
        .find(|s| !s.trim().is_empty())
        .unwrap_or(v.name.as_str())
        .to_string();
    ctx.claimed.insert(table_name.to_lowercase());

    let spine_data = ctx.rows_of(&table_name);
    if spine_data.is_none() {
        ctx.add(
            ReportKind::ViewDataMissing,
            v.name.clone(),
            format!("base table {table_name} is not in the file"),
        );
    }

    let sys_fields: Vec<&MetaField> = v_fields.iter().copied().filter(|f| !f.has_page).collect();
    let mut data_fields: Vec<&MetaField> =
        v_fields.iter().copied().filter(|f| f.has_page).collect();
    data_fields.sort_by(|a, b| {
        let pa = page_by_id
            .get(a.page_id.as_str())
            .map_or(0.0, |p| p.position);
        let pb = page_by_id
            .get(b.page_id.as_str())
            .map_or(0.0, |p| p.position);
        pa.partial_cmp(&pb)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.field_id
                    .parse::<f64>()
                    .unwrap_or(f64::NAN)
                    .partial_cmp(&b.field_id.parse::<f64>().unwrap_or(f64::NAN))
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    // pageId → row index keyed on the page table's GlobalRecordId.
    let page_data = page::page_indexes(ctx, &v.name, &table_name, &v_pages);

    let mut cols: Vec<OutCol> = Vec::new();
    let mut links: Vec<DatasetLink> = Vec::new();
    let mut grids: Vec<(String, Vec<OutCol>)> = Vec::new();
    // `spineRows === null` — the base table is missing, not just empty.
    let spine_present = spine_data.is_some();
    let spine = spine_data.unwrap_or_else(|| Rc::new(TableData::empty(&table_name)));

    for f in &sys_fields {
        let kind = fields::type_kind(f.field_type).unwrap_or("meta");
        let col_name = if spine_present {
            ctx.col_of(&table_name, &f.name)
        } else {
            None
        };
        let raw: Vec<Option<Cell>> = col_name.as_ref().map_or_else(
            || vec![None; spine.rows.len()],
            |cn| {
                spine
                    .col_ci(cn)
                    .map_or_else(|| vec![None; spine.rows.len()], |ci| spine.column(ci))
            },
        );
        if spine_present && col_name.is_none() {
            ctx.add(
                ReportKind::FieldColumnMissing,
                format!("{}/{}", v.name, f.name),
                format!("no column in {table_name}"),
            );
        }
        let ty = if kind == "system_int" {
            "integer"
        } else {
            "text"
        };
        cols.push(OutCol {
            name: f.name.clone(),
            ty,
            values: raw
                .iter()
                .map(|x| datetime::convert_cell(x.as_ref(), ty))
                .collect(),
            meta: ColumnMeta {
                prompt: (!f.prompt.is_empty()).then(|| f.prompt.clone()),
                legacy_type: Some(fields::field_type_name(f.field_type)),
                legacy_name: col_name.filter(|cn| *cn != f.name),
                label_set: None,
            },
        });
    }

    let field_by_id: HashMap<&str, &MetaField> =
        fields.iter().map(|f| (f.field_id.as_str(), f)).collect();

    for f in data_fields {
        let kind = fields::type_kind(f.field_type);
        let p_idx = match page_data.get(&f.page_id) {
            Some(None) => PageIdx::Missing,
            Some(Some(idx)) => PageIdx::Present(idx.clone()),
            None => PageIdx::Empty,
        };
        let mut meta = ColumnMeta {
            prompt: (!f.prompt.is_empty()).then(|| f.prompt.clone()),
            legacy_type: Some(fields::field_type_name(f.field_type)),
            ..ColumnMeta::default()
        };
        match kind {
            Some("meta") | Some("binary") => {
                let suffix = if kind == Some("binary") {
                    " (binary OLE data)"
                } else {
                    ""
                };
                ctx.add(
                    ReportKind::FieldNotImported,
                    format!("{}/{}", v.name, f.name),
                    format!(
                        "{} is form metadata{suffix}",
                        fields::field_type_name(f.field_type)
                    ),
                );
            }
            Some("link") => {
                let child = ctx
                    .view_name(&f.related_view_id)
                    .unwrap_or_else(|| format!("view{}", f.related_view_id));
                links.push(DatasetLink {
                    parent: v.name.clone(),
                    child,
                    child_key: "FKEY".to_string(),
                    parent_key: "GlobalRecordId".to_string(),
                    kind: LinkKind::Relate,
                });
            }
            Some("grid") => {
                let g = fields::build_grid(ctx, f, &v.name, &table_name, grid_cols);
                links.push(g.link);
                grids.push((g.dataset_id, g.cols));
            }
            Some("mirror") => {
                let Some(src) = field_by_id.get(f.source_field_id.as_str()).copied() else {
                    ctx.add(
                        ReportKind::FieldColumnMissing,
                        format!("{}/{}", v.name, f.name),
                        "mirror source field not found".to_string(),
                    );
                    cols.push(OutCol {
                        name: f.name.clone(),
                        ty: "text",
                        values: vec![None; spine.rows.len()],
                        meta,
                    });
                    continue;
                };
                // `pageData.get(id) || new Map()`: a null (missing
                // page table) collapses to an empty index, so the
                // source column is still looked up and reported.
                let src_idx = match page_data.get(&src.page_id) {
                    Some(Some(idx)) => PageIdx::Present(idx.clone()),
                    Some(None) | None => PageIdx::Empty,
                };
                let (values, _) =
                    page::read_field(ctx, &table_name, src, &src_idx, &spine, &v.name);
                let sk = fields::type_kind(src.field_type).unwrap_or("text");
                let ty = match sk {
                    "number" => "real",
                    "system_int" => "integer",
                    "boolean" | "date" | "time" | "date_time" => sk,
                    _ => "text",
                };
                ctx.add(
                    ReportKind::FieldChanged,
                    format!("{}/{}", v.name, f.name),
                    format!("Mirror field imported as the value of {}", src.name),
                );
                cols.push(OutCol {
                    name: f.name.clone(),
                    ty,
                    values: values
                        .iter()
                        .map(|x| datetime::convert_cell(x.as_ref(), ty))
                        .collect(),
                    meta,
                });
            }
            _ => {
                let (values, col_name) =
                    page::read_field(ctx, &table_name, f, &p_idx, &spine, &v.name);
                meta.legacy_name = col_name.filter(|cn| *cn != f.name);
                let mut ty = "text";
                if kind == Some("number") {
                    ty = if f.pattern.contains('.') || !datetime::is_integral_column(&values) {
                        "real"
                    } else {
                        "integer"
                    };
                } else if let Some(k @ ("boolean" | "date" | "time" | "date_time")) = kind {
                    ty = k;
                }
                if matches!(kind, Some("coded") | Some("codes") | Some("option")) {
                    if let Some(lid) = fields::label_set_for(ctx, &fields::CodeRef::from(f)) {
                        meta.label_set = Some(lid);
                    }
                }
                if kind == Some("codes")
                    && !f.code_col.is_empty()
                    && !f.text_col.is_empty()
                    && f.code_col != f.text_col
                {
                    ctx.add(
                        ReportKind::FieldChanged,
                        format!("{}/{}", v.name, f.name),
                        format!(
                            "Codes field stores the {} display value; the {} code is kept in its label set",
                            f.text_col, f.code_col
                        ),
                    );
                }
                if kind.is_none() {
                    ctx.add(
                        ReportKind::FieldNotImported,
                        format!("{}/{}", v.name, f.name),
                        format!("unknown field type {} imported as text", f.field_type),
                    );
                }
                cols.push(OutCol {
                    name: f.name.clone(),
                    ty,
                    values: values
                        .iter()
                        .map(|x| datetime::convert_cell(x.as_ref(), ty))
                        .collect(),
                    meta,
                });
            }
        }
    }

    // Columns no field claims → report (spine + every page table).
    let used: Vec<String> = v_fields.iter().map(|f| f.name.to_lowercase()).collect();
    for t in std::iter::once(table_name.clone())
        .chain(v_pages.iter().map(|p| format!("{table_name}{}", p.page_id)))
    {
        if let Some(td) = ctx.rows_of(&t) {
            for c in &td.cols {
                let lc = c.name.to_lowercase();
                if used.iter().any(|u| u == &lc)
                    || matches!(
                        lc.as_str(),
                        "recstatus" | "uniquekey" | "globalrecordid" | "fkey" | "uniquerowid"
                    )
                {
                    continue;
                }
                ctx.add(
                    ReportKind::ColumnNotImported,
                    format!("{t}/{}", c.name),
                    "no metaFields entry".to_string(),
                );
            }
        }
    }

    // Deleted rows stay in the dataset; only the count is recorded.
    let deleted = spine
        .col_ci("RecStatus")
        .map(|ci| {
            spine
                .rows
                .iter()
                .filter(|r| {
                    r.get(ci)
                        .and_then(|c| c.as_ref())
                        .is_none_or(|c| c.as_number() == 0.0)
                })
                .count() as u64
        })
        .unwrap_or(0);

    let blocks: u32 = v.check.iter().map(|t| datetime::count_blocks(t)).sum();
    let extra: u32 = v_pages
        .iter()
        .map(|p| u32::from(!p.check_before.is_empty()) + u32::from(!p.check_after.is_empty()))
        .sum::<u32>()
        + v_fields
            .iter()
            .map(|f| u32::from(!f.check_before.is_empty()) + u32::from(!f.check_after.is_empty()))
            .sum::<u32>();
    let check = (blocks + extra > 0).then(|| {
        let mut parts: Vec<String> = v.check.iter().filter(|t| !t.is_empty()).cloned().collect();
        for p in &v_pages {
            if !p.check_before.is_empty() {
                parts.push(p.check_before.clone());
            }
            if !p.check_after.is_empty() {
                parts.push(p.check_after.clone());
            }
        }
        for f in &v_fields {
            if !f.check_before.is_empty() {
                parts.push(f.check_before.clone());
            }
            if !f.check_after.is_empty() {
                parts.push(f.check_after.clone());
            }
        }
        CheckCode {
            view: v.name.clone(),
            text: parts.join("\n\n"),
            blocks: blocks + extra,
        }
    });

    let (columns, metas) = super::emit::out_columns(cols);
    let key_column = if columns
        .iter()
        .any(|c| c.name.eq_ignore_ascii_case("GlobalRecordId"))
    {
        "GlobalRecordId"
    } else if columns
        .iter()
        .any(|c| c.name.eq_ignore_ascii_case("UniqueKey"))
    {
        "UniqueKey"
    } else {
        ""
    };
    let dataset = super::emit::finish_dataset(&v.name, columns, metas, ctx)?;
    Ok(ViewOut {
        view: LegacyView {
            name: v.name.clone(),
            dataset,
            key_column: key_column.to_string(),
            deleted_records: deleted,
        },
        links,
        grids,
        check,
    })
}
