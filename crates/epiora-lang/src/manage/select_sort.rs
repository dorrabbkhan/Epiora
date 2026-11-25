//! `SELECT` / `CANCEL SELECT` and `SORT` / `CANCEL SORT` (spec §5.4):
//! the row set and order the view presents. A second SELECT intersects
//! the mask — select_sort.pgm pins `ILL = 1` then `AGE > 40` keeping 22
//! rows, not 38. SORT is a stable multi-key order with missing cells
//! first ascending (`SORT TIMESUPPER` leads with `(.)` rows).

use crate::expr::ast::Expr;
use crate::expr::eval::{eval_column, EvalEnv};
use crate::expr::missing::{cell_ev, is_empty, truthy, EV};
use crate::expr::ops::same_ordering;
use crate::view::{Order, WorkingView};
use crate::LangError;
use std::cmp::Ordering;

/// `eval_column`'s result as per-row booleans — shared by SELECT,
/// DELETE/UNDELETE and IF.
pub(crate) fn eval_mask(
    view: &WorkingView,
    env: &EvalEnv,
    e: &Expr,
) -> Result<Vec<bool>, LangError> {
    let col = eval_column(e, &view.dataset, env)?;
    Ok((0..view.dataset.row_count())
        .map(|r| truthy(&cell_ev(&col, r)))
        .collect())
}

/// `SELECT <expr>` — keeps the passing rows among the already
/// selected ones.
pub(crate) fn select(view: &mut WorkingView, env: &EvalEnv, e: &Expr) -> Result<(), LangError> {
    let mask = eval_mask(view, env, e)?;
    let keep: Vec<u32> = match &view.selected {
        Some(prev) => prev.iter().copied().filter(|&i| mask[i as usize]).collect(),
        None => (0..view.dataset.row_count() as u32)
            .filter(|&i| mask[i as usize])
            .collect(),
    };
    view.selected = Some(keep);
    Ok(())
}

pub(crate) fn cancel_select(view: &mut WorkingView) -> Result<(), LangError> {
    view.selected = None;
    Ok(())
}

pub(crate) fn sort(view: &mut WorkingView, keys: &[(String, Order)]) -> Result<(), LangError> {
    let cols: Result<Vec<_>, LangError> = keys
        .iter()
        .map(|(name, _)| {
            view.dataset
                .column(name)
                .map(|c| c.data.clone())
                .map_err(|_| LangError::UnknownVariable { name: name.clone() })
        })
        .collect();
    let cols = cols?;
    let n = view.dataset.row_count();
    let mut perm: Vec<u32> = (0..n as u32).collect();
    perm.sort_by(|&a, &b| {
        for (col, (_, ord)) in cols.iter().zip(keys.iter()) {
            let c = cmp_cell(&cell_ev(col, a as usize), &cell_ev(col, b as usize), *ord);
            if c != Ordering::Equal {
                return c;
            }
        }
        Ordering::Equal
    });
    view.order = Some(perm);
    Ok(())
}

pub(crate) fn cancel_sort(view: &mut WorkingView) -> Result<(), LangError> {
    view.order = None;
    Ok(())
}

/// EI7's sort order: missing first ascending, last descending; within
/// values the expression domain's `same_ordering` (numbers, trimmed
/// text, instants), integers before reals harmonized by `as_f64`.
fn cmp_cell(a: &EV, b: &EV, ord: Order) -> Ordering {
    match (is_empty(a), is_empty(b)) {
        (true, true) => Ordering::Equal,
        (true, false) => {
            if ord == Order::Asc {
                Ordering::Less
            } else {
                Ordering::Greater
            }
        }
        (false, true) => {
            if ord == Order::Asc {
                Ordering::Greater
            } else {
                Ordering::Less
            }
        }
        (false, false) => {
            let c = match (
                crate::expr::missing::as_f64(a),
                crate::expr::missing::as_f64(b),
            ) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
                _ => same_ordering(a, b).cmp(&0),
            };
            if ord == Order::Asc {
                c
            } else {
                c.reverse()
            }
        }
    }
}
