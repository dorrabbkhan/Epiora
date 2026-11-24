//! `RELATE <table> [<dkey>::<ckey> ...] [ALL]` (spec §5.4): the
//! one-to-many join EI7 materializes into the working view. Each
//! destination row expands to one row per matching child; child
//! columns colliding with destination names keep the destination
//! values. `MATCHING` (bare) drops unmatched rows, `ALL` keeps them
//! with missing child cells — merge_relate pins 11/13 for the same
//! key set. A keyless RELATE asks the session for the stored link
//! (`relate_no_link` when it has none).

use crate::expr::missing::{cell_ev, ei_to_string, is_empty, EV};
use crate::manage::{is_all, named};
use crate::view::{DatasetRef, JoinMode, LinkedDatasets, WorkingView};
use crate::LangError;
use epiora_core::dataset::{Column, ColumnData, Dataset};

/// Runs the relate: expands the view's dataset by the child join.
pub(crate) fn relate(
    view: &mut WorkingView,
    linked: &dyn LinkedDatasets,
    dref: &DatasetRef,
    keys: &[(String, String)],
) -> Result<(), LangError> {
    let child = named(linked, dref)?.clone();
    let keys: Vec<(String, String)> = if keys.is_empty() {
        let link =
            linked
                .link(view.dataset.id(), &dref.name)
                .ok_or_else(|| LangError::RelateNoLink {
                    parent: view.dataset.id().to_string(),
                    child: dref.name.clone(),
                })?;
        vec![(link.parent_key.clone(), link.child_key.clone())]
    } else {
        keys.to_vec()
    };

    let dkeys: Vec<usize> = keys
        .iter()
        .map(|(dk, _)| {
            pos(&view.dataset, dk).ok_or_else(|| LangError::UnknownVariable { name: dk.clone() })
        })
        .collect::<Result<_, _>>()?;
    let ckeys: Vec<usize> = keys
        .iter()
        .map(|(_, ck)| {
            pos(&child, ck).ok_or_else(|| LangError::UnknownVariable { name: ck.clone() })
        })
        .collect::<Result<_, _>>()?;

    // Child key index: key-tuple → child row indices.
    let mut index: std::collections::BTreeMap<Vec<String>, Vec<usize>> =
        std::collections::BTreeMap::new();
    for r in 0..child.row_count() {
        let k: Vec<String> = ckeys
            .iter()
            .map(|&c| key_text(&cell_ev(&child.columns()[c].data, r)))
            .collect();
        index.entry(k).or_default().push(r);
    }

    // The joined schema: destination columns, then the child columns
    // whose names don't collide.
    let mut cols: Vec<Column> = view.dataset.columns().to_vec();
    let mut child_map: Vec<(usize, usize)> = vec![]; // (child col, dest col)
    for (ci, c) in child.columns().iter().enumerate() {
        match position(&cols, &c.name) {
            Some(d) => child_map.push((ci, d)),
            None => {
                child_map.push((ci, cols.len()));
                cols.push(Column {
                    name: c.name.clone(),
                    data: blank_like(&c.data, 0),
                });
            }
        }
    }

    // Expand: every destination row emits one row per matching child;
    // ALL pads unmatched rows with missing child cells.
    let all = is_all(dref) || dref.join == JoinMode::All;
    let mut rows: Vec<Vec<Option<usize>>> = vec![]; // (parent row, child row)
    for r in 0..view.dataset.row_count() {
        let k: Vec<String> = dkeys
            .iter()
            .map(|&c| key_text(&cell_ev(&view.dataset.columns()[c].data, r)))
            .collect();
        match index.get(&k) {
            Some(hit) if !hit.is_empty() => {
                for &c in hit {
                    rows.push(vec![Some(r), Some(c)]);
                }
            }
            _ if all => rows.push(vec![Some(r), None]),
            _ => {}
        }
    }

    // Rewrite each column from (parent, child) pairs: destination
    // columns read the parent, added child columns read the child
    // (missing on padding rows).
    let n = rows.len();
    let dcols = view.dataset.columns().len();
    for (ci, col) in cols.iter_mut().enumerate() {
        let data = if ci < dcols {
            gather_opt(&col.data, &rows, |row| row[0])
        } else {
            let child_col = child_map.iter().find(|(_, d)| *d == ci).map(|(c, _)| *c);
            match child_col {
                Some(cc) => gather_opt(&child.columns()[cc].data, &rows, |row| row[1]),
                None => blank_like(&col.data, n),
            }
        };
        col.data = data;
    }

    // Selected/order remap through the expansion.
    let sel = view.selected.take();
    view.selected = sel.map(|s| {
        rows.iter()
            .enumerate()
            .filter(|(_, row)| s.contains(&(row[0].map(|x| x as u32).unwrap_or(u32::MAX))))
            .map(|(i, _)| i as u32)
            .collect()
    });
    view.order = None; // the expansion order is the view's order

    let mut meta: Vec<epiora_core::dataset::ColumnMeta> = view.dataset.meta().to_vec();
    while meta.len() < cols.len() {
        meta.push(epiora_core::dataset::ColumnMeta::default());
    }
    if let Ok(d) = Dataset::new(view.dataset.id(), view.dataset.version(), cols)
        .and_then(|d| d.with_meta(meta, view.dataset.label_sets().clone()))
    {
        view.dataset = d;
    }
    Ok(())
}

fn pos(ds: &Dataset, name: &str) -> Option<usize> {
    ds.columns()
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(name))
}

fn position(cols: &[Column], name: &str) -> Option<usize> {
    cols.iter().position(|c| c.name.eq_ignore_ascii_case(name))
}

fn key_text(v: &EV) -> String {
    if is_empty(v) {
        return String::new();
    }
    match v {
        EV::Real(x) if x.fract() == 0.0 && x.abs() < 1e15 => format!("{}", *x as i64),
        _ => ei_to_string(v),
    }
}

/// Gathers rows by an `Option` index — `None` emits a missing cell.
fn gather_opt(
    data: &ColumnData,
    rows: &[Vec<Option<usize>>],
    pick: impl Fn(&Vec<Option<usize>>) -> Option<usize>,
) -> ColumnData {
    macro_rules! g {
        ($v:expr, $variant:ident) => {
            ColumnData::$variant(
                rows.iter()
                    .map(|row| pick(row).map(|i| $v[i].clone()).flatten())
                    .collect(),
            )
        };
    }
    match data {
        ColumnData::Integer(v) => g!(v, Integer),
        ColumnData::Real(v) => g!(v, Real),
        ColumnData::Text(v) => g!(v, Text),
        ColumnData::Boolean(v) => g!(v, Boolean),
        ColumnData::Date(v) => g!(v, Date),
        ColumnData::Time(v) => g!(v, Time),
        ColumnData::DateTime(v) => g!(v, DateTime),
    }
}

fn blank_like(data: &ColumnData, n: usize) -> ColumnData {
    match data {
        ColumnData::Integer(_) => ColumnData::Integer(vec![None; n]),
        ColumnData::Real(_) => ColumnData::Real(vec![None; n]),
        ColumnData::Text(_) => ColumnData::Text(vec![None; n]),
        ColumnData::Boolean(_) => ColumnData::Boolean(vec![None; n]),
        ColumnData::Date(_) => ColumnData::Date(vec![None; n]),
        ColumnData::Time(_) => ColumnData::Time(vec![None; n]),
        ColumnData::DateTime(_) => ColumnData::DateTime(vec![None; n]),
    }
}
