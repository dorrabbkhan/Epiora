//! `groups` — the one-pass grouping behind MEANS (v1.0 spec §4.2):
//! the numeric variable's values bucketed by the cross-tabulation
//! variable's categories, in v0.2 category order, with `"(missing)"`
//! last when `include_missing`. A record missing the variable is
//! always excluded; one missing only the group is excluded unless it
//! forms the `"(missing)"` group. Excluded records are counted under
//! every variable they are missing (spec §4.1), as for TABLES.

use super::categories::{collect, Cats};
use crate::dataset::{ColumnData, Dataset};
use crate::procedures::MAX_MEANS_GROUPS;
use crate::EngineError;
use std::collections::BTreeMap;

/// The label of the implicit single group of an ungrouped run.
const ALL_LABEL: &str = "(all)";

/// The grouped numeric values of one MEANS run.
#[derive(Debug, Clone)]
pub struct Groups {
    /// Group labels in category order; `"(missing)"` last; `"(all)"`
    /// for an ungrouped run.
    pub labels: Vec<String>,
    /// Per-group values in data order, aligned with `labels`; every
    /// group holds at least one value.
    pub values: Vec<Vec<f64>>,
    /// Every kept value in data order (the overall block's input).
    pub all: Vec<f64>,
    /// Excluded records per used variable.
    pub exclusions: BTreeMap<String, u64>,
}

fn numeric(data: &ColumnData, i: usize) -> Option<f64> {
    match data {
        ColumnData::Integer(v) => v[i].map(|x| x as f64),
        ColumnData::Real(v) => v[i],
        ColumnData::Date(v) => v[i].map(f64::from),
        ColumnData::Text(_) | ColumnData::Boolean(_) => None,
    }
}

fn is_numeric(data: &ColumnData) -> bool {
    matches!(
        data,
        ColumnData::Integer(_) | ColumnData::Real(_) | ColumnData::Date(_)
    )
}

/// Buckets `variable`'s values by `group_by` (v1.0 spec §4.2). Dates
/// convert to days since 1970-01-01. `NotNumericVariable` on a Text or
/// Boolean analysis variable; `TooManyGroups` above
/// [`MAX_MEANS_GROUPS`] distinct categories.
pub fn group_numeric(
    ds: &Dataset,
    variable: &str,
    group_by: Option<&str>,
    include_missing: bool,
) -> Result<Groups, EngineError> {
    let vdata = &ds.column(variable)?.data;
    if !is_numeric(vdata) {
        return Err(EngineError::NotNumericVariable {
            name: variable.to_string(),
        });
    }
    let mut exclusions: BTreeMap<String, u64> = [(variable.to_string(), 0)]
        .into_iter()
        .chain(group_by.map(|g| (g.to_string(), 0)))
        .collect();

    let Some(group) = group_by else {
        let mut all = Vec::new();
        for i in 0..ds.row_count() {
            match numeric(vdata, i) {
                Some(x) => all.push(x),
                None => *exclusions.entry(variable.to_string()).or_default() += 1,
            }
        }
        return Ok(Groups {
            labels: vec![ALL_LABEL.to_string()],
            values: vec![all.clone()],
            all,
            exclusions,
        });
    };

    let gdata = &ds.column(group)?.data;
    let (keys, missing) = collect(gdata);
    let cats = Cats::build(&keys, missing, gdata, include_missing);
    let mut values: Vec<Vec<f64>> = (0..cats.labels.len()).map(|_| Vec::new()).collect();
    let mut all = Vec::new();
    for i in 0..ds.row_count() {
        let xv = numeric(vdata, i);
        let gi = match cats.idx(gdata, i) {
            Some(x) => Some(x),
            None if include_missing => cats.missing_idx,
            None => None,
        };
        match (xv, gi) {
            (Some(x), Some(g)) => {
                values[g].push(x);
                all.push(x);
            }
            _ => {
                if xv.is_none() {
                    *exclusions.entry(variable.to_string()).or_default() += 1;
                }
                if gi.is_none() {
                    *exclusions.entry(group.to_string()).or_default() += 1;
                }
            }
        }
    }

    // Categories left without a kept value vanish, as R's
    // `sort(unique(gs[keep]))` does — only non-empty groups emit.
    let mut labels = Vec::new();
    let mut kept: Vec<Vec<f64>> = Vec::new();
    for (label, group_values) in cats.labels.iter().zip(values) {
        if group_values.is_empty() {
            continue;
        }
        labels.push(label.clone());
        kept.push(group_values);
    }
    let levels = kept.len() as u64;
    if levels > MAX_MEANS_GROUPS as u64 {
        return Err(EngineError::TooManyGroups {
            name: group.to_string(),
            levels,
        });
    }
    Ok(Groups {
        labels,
        values: kept,
        all,
        exclusions,
    })
}
