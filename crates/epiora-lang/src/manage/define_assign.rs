//! `DEFINE` / `DEFINE = expr` / `ASSIGN` / `UNDEFINE` / `IF` (spec
//! §5.4): the variable commands. EI7's two DEFINE forms differ in
//! what the column holds — `DEFINE x NUMERIC` an empty column,
//! `DEFINE x = <expr>` a zero-filled column of the expression's type
//! (the corpus's `MEANS DOUBLED` over `AGE + AGE` reports a total of
//! 0 over 75 observations). `ASSIGN` into a TEXTINPUT-defined column
//! uppercases text (`"yes"` lands as `YES`).

use super::eval_mask;
use crate::expr::ast::Expr;
use crate::expr::eval::{eval_column, EvalEnv};
use crate::manage::{Command, Scope};
use crate::view::{DefinedVar, WorkingView};
use crate::LangError;
use epiora_core::dataset::{Column, ColumnData, Dataset, ValueType};

fn lname(name: &str) -> String {
    name.to_ascii_lowercase()
}

/// `DEFINE <name> <type> [<scope>]` — a missing-filled typed column.
pub(crate) fn define(
    view: &mut WorkingView,
    name: &str,
    ty: ValueType,
    _scope: Scope,
) -> Result<(), LangError> {
    let n = view.dataset.row_count();
    let data = match ty {
        ValueType::Integer => ColumnData::Integer(vec![None; n]),
        ValueType::Real => ColumnData::Real(vec![None; n]),
        ValueType::Text => ColumnData::Text(vec![None; n]),
        ValueType::Boolean => ColumnData::Boolean(vec![None; n]),
        ValueType::Date => ColumnData::Date(vec![None; n]),
        ValueType::Time => ColumnData::Time(vec![None; n]),
        ValueType::DateTime => ColumnData::DateTime(vec![None; n]),
    };
    put_column(view, name, data);
    view.defined.insert(
        lname(name),
        DefinedVar {
            text_input: ty == ValueType::Text,
        },
    );
    Ok(())
}

/// `DEFINE <name> = <expr>` — a column of the expression's type
/// filled with its zero/empty value; the expression is not evaluated
/// (EI7 initializes the var, it does not compute it).
pub(crate) fn define_expr(
    view: &mut WorkingView,
    _env: &EvalEnv,
    name: &str,
    _expr: &Expr,
    ty: ValueType,
    _scope: Scope,
) -> Result<(), LangError> {
    let n = view.dataset.row_count();
    let data = match ty {
        ValueType::Integer => ColumnData::Integer(vec![Some(0); n]),
        ValueType::Real => ColumnData::Real(vec![Some(0.0); n]),
        ValueType::Text => ColumnData::Text(vec![Some(String::new()); n]),
        ValueType::Boolean => ColumnData::Boolean(vec![Some(false); n]),
        ValueType::Date => ColumnData::Date(vec![Some(0); n]),
        ValueType::Time => ColumnData::Time(vec![Some(0); n]),
        ValueType::DateTime => ColumnData::DateTime(vec![Some(0); n]),
    };
    put_column(view, name, data);
    view.defined.insert(
        lname(name),
        DefinedVar {
            text_input: ty == ValueType::Text,
        },
    );
    Ok(())
}

/// `UNDEFINE <name>` — the column and its bookkeeping leave.
pub(crate) fn undefine(view: &mut WorkingView, name: &str) -> Result<(), LangError> {
    if view.dataset.column(name).is_err() {
        return Err(LangError::UnknownVariable {
            name: name.to_string(),
        });
    }
    remove_column(view, name);
    view.defined.remove(&lname(name));
    Ok(())
}

/// `ASSIGN <name> = <expr>` — evaluates the expression into the
/// column (creating it when absent). TEXTINPUT's uppercase applies
/// only to the *immediate* path EI7 takes when the expression holds
/// no field reference (`"yes"` → `YES`); an assign through field
/// data keeps case (`SUBSTRING(Name,1,3)` → `Doe`).
pub(crate) fn assign(
    view: &mut WorkingView,
    env: &EvalEnv,
    name: &str,
    expr: &Expr,
) -> Result<(), LangError> {
    let mut data = eval_column(expr, &view.dataset, env)?;
    if !has_column_ref(expr, view) && view.defined.get(&lname(name)).is_some_and(|d| d.text_input) {
        data = upper_text(data);
    }
    // A declared variable's type does not change (EI7: an attempted
    // type change is an error); coerce the result into the existing
    // column's type (`DEFINE P NUMERIC` + `ASSIGN P = 42` stays Real).
    if let Ok(existing) = view.dataset.column(name) {
        if std::mem::discriminant(&existing.data) != std::mem::discriminant(&data) {
            data = retype_into(&data, &existing.data);
        }
    }
    put_column(view, name, data);
    Ok(())
}

