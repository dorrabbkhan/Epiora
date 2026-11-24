//! `RECODE <from> TO <to> <rules> [ELSE = v] END` (spec §5.4):
//! boundaries are inclusive on both ends (`LOVALUE`/`HIVALUE`
//! unbounded), rules win in order, and a missing source cell always
//! lands missing — `ELSE` catches unmatched values only (MPGRP pins
//! a 74-row total, not 75). Target columns keep the rule's literal
//! types: `"well"`/`"ill"` stay lower-case (ASSIGN's TEXTINPUT
//! uppercase does not apply).

use super::define_assign::put_column;
use crate::expr::missing::{as_f64, cell_ev, ei_to_string, is_empty, EV};
use crate::expr::ops::same_family;
use crate::manage::{RecodeBound, RecodeRule};
use crate::view::WorkingView;
use crate::LangError;
use epiora_core::dataset::{ColumnData, Value, ValueType};

/// Executes the recode: evaluates the source column, then writes the
/// target cell per the first matching rule.
pub(crate) fn recode(
    view: &mut WorkingView,
    from: &str,
    to: &str,
    rules: &[RecodeRule],
    otherwise: Option<&Value>,
) -> Result<(), LangError> {
    let src = view
        .dataset
        .column(from)
        .map_err(|_| LangError::UnknownVariable {
            name: from.to_string(),
        })?
        .data
        .clone();
    let n = view.dataset.row_count();
    let mut out: Vec<EV> = (0..n).map(|_| EV::Missing).collect();
    for (r, cell_out) in out.iter_mut().enumerate() {
        let cell = cell_ev(&src, r);
        if is_empty(&cell) {
            continue; // missing stays missing, ELSE or not
        }
        let mut hit = None;
        for rule in rules {
            if rule_matches(&cell, rule) {
                hit = Some(rule.to().clone());
                break;
            }
        }
        *cell_out = match (hit, otherwise) {
            (Some(v), _) => value_ev(&v),
            (None, Some(v)) => value_ev(v),
            (None, None) => EV::Missing,
        };
    }
    let ty = target_type(view, to, rules, otherwise);
    put_column(view, to, evs_to_column(&out, ty));
    Ok(())
}

impl RecodeRule {
    fn to(&self) -> &Value {
        match self {
            RecodeRule::Single { to, .. } | RecodeRule::Range { to, .. } => to,
        }
    }
}

fn rule_matches(cell: &EV, rule: &RecodeRule) -> bool {
    match rule {
        RecodeRule::Single { from, .. } => ev_eq(cell, &value_ev(from)),
        RecodeRule::Range { lo, hi, .. } => ge_bound(cell, lo) && le_bound(cell, hi),
    }
}

/// Inclusive bounds; `LOVALUE`/`HIVALUE` always pass. Missing bounds
/// never match (a `(.)` literal bound).
fn ge_bound(cell: &EV, b: &RecodeBound) -> bool {
    match b {
        RecodeBound::LoValue => true,
        RecodeBound::HiValue => false,
        RecodeBound::Value(v) => ev_ge(cell, &value_ev(v)),
    }
}

fn le_bound(cell: &EV, b: &RecodeBound) -> bool {
    match b {
        RecodeBound::HiValue => true,
        RecodeBound::LoValue => false,
        RecodeBound::Value(v) => ev_le(cell, &value_ev(v)),
    }
}

/// `a = b` on the expression domain — numeric when both parse,
/// same-family ordering otherwise; text compares by its own ordering.
fn ev_eq(a: &EV, b: &EV) -> bool {
    if let (Some(x), Some(y)) = (as_f64(a), as_f64(b)) {
        return x == y;
    }
    if !same_family(a, b) {
        return ei_to_string(a).eq_ignore_ascii_case(&ei_to_string(b));
    }
    crate::expr::ops::compare(&crate::expr::ast::BinOp::Eq, a, b)
}

fn ev_ge(a: &EV, b: &EV) -> bool {
    match (as_f64(a), as_f64(b)) {
        (Some(x), Some(y)) => x >= y,
        _ => crate::expr::ops::compare(&crate::expr::ast::BinOp::Ge, a, b),
    }
}

fn ev_le(a: &EV, b: &EV) -> bool {
    match (as_f64(a), as_f64(b)) {
        (Some(x), Some(y)) => x <= y,
        _ => crate::expr::ops::compare(&crate::expr::ast::BinOp::Le, a, b),
    }
}

fn value_ev(v: &Value) -> EV {
    crate::expr::missing::value_ev(v)
}

/// The target column's type: the existing column's when it exists,
/// else the first non-`(.)` literal's type (text stays `Text`, so
/// ILLTXT's `"well"`/`"ill"` keep case).
fn target_type(
    view: &WorkingView,
    to: &str,
    rules: &[RecodeRule],
    otherwise: Option<&Value>,
) -> ValueType {
    if let Ok(c) = view.dataset.column(to) {
        return c.data.value_type();
    }
    match rules.iter().map(|r| r.to()).chain(otherwise).next() {
        Some(Value::Integer(_)) => ValueType::Integer,
        Some(Value::Real(_)) => ValueType::Real,
        Some(Value::Text(_)) => ValueType::Text,
        Some(Value::Boolean(_)) => ValueType::Boolean,
        Some(Value::Date(_)) => ValueType::Date,
        Some(Value::Time(_)) => ValueType::Time,
        Some(Value::DateTime(_)) => ValueType::DateTime,
        None => ValueType::Text,
    }
}

/// Builds the target column: values land in the column's type, a
/// numeric target taking ints and reals alike.
fn evs_to_column(evs: &[EV], ty: ValueType) -> ColumnData {
    macro_rules! col {
        ($variant:ident, $map:expr) => {
            ColumnData::$variant(evs.iter().map($map).collect())
        };
    }
    match ty {
        ValueType::Integer => col!(Integer, |v: &EV| match v {
            EV::Int(x) => Some(*x),
            EV::Real(x) => Some(*x as i64),
            _ => None,
        }),
        ValueType::Real => col!(Real, |v: &EV| match v {
            EV::Int(x) => Some(*x as f64),
            EV::Real(x) => Some(*x),
            _ => None,
        }),
        ValueType::Text => col!(Text, |v: &EV| match v {
            _ if is_empty(v) => None,
            _ => Some(ei_to_string(v)),
        }),
        ValueType::Boolean => col!(Boolean, |v: &EV| match v {
            EV::Bool(b) => Some(*b),
            _ => None,
        }),
        ValueType::Date => col!(Date, |v: &EV| match v {
            EV::Date(d) => Some(*d),
            _ => None,
        }),
        ValueType::Time => col!(Time, |v: &EV| match v {
            EV::Time(t) => Some(*t),
            _ => None,
        }),
        ValueType::DateTime => col!(DateTime, |v: &EV| match v {
            EV::Date(d) => Some(i64::from(*d) * 86_400),
            EV::DateTime(s) => Some(*s),
            _ => None,
        }),
    }
}
