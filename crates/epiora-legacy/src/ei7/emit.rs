//! `OutCol` → `Dataset`: typed `ColumnData` assembly, label-set
//! distribution and the final `Dataset::new().with_meta()` build
//! (the golden's emit step).

use std::collections::{BTreeMap, HashSet};

use epiora_core::dataset::{Column, ColumnData, ColumnMeta, Dataset, LabelSet};

use super::datetime;
use super::fields;
use super::MapCtx;
use crate::error::LegacyError;

/// `OutCol` → `(Column, ColumnMeta)` lists for `Dataset::new`.
pub(crate) fn out_columns(cols: Vec<fields::OutCol>) -> (Vec<Column>, Vec<ColumnMeta>) {
    let mut columns = Vec::with_capacity(cols.len());
    let mut metas = Vec::with_capacity(cols.len());
    for c in cols {
        let n = c.values.len();
        let data = match c.ty {
            "integer" => ColumnData::Integer(
                c.values
                    .into_iter()
                    .map(|v| match v {
                        Some(datetime::Typed::Int(i)) => Some(i),
                        _ => None,
                    })
                    .collect(),
            ),
            "real" => ColumnData::Real(
                c.values
                    .into_iter()
                    .map(|v| match v {
                        Some(datetime::Typed::Real(f)) => Some(f),
                        _ => None,
                    })
                    .collect(),
            ),
            "boolean" => ColumnData::Boolean(
                c.values
                    .into_iter()
                    .map(|v| match v {
                        Some(datetime::Typed::Bool(b)) => Some(b),
                        _ => None,
                    })
                    .collect(),
            ),
            "date" => ColumnData::Date(
                c.values
                    .into_iter()
                    .map(|v| match v {
                        Some(datetime::Typed::Date(d)) => Some(d),
                        _ => None,
                    })
                    .collect(),
            ),
            "time" => ColumnData::Time(
                c.values
                    .into_iter()
                    .map(|v| match v {
                        Some(datetime::Typed::Time(t)) => Some(t),
                        _ => None,
                    })
                    .collect(),
            ),
            "date_time" => ColumnData::DateTime(
                c.values
                    .into_iter()
                    .map(|v| match v {
                        Some(datetime::Typed::DateTime(t)) => Some(t),
                        _ => None,
                    })
                    .collect(),
            ),
            _ => ColumnData::Text(
                c.values
                    .into_iter()
                    .map(|v| match v {
                        Some(datetime::Typed::Text(s)) => Some(s),
                        _ => None,
                    })
                    .collect(),
            ),
        };
        debug_assert_eq!(data.len(), n);
        columns.push(Column { name: c.name, data });
        metas.push(c.meta);
    }
    (columns, metas)
}

/// Build the `Dataset`: referenced label sets (first-seen column
/// order, like the golden's emit) then `Dataset::new().with_meta()`.
pub(crate) fn finish_dataset(
    id: &str,
    columns: Vec<Column>,
    metas: Vec<ColumnMeta>,
    ctx: &mut MapCtx,
) -> Result<Dataset, LegacyError> {
    let mut seen = HashSet::new();
    let mut sets: BTreeMap<String, LabelSet> = BTreeMap::new();
    for m in &metas {
        if let Some(lid) = &m.label_set {
            if seen.insert(lid.clone()) {
                if let Some(set) = ctx.label_sets.get(lid) {
                    sets.insert(lid.clone(), set.clone());
                }
            }
        }
    }
    Ok(Dataset::new(id.to_string(), 1, columns)?.with_meta(metas, sets)?)
}