/// `data` re-expressed in `target`'s column type — numeric widening,
/// everything else through `set_cell`'s per-arm conversions
/// (non-convertible cells land as missing).
fn retype_into(data: &ColumnData, target: &ColumnData) -> ColumnData {
    let n = view_rows(data);
    let mut out = blank_like(target, n);
    for r in 0..n {
        set_cell(&mut out, r, crate::expr::missing::cell_ev(data, r));
    }
    out
}

fn view_rows(data: &ColumnData) -> usize {
    match data {
        ColumnData::Integer(v) => v.len(),
        ColumnData::Real(v) => v.len(),
        ColumnData::Text(v) => v.len(),
        ColumnData::Boolean(v) => v.len(),
        ColumnData::Date(v) => v.len(),
        ColumnData::Time(v) => v.len(),
        ColumnData::DateTime(v) => v.len(),
    }
}

/// `true` when `expr` names a dataset column anywhere — the ASSIGN
/// then runs per-record on EI7's field-dependent path.
fn has_column_ref(e: &Expr, view: &WorkingView) -> bool {
    match e {
        Expr::Ident(name) => view.dataset.column(name).is_ok(),
        Expr::Literal(_) => false,
        Expr::Unary { operand, .. } | Expr::Like { operand, .. } => has_column_ref(operand, view),
        Expr::Binary { lhs, rhs, .. } => has_column_ref(lhs, view) || has_column_ref(rhs, view),
        Expr::Call { args, .. } => args.iter().any(|a| has_column_ref(a, view)),
        Expr::List(xs) => xs.iter().any(|a| has_column_ref(a, view)),
    }
}

/// `IF <cond> THEN <assigns> [ELSE <assigns>] END` — evaluates each
/// side's assignments over the whole column, then picks per row.
/// Rows a no-ELSE IF doesn't select keep the prior cell (missing for
/// a fresh variable — BIGAGE pins that propagation).
pub(crate) fn if_then(
    view: &mut WorkingView,
    env: &EvalEnv,
    cond: &Expr,
    then: &[Command],
    otherwise: &[Command],
) -> Result<(), LangError> {
    let mask = eval_mask(view, env, cond)?;
    let mut then_vals = Vec::new();
    for c in then {
        match c {
            Command::Assign { name, expr } => {
                then_vals.push((name.clone(), eval_column(expr, &view.dataset, env)?))
            }
            _ => {
                return Err(LangError::ExprType {
                    column: "IF".into(),
                    expected: "ASSIGN".into(),
                    found: "command".into(),
                })
            }
        }
    }
    let mut else_vals = Vec::new();
    for c in otherwise {
        match c {
            Command::Assign { name, expr } => {
                else_vals.push((name.clone(), eval_column(expr, &view.dataset, env)?))
            }
            _ => {
                return Err(LangError::ExprType {
                    column: "IF".into(),
                    expected: "ASSIGN".into(),
                    found: "command".into(),
                })
            }
        }
    }
    // Each target column merges the two branches per row.
    let mut names: Vec<String> = then_vals.iter().map(|(n, _)| n.clone()).collect();
    for (n, _) in &else_vals {
        if !names.iter().any(|x| x.eq_ignore_ascii_case(n)) {
            names.push(n.clone());
        }
    }
    let n = view.dataset.row_count();
    for name in names {
        let tv = then_vals
            .iter()
            .find(|(x, _)| x.eq_ignore_ascii_case(&name))
            .map(|(_, v)| v.clone());
        let ev = else_vals
            .iter()
            .find(|(x, _)| x.eq_ignore_ascii_case(&name))
            .map(|(_, v)| v.clone());
        let base = view
            .dataset
            .column(&name)
            .ok()
            .map(|c| c.data.clone())
            .or_else(|| tv.clone().or_else(|| ev.clone()).map(|d| blank_like(&d, n)));
        let mut data = base.unwrap_or_else(|| ColumnData::Real(vec![None; n]));
        for (r, &m) in mask.iter().enumerate() {
            let pick = if m { tv.as_ref() } else { ev.as_ref() };
            if let Some(src) = pick {
                set_cell(&mut data, r, crate::expr::missing::cell_ev(src, r));
            }
        }
        let field_path = then.iter().chain(otherwise.iter()).any(|c| match c {
            Command::Assign { name: n, expr } => {
                n.eq_ignore_ascii_case(&name) && has_column_ref(expr, view)
            }
            _ => false,
        });
        if !field_path
            && view
                .defined
                .get(&lname(&name))
                .is_some_and(|d| d.text_input)
        {
            data = upper_text(data);
        }
        put_column(view, &name, data);
    }
    Ok(())
}

/// A missing-filled column of the same type — the untouched branch's
/// base when the target didn't exist.
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

