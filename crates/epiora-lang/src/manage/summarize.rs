//! `SUMMARIZE <name>::<agg>(<arg>) ... TO <table> [STRATAVAR=<v>]`
//! (spec §5.4): groups the view's rows by the strata columns and
//! writes the grouped table — it becomes the working view's dataset.

use crate::expr::eval::EvalEnv;
use crate::expr::missing::{as_f64, cell_ev, is_empty, EV};
use crate::manage::{AggFunc, Aggregate};
use crate::view::{DatasetRef, WorkingView};
use crate::LangError;
use epiora_core::dataset::{Column, ColumnData, Dataset};
use std::collections::BTreeMap;

/// Groups the materialized view's rows and writes `into`.
pub(crate) fn summarize(
    view: &mut WorkingView,
    _env: &EvalEnv,
    aggregates: &[Aggregate],
    by: &[String],
    into: &DatasetRef,
) -> Result<(), LangError> {
    let src = view.materialize();
    let keys: Vec<usize> = by
        .iter()
        .map(|k| {
            src.columns()
                .iter()
                .position(|c| c.name.eq_ignore_ascii_case(k))
                .ok_or_else(|| LangError::UnknownVariable { name: k.clone() })
        })
        .collect::<Result<_, _>>()?;
    let args: Vec<Option<usize>> = aggregates
        .iter()
        .map(|a| {
            a.arg
                .as_ref()
                .map(|name| {
                    src.columns()
                        .iter()
                        .position(|c| c.name.eq_ignore_ascii_case(name))
                        .ok_or_else(|| LangError::UnknownVariable { name: name.clone() })
                })
                .transpose()
        })
        .collect::<Result<_, _>>()?;

    // Group row indices by their strata key (first-seen order).
    let mut groups: Vec<(Vec<EV>, Vec<usize>)> = vec![];
    let mut lookup: BTreeMap<Vec<String>, usize> = BTreeMap::new();
    for r in 0..src.row_count() {
        let evs: Vec<EV> = keys
            .iter()
            .map(|&c| cell_ev(&src.columns()[c].data, r))
            .collect();
        let key: Vec<String> = evs.iter().map(key_text).collect();
        match lookup.get(&key) {
            Some(&g) => groups[g].1.push(r),
            None => {
                lookup.insert(key, groups.len());
                groups.push((evs, vec![r]));
            }
        }
    }

    let mut cols: Vec<Column> = by
        .iter()
        .enumerate()
        .map(|(i, k)| Column {
            name: k.clone(),
            data: evs_to_column(
                &groups.iter().map(|g| g.0[i].clone()).collect::<Vec<_>>(),
                src.columns()[keys[i]].data.value_type(),
            ),
        })
        .collect();
    for (a, &arg) in aggregates.iter().zip(args.iter()) {
        let (_ty, data) = aggregate(&src, &groups, a, arg);
        cols.push(Column {
            name: a.name.clone(),
            data,
        });
    }

    let out = match Dataset::new(&into.name, 1, cols) {
        Ok(d) => d,
        Err(e) => {
            return Err(LangError::ExprType {
                column: into.name.clone(),
                expected: "dataset".into(),
                found: e.to_string(),
            })
        }
    };
    view.dataset = out;
    view.selected = None;
    view.order = None;
    Ok(())
}

fn key_text(v: &EV) -> String {
    if is_empty(v) {
        return String::new();
    }
    crate::expr::missing::ei_to_string(v)
}

