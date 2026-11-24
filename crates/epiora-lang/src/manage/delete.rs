//! `DELETE <expr>` / `UNDELETE *|<expr>` / `DELETE VARIABLES` /
//! `KEYVARS` (spec §5.4): the deletion pair follows the table — a
//! `__recstatus` carrier writes marks (0) that PROCESS filters and
//! UNDELETE can clear, while a raw table loses the rows outright and
//! UNDELETE restores nothing (the corpus pins `records_restored = 0`).

use super::define_assign::remove_column;
use super::eval_mask;
use crate::expr::ast::Expr;
use crate::expr::eval::EvalEnv;
use crate::view::{gather, WorkingView};
use crate::LangError;
use epiora_core::dataset::{Column, ColumnData, Dataset};

/// `DELETE <expr>` — marks or removes the matching rows.
pub(crate) fn delete_records(
    view: &mut WorkingView,
    env: &EvalEnv,
    e: &Expr,
) -> Result<(), LangError> {
    let mask = eval_mask(view, env, e)?;
    if let Some(ci) = view.recstatus_col() {
        let cols: Vec<Column> = view
            .dataset
            .columns()
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let mut data = c.data.clone();
                if i == ci {
                    if let ColumnData::Integer(v) = &mut data {
                        for (r, m) in mask.iter().enumerate() {
                            if *m {
                                v[r] = Some(0);
                            }
                        }
                    }
                }
                Column {
                    name: c.name.clone(),
                    data,
                }
            })
            .collect();
        rebuild(view, cols);
        return Ok(());
    }
    // Raw table: the rows leave the view for good.
    let keep: Vec<u32> = (0..view.dataset.row_count() as u32)
        .filter(|&i| !mask[i as usize])
        .collect();
    let removed = (view.dataset.row_count() - keep.len()) as u64;
    view.settings.physically_deleted += removed;

    let cols: Vec<Column> = view
        .dataset
        .columns()
        .iter()
        .map(|c| Column {
            name: c.name.clone(),
            data: gather(&c.data, &keep),
        })
        .collect();
    let mut meta: Vec<epiora_core::dataset::ColumnMeta> = view.dataset.meta().to_vec();
    while meta.len() < cols.len() {
        meta.push(epiora_core::dataset::ColumnMeta::default());
    }
    // Remap selection and order through the kept positions.
    let mut remap = vec![u32::MAX; view.dataset.row_count()];
    for (new, &old) in keep.iter().enumerate() {
        remap[old as usize] = new as u32;
    }
    view.selected = view.selected.take().map(|s| {
        s.iter()
            .filter_map(|&i| (remap[i as usize] != u32::MAX).then_some(remap[i as usize]))
            .collect()
    });
    view.order = view.order.take().map(|o| {
        o.iter()
            .filter_map(|&i| (remap[i as usize] != u32::MAX).then_some(remap[i as usize]))
            .collect()
    });
    if let Ok(d) = Dataset::new(view.dataset.id(), view.dataset.version(), cols)
        .and_then(|d| d.with_meta(meta, view.dataset.label_sets().clone()))
    {
        view.dataset = d;
    }
    Ok(())
}

/// `UNDELETE *|<expr>` — clears marks; on a raw table nothing comes
/// back (EI7 restores `0`).
pub(crate) fn undelete(
    view: &mut WorkingView,
    env: &EvalEnv,
    e: Option<&Expr>,
) -> Result<(), LangError> {
    let Some(ci) = view.recstatus_col() else {
        return Ok(());
    };
    let mask = match e {
        Some(e) => eval_mask(view, env, e)?,
        None => vec![true; view.dataset.row_count()],
    };
    let cols: Vec<Column> = view
        .dataset
        .columns()
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut data = c.data.clone();
            if i == ci {
                if let ColumnData::Integer(v) = &mut data {
                    for (r, m) in mask.iter().enumerate() {
                        if *m && v[r] == Some(0) {
                            v[r] = Some(1);
                        }
                    }
                }
            }
            Column {
                name: c.name.clone(),
                data,
            }
        })
        .collect();
    rebuild(view, cols);
    Ok(())
}

/// Rebuilds the view's dataset preserving id/version/meta.
fn rebuild(view: &mut WorkingView, cols: Vec<Column>) {
    let mut meta: Vec<epiora_core::dataset::ColumnMeta> = view.dataset.meta().to_vec();
    while meta.len() < cols.len() {
        meta.push(epiora_core::dataset::ColumnMeta::default());
    }
    if let Ok(d) = Dataset::new(view.dataset.id(), view.dataset.version(), cols)
        .and_then(|d| d.with_meta(meta, view.dataset.label_sets().clone()))
    {
        view.dataset = d;
    }
}

/// `DELETE VARIABLES <name> ...` — drops the columns.
pub(crate) fn delete_variables(view: &mut WorkingView, names: &[String]) -> Result<(), LangError> {
    for n in names {
        if view.dataset.column(n).is_err() {
            return Err(LangError::UnknownVariable { name: n.clone() });
        }
    }
    for n in names {
        remove_column(view, n);
        view.defined.remove(&n.to_ascii_lowercase());
    }
    Ok(())
}

/// `KEYVARS <name> ...`.
pub(crate) fn key_vars(view: &mut WorkingView, names: &[String]) -> Result<(), LangError> {
    view.key_vars = names.to_vec();
    Ok(())
}