/// Writes `v` into `data`'s row `r`, extending the column's type when
/// the value needs one the column doesn't have (the whole column is
/// rebuilt by the caller's per-type arm).
fn set_cell(data: &mut ColumnData, r: usize, v: crate::expr::missing::EV) {
    use crate::expr::missing::EV;
    match (data, v) {
        (ColumnData::Integer(c), EV::Int(x)) => c[r] = Some(x),
        (ColumnData::Integer(c), EV::Real(x)) => c[r] = Some(x as i64),
        (ColumnData::Real(c), EV::Int(x)) => c[r] = Some(x as f64),
        (ColumnData::Real(c), EV::Real(x)) => c[r] = Some(x),
        (ColumnData::Text(c), EV::Text(s)) => c[r] = Some(s),
        (ColumnData::Text(c), other) => {
            c[r] = if is_missing(&other) {
                None
            } else {
                Some(crate::expr::missing::ei_to_string(&other))
            }
        }
        (ColumnData::Boolean(c), EV::Bool(b)) => c[r] = Some(b),
        (ColumnData::Date(c), EV::Date(d)) => c[r] = Some(d),
        (ColumnData::Date(c), EV::DateTime(s)) => c[r] = Some((s / 86_400) as i32),
        (ColumnData::Time(c), EV::Time(t)) => c[r] = Some(t),
        (ColumnData::DateTime(c), EV::DateTime(s)) => c[r] = Some(s),
        (ColumnData::DateTime(c), EV::Date(d)) => c[r] = Some(i64::from(d) * 86_400),
        (col, v) => {
            let miss = is_missing(&v);
            match col {
                ColumnData::Integer(c) => c[r] = if miss { None } else { c[r] },
                ColumnData::Real(c) => c[r] = if miss { None } else { c[r] },
                ColumnData::Text(c) => c[r] = if miss { None } else { c[r].clone() },
                ColumnData::Boolean(c) => c[r] = if miss { None } else { c[r] },
                ColumnData::Date(c) => c[r] = if miss { None } else { c[r] },
                ColumnData::Time(c) => c[r] = if miss { None } else { c[r] },
                ColumnData::DateTime(c) => c[r] = if miss { None } else { c[r] },
            }
        }
    }
}

fn is_missing(v: &crate::expr::missing::EV) -> bool {
    matches!(v, crate::expr::missing::EV::Missing)
        || matches!(v, crate::expr::missing::EV::Text(s) if s.is_empty())
}

/// Uppercases a text column's cells (TEXTINPUT semantics).
fn upper_text(data: ColumnData) -> ColumnData {
    match data {
        ColumnData::Text(v) => ColumnData::Text(
            v.iter()
                .map(|x| x.as_ref().map(|s| s.to_ascii_uppercase()))
                .collect(),
        ),
        other => other,
    }
}

/// Inserts or replaces a column, preserving row length.
pub(crate) fn put_column(view: &mut WorkingView, name: &str, data: ColumnData) {
    let cols: Vec<Column> = view
        .dataset
        .columns()
        .iter()
        .map(|c| {
            if c.name.eq_ignore_ascii_case(name) {
                Column {
                    name: c.name.clone(),
                    data: data.clone(),
                }
            } else {
                c.clone()
            }
        })
        .collect();
    let replaced = view
        .dataset
        .columns()
        .iter()
        .any(|c| c.name.eq_ignore_ascii_case(name));
    let cols = if replaced {
        cols
    } else {
        let mut c = cols;
        c.push(Column {
            name: name.to_string(),
            data,
        });
        c
    };
    let mut meta: Vec<epiora_core::dataset::ColumnMeta> = view.dataset.meta().to_vec();
    let sets = view.dataset.label_sets().clone();
    while meta.len() < cols.len() {
        meta.push(epiora_core::dataset::ColumnMeta::default());
    }
    let id = view.dataset.id().to_string();
    let version = view.dataset.version();
    if let Ok(d) = Dataset::new(&id, version, cols).and_then(|d| d.with_meta(meta, sets)) {
        view.dataset = d;
    }
}

/// Removes a column by name (case-insensitive).
pub(crate) fn remove_column(view: &mut WorkingView, name: &str) {
    let cols: Vec<Column> = view
        .dataset
        .columns()
        .iter()
        .filter(|c| !c.name.eq_ignore_ascii_case(name))
        .cloned()
        .collect();
    let meta: Vec<epiora_core::dataset::ColumnMeta> = view
        .dataset
        .columns()
        .iter()
        .enumerate()
        .filter(|(_, c)| !c.name.eq_ignore_ascii_case(name))
        .filter_map(|(i, _)| view.dataset.meta().get(i).cloned())
        .collect();
    let id = view.dataset.id().to_string();
    let version = view.dataset.version();
    let meta = if meta.len() == cols.len() {
        meta
    } else {
        vec![epiora_core::dataset::ColumnMeta::default(); cols.len()]
    };
    if let Ok(d) = Dataset::new(&id, version, cols)
        .and_then(|d| d.with_meta(meta, view.dataset.label_sets().clone()))
    {
        view.dataset = d;
    }
}
