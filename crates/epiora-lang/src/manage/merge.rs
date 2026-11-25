//! `MERGE '<file>':<table> <dkey>::<skey> [<mode>]` (spec §5.4):
//! the source table's rows merge into the current table on the key
//! pair(s) — EI7's "match or insert": matched destination rows take
//! the source's shared columns (the last matching source row wins,
//! merge_relate's GUID pin), unmatched source rows insert carrying
//! destination-only columns missing, and source-only columns join the
//! destination schema. `UPDATE` suppresses the inserts, `APPEND`
//! appends every source row unconditionally (owner-pinned: neither
//! mode is compared to EI7 — 7.2.7.0 crashes on the option word).

use crate::expr::missing::{as_f64, cell_ev, ei_to_string, is_empty, EV};
use crate::expr::ops::parse_f64;
use crate::manage::{named, MergeMode};
use crate::view::{DatasetRef, LinkedDatasets, WorkingView};
use crate::LangError;
use epiora_core::dataset::{Column, ColumnData, Dataset};

/// Runs the merge against the named source table.
pub(crate) fn merge(
    view: &mut WorkingView,
    linked: &dyn LinkedDatasets,
    dref: &DatasetRef,
    keys: &[(String, String)],
    mode: MergeMode,
) -> Result<(), LangError> {
    let src = named(linked, dref)?.clone();
    if keys.is_empty() {
        return Err(LangError::MergeKeyType {
            column: String::new(),
            message: "MERGE requires a key pair".into(),
        });
    }
    let dest_keys: Vec<usize> = keys
        .iter()
        .map(|(dk, _)| {
            col_index(&view.dataset, dk)
                .ok_or_else(|| LangError::UnknownVariable { name: dk.clone() })
        })
        .collect::<Result<_, _>>()?;
    let src_keys: Vec<usize> = keys
        .iter()
        .map(|(_, sk)| {
            col_index(&src, sk).ok_or_else(|| LangError::UnknownVariable { name: sk.clone() })
        })
        .collect::<Result<_, _>>()?;
    for (&d, &s) in dest_keys.iter().zip(src_keys.iter()) {
        if !key_family(&view.dataset.columns()[d].data, &src.columns()[s].data) {
            return Err(LangError::MergeKeyType {
                column: view.dataset.columns()[d].name.clone(),
                message: format!(
                    "merge key type mismatch: {} vs {}",
                    view.dataset.columns()[d].name,
                    src.columns()[s].name
                ),
            });
        }
    }

    // Destination key index: key-tuple → row indices.
    let mut index: std::collections::BTreeMap<Vec<String>, Vec<usize>> =
        std::collections::BTreeMap::new();
    for r in 0..view.dataset.row_count() {
        let k: Vec<String> = dest_keys
            .iter()
            .map(|&c| key_text(&cell_ev(&view.dataset.columns()[c].data, r)))
            .collect();
        index.entry(k).or_default().push(r);
    }

    // The destination schema grows the source-only columns.
    let mut cols: Vec<Column> = view.dataset.columns().to_vec();
    for c in src.columns() {
        if position(&cols, &c.name).is_none() {
            cols.push(Column {
                name: c.name.clone(),
                data: blank_like(&c.data, view.dataset.row_count()),
            });
        }
    }

    let mut appends: Vec<Vec<EV>> = vec![];
    for r in 0..src.row_count() {
        let k: Vec<String> = src_keys
            .iter()
            .map(|&c| key_text(&cell_ev(&src.columns()[c].data, r)))
            .collect();
        let matched = index.get(&k).cloned().unwrap_or_default();
        if mode == MergeMode::Append || matched.is_empty() {
            if mode != MergeMode::Update {
                appends.push(src.columns().iter().map(|c| cell_ev(&c.data, r)).collect());
            }
            continue;
        }
        for d in matched {
            for scol in src.columns() {
                if let Some(dpos) = position(&cols, &scol.name) {
                    write_cell(&mut cols[dpos].data, d, &cell_ev(&scol.data, r));
                }
            }
        }
    }

    // Appended rows: shared + source-only cells copied, destination-
    // only cells missing.
    let napp = appends.len();
    for col in cols.iter_mut() {
        let sp = src
            .columns()
            .iter()
            .position(|c| c.name.eq_ignore_ascii_case(&col.name));
        for a in &appends {
            push_cell(
                &mut col.data,
                sp.map(|p| a[p].clone()).unwrap_or(EV::Missing),
            );
        }
    }

    view.dataset = rebuild(view, cols);

    // Appended indices go at the end of `order`; they stay unselected.
    if let Some(order) = &mut view.order {
        let start = order.len() as u32;
        order.extend(start..start + napp as u32);
    }
    Ok(())
}

fn col_index(ds: &Dataset, name: &str) -> Option<usize> {
    ds.columns()
        .iter()
        .position(|c| c.name.eq_ignore_ascii_case(name))
}