/// One aggregate over its groups → (output type, column).
fn aggregate(
    src: &Dataset,
    groups: &[(Vec<EV>, Vec<usize>)],
    a: &Aggregate,
    arg: Option<usize>,
) -> (epiora_core::dataset::ValueType, ColumnData) {
    use epiora_core::dataset::ValueType::*;
    let cells = |rows: &[usize]| -> Vec<EV> {
        match arg {
            Some(c) => rows
                .iter()
                .map(|&r| cell_ev(&src.columns()[c].data, r))
                .collect(),
            None => vec![],
        }
    };
    let nums = |rows: &[usize]| -> Vec<f64> { cells(rows).iter().filter_map(as_f64).collect() };
    match a.func {
        AggFunc::Count => (
            Integer,
            ColumnData::Integer(groups.iter().map(|(_, r)| Some(r.len() as i64)).collect()),
        ),
        AggFunc::Sum => (
            Real,
            ColumnData::Real(
                groups
                    .iter()
                    .map(|(_, r)| Some(nums(r).iter().sum()))
                    .collect(),
            ),
        ),
        AggFunc::Avg => (
            Real,
            ColumnData::Real(
                groups
                    .iter()
                    .map(|(_, r)| {
                        let n = nums(r);
                        (!n.is_empty()).then(|| n.iter().sum::<f64>() / n.len() as f64)
                    })
                    .collect(),
            ),
        ),
        AggFunc::Min | AggFunc::Max | AggFunc::First | AggFunc::Last => {
            let ty = arg
                .map(|c| src.columns()[c].data.value_type())
                .unwrap_or(Real);
            let pick = |rows: &[usize]| -> EV {
                let cs = cells(rows);
                let ord = |a: &EV, b: &EV| -> std::cmp::Ordering {
                    match (as_f64(a), as_f64(b)) {
                        (Some(x), Some(y)) => {
                            x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        _ => crate::expr::ops::same_ordering(a, b).cmp(&0),
                    }
                };
                match a.func {
                    AggFunc::Min => cs
                        .iter()
                        .filter(|v| !is_empty(v))
                        .min_by(|a, b| ord(a, b))
                        .cloned()
                        .unwrap_or(EV::Missing),
                    AggFunc::Max => cs
                        .iter()
                        .filter(|v| !is_empty(v))
                        .max_by(|a, b| ord(a, b))
                        .cloned()
                        .unwrap_or(EV::Missing),
                    AggFunc::First => cs.first().cloned().unwrap_or(EV::Missing),
                    AggFunc::Last => cs.last().cloned().unwrap_or(EV::Missing),
                    _ => EV::Missing,
                }
            };
            (
                ty,
                evs_to_column(&groups.iter().map(|(_, r)| pick(r)).collect::<Vec<_>>(), ty),
            )
        }
        AggFunc::Stdev | AggFunc::Variance => (
            Real,
            ColumnData::Real(
                groups
                    .iter()
                    .map(|(_, r)| {
                        let n = nums(r);
                        if n.len() < 2 {
                            return None;
                        }
                        let mean = n.iter().sum::<f64>() / n.len() as f64;
                        let var = n.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>()
                            / (n.len() - 1) as f64;
                        Some(if a.func == AggFunc::Stdev {
                            var.sqrt()
                        } else {
                            var
                        })
                    })
                    .collect(),
            ),
        ),
    }
}

/// EVs → a typed column (mirrors recode's coercion for the by cols
/// and FIRST/LAST).
fn evs_to_column(evs: &[EV], ty: epiora_core::dataset::ValueType) -> ColumnData {
    use epiora_core::dataset::ValueType::*;
    match ty {
        Integer => ColumnData::Integer(
            evs.iter()
                .map(|v| match v {
                    EV::Int(x) => Some(*x),
                    EV::Real(x) => Some(*x as i64),
                    _ => None,
                })
                .collect(),
        ),
        Real => ColumnData::Real(
            evs.iter()
                .map(|v| match v {
                    EV::Int(x) => Some(*x as f64),
                    EV::Real(x) => Some(*x),
                    _ => None,
                })
                .collect(),
        ),
        Text => ColumnData::Text(
            evs.iter()
                .map(|v| {
                    if is_empty(v) {
                        None
                    } else {
                        Some(crate::expr::missing::ei_to_string(v))
                    }
                })
                .collect(),
        ),
        Boolean => ColumnData::Boolean(
            evs.iter()
                .map(|v| match v {
                    EV::Bool(b) => Some(*b),
                    _ => None,
                })
                .collect(),
        ),
        Date => ColumnData::Date(
            evs.iter()
                .map(|v| match v {
                    EV::Date(d) => Some(*d),
                    _ => None,
                })
                .collect(),
        ),
        Time => ColumnData::Time(
            evs.iter()
                .map(|v| match v {
                    EV::Time(t) => Some(*t),
                    _ => None,
                })
                .collect(),
        ),
        DateTime => ColumnData::DateTime(
            evs.iter()
                .map(|v| match v {
                    EV::Date(d) => Some(i64::from(*d) * 86_400),
                    EV::DateTime(s) => Some(*s),
                    _ => None,
                })
                .collect(),
        ),
    }
}
