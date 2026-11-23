//! Page-table row indexes: the `pageData` maps the golden builds per
//! page (`GlobalRecordId` → row) and `readField`'s spine lookup
//! through them.

use std::collections::HashMap;
use std::rc::Rc;

use super::meta::{MetaField, MetaPage};
use super::MapCtx;
use crate::report::ReportKind;
use crate::source::{Cell, TableData};

/// `pageData` state for one page id: `Missing` means the physical
/// page table is absent (already reported); `Empty` means the page id
/// is not one of the view's pages (golden's `|| new Map()`).
pub(crate) enum PageIdx {
    /// The page table is absent from the file.
    Missing,
    /// The page table exists; `grid → row` index.
    Present(HashMap<String, usize>),
    /// The field's page id is not a view page — an empty index.
    Empty,
}

/// `pageId → row index` keyed on the page table's GlobalRecordId —
/// the `pageData` build of `buildEi7Project`. `None` per page means
/// the physical table is absent (`view_data_missing` reported here,
/// once per missing page).
pub(crate) fn page_indexes(
    ctx: &mut MapCtx,
    view_name: &str,
    table_name: &str,
    v_pages: &[&MetaPage],
) -> HashMap<String, Option<HashMap<String, usize>>> {
    let mut page_data: HashMap<String, Option<HashMap<String, usize>>> = HashMap::new();
    for p in v_pages {
        let t = format!("{table_name}{}", p.page_id);
        ctx.claimed.insert(t.to_lowercase());
        let idx = match ctx.rows_of(&t) {
            None => {
                ctx.add(
                    ReportKind::ViewDataMissing,
                    view_name.to_string(),
                    format!("page table {t} is not in the file"),
                );
                None
            }
            Some(rows) => {
                let mut idx = HashMap::new();
                if let Some(g) = rows.col_ci("GlobalRecordId") {
                    for (ri, row) in rows.rows.iter().enumerate() {
                        if let Some(c) = row.get(g).and_then(|c| c.as_ref()) {
                            idx.insert(c.as_string(), ri);
                        }
                    }
                }
                Some(idx)
            }
        };
        page_data.insert(p.page_id.clone(), idx);
    }
    page_data
}

/// `readField` — a data field's values over the spine rows.
/// Returns `(values, col_name)`; `col_name` is the physical column.
pub(crate) fn read_field(
    ctx: &mut MapCtx,
    table_name: &str,
    f: &MetaField,
    p_idx: &PageIdx,
    spine: &Rc<TableData>,
    view_name: &str,
) -> (Vec<Option<Cell>>, Option<String>) {
    let t = format!("{table_name}{}", f.page_id);
    let col_name = match p_idx {
        PageIdx::Missing => None,
        _ => ctx.col_of(&t, &f.name),
    };
    if !matches!(p_idx, PageIdx::Missing) && col_name.is_none() {
        ctx.add(
            ReportKind::FieldColumnMissing,
            format!("{view_name}/{}", f.name),
            format!("no column in {t}"),
        );
        return (vec![None; spine.rows.len()], None);
    }
    let page_t = ctx.rows_of(&t);
    let grid_ci = spine.col_ci("GlobalRecordId");
    let values = (0..spine.rows.len())
        .map(|r| {
            let key = grid_ci
                .and_then(|c| spine.get(r, c))
                .map_or_else(|| "undefined".to_string(), Cell::as_string);
            let ri = match p_idx {
                PageIdx::Present(idx) => idx.get(&key).copied(),
                _ => None,
            };
            ri.and_then(|ri| {
                page_t.as_ref().and_then(|t| {
                    col_name
                        .as_ref()
                        .and_then(|cn| t.col_ci(cn))
                        .and_then(|c| t.get(ri, c))
                        .cloned()
                })
            })
        })
        .collect();
    (values, col_name)
}