fn position(cols: &[Column], name: &str) -> Option<usize> {
    cols.iter().position(|c| c.name.eq_ignore_ascii_case(name))
}

/// The merge key match is textual — EI7 joins on the rendered cell
/// (`"1"` source = `"1"` destination; ints render identically).
fn key_text(v: &EV) -> String {
    if is_empty(v) {
        return String::new();
    }
    match v {
        EV::Real(x) if x.fract() == 0.0 && x.abs() < 1e15 => format!("{}", *x as i64),
        _ => ei_to_string(v),
    }
}

/// Numeric↔numeric and text↔text join; numeric↔text is
/// `merge_key_type` (int "1" never merges with text "1").
fn key_family(d: &ColumnData, s: &ColumnData) -> bool {
    use ColumnData::*;
    matches!(
        (d, s),
        (Integer(_), Integer(_))
            | (Real(_), Real(_))
            | (Integer(_), Real(_))
            | (Real(_), Integer(_))
            | (Text(_), Text(_))
            | (Boolean(_), Boolean(_))
            | (Date(_), Date(_))
            | (Time(_), Time(_))
            | (DateTime(_), DateTime(_))
            | (DateTime(_), Date(_))
            | (Date(_), DateTime(_))
    )
}

/// Writes `v` into `col`'s row `r`, coercing across the cell type the
/// column carries.
fn write_cell(col: &mut ColumnData, r: usize, v: &EV) {
    match col {
        ColumnData::Integer(c) => {
            c[r] = match v {
                EV::Int(x) => Some(*x),
                EV::Real(x) => Some(*x as i64),
                _ if is_empty(v) => None,
                _ => parse_f64(&ei_to_string(v)).map(|x| x as i64),
            }
        }
        ColumnData::Real(c) => {
            c[r] = match v {
                EV::Int(x) => Some(*x as f64),
                EV::Real(x) => Some(*x),
                _ if is_empty(v) => None,
                _ => as_f64(v),
            }
        }
        ColumnData::Text(c) => {
            c[r] = if is_empty(v) {
                None
            } else {
                Some(ei_to_string(v))
            }
        }
        ColumnData::Boolean(c) => {
            c[r] = match v {
                EV::Bool(b) => Some(*b),
                _ => None,
            }
        }
        ColumnData::Date(c) => {
            c[r] = match v {
                EV::Date(d) => Some(*d),
                EV::DateTime(s) => Some((*s / 86_400) as i32),
                _ => None,
            }
        }
        ColumnData::Time(c) => {
            c[r] = match v {
                EV::Time(t) => Some(*t),
                _ => None,
            }
        }
        ColumnData::DateTime(c) => {
            c[r] = match v {
                EV::Date(d) => Some(i64::from(*d) * 86_400),
                EV::DateTime(s) => Some(*s),
                _ => None,
            }
        }
    }
}

/// Appends one cell to the column — same coercion as `write_cell`.
fn push_cell(col: &mut ColumnData, v: EV) {
    match col {
        ColumnData::Integer(c) => c.push(match v {
            EV::Int(x) => Some(x),
            EV::Real(x) => Some(x as i64),
            _ => None,
        }),
        ColumnData::Real(c) => c.push(match v {
            EV::Int(x) => Some(x as f64),
            EV::Real(x) => Some(x),
            _ => None,
        }),
        ColumnData::Text(c) => c.push(if is_empty(&v) {
            None
        } else {
            Some(ei_to_string(&v))
        }),
        ColumnData::Boolean(c) => c.push(match v {
            EV::Bool(b) => Some(b),
            _ => None,
        }),
        ColumnData::Date(c) => c.push(match v {
            EV::Date(d) => Some(d),
            EV::DateTime(s) => Some((s / 86_400) as i32),
            _ => None,
        }),
        ColumnData::Time(c) => c.push(match v {
            EV::Time(t) => Some(t),
            _ => None,
        }),
        ColumnData::DateTime(c) => c.push(match v {
            EV::Date(d) => Some(i64::from(d) * 86_400),
            EV::DateTime(s) => Some(s),
            _ => None,
        }),
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

/// Rebuilds the view's dataset with the merged columns.
fn rebuild(view: &WorkingView, cols: Vec<Column>) -> Dataset {
    let mut meta: Vec<epiora_core::dataset::ColumnMeta> = view.dataset.meta().to_vec();
    while meta.len() < cols.len() {
        meta.push(epiora_core::dataset::ColumnMeta::default());
    }
    match Dataset::new(view.dataset.id(), view.dataset.version(), cols)
        .and_then(|d| d.with_meta(meta, view.dataset.label_sets().clone()))
    {
        Ok(d) => d,
        Err(_) => view.dataset.clone(),
    }
}
