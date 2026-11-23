//! Loader for `fixtures/data/*.dataset.json` (spec §3.3) and the v1.1
//! dataset JSON v2 (`columns[].type` adds `time`/`date_time` as
//! `hh:mm:ss` / `YYYY-MM-DD hh:mm:ss` strings; `columns[].meta`;
//! `label_sets`), used by tests and `corpus-runner` only — not a
//! product importer. Malformed JSON or a schema violation is
//! `CorpusFormat`; `Dataset::new`/`with_meta` rules keep their own
//! errors.

use crate::dataset::{Column, ColumnData, ColumnMeta, Dataset, LabelSet};
use crate::EngineError;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Parses one `*.dataset.json` document into a validated `Dataset`.
/// A document without `meta` or `label_sets` builds the same v1
/// dataset as before.
pub fn load_dataset(json: &str) -> Result<Dataset, EngineError> {
    let raw: DatasetJson =
        serde_json::from_str(json).map_err(|e| corpus(format!("malformed JSON: {e}")))?;
    if raw.id.is_empty() {
        return Err(corpus("id must not be empty"));
    }
    if raw.version == 0 {
        return Err(corpus("version must be at least 1"));
    }
    let mut columns = Vec::with_capacity(raw.columns.len());
    for column in &raw.columns {
        columns.push(Column {
            name: column.name.clone(),
            data: column_data(column)?,
        });
    }
    let ds = Dataset::new(raw.id, raw.version, columns)?;
    let has_meta = raw.columns.iter().any(|c| c.meta.is_some());
    if !has_meta && raw.label_sets.is_none() {
        return Ok(ds);
    }
    let metas: Vec<ColumnMeta> = raw
        .columns
        .iter()
        .map(|c| c.meta.clone().unwrap_or_default())
        .collect();
    let mut label_sets = BTreeMap::new();
    for set in raw.label_sets.iter().flatten() {
        if set.id.is_empty() {
            return Err(corpus("label set id must not be empty"));
        }
        let mut entries = Vec::with_capacity(set.entries.len());
        for (i, (value, label)) in set.entries.iter().enumerate() {
            let value = serde_json::from_value::<crate::dataset::Value>(value.clone())
                .map_err(|e| corpus(format!("label set {:?}: entry {i}: {e}", set.id)))?;
            entries.push((value, label.clone()));
        }
        if label_sets
            .insert(
                set.id.clone(),
                LabelSet {
                    id: set.id.clone(),
                    entries,
                },
            )
            .is_some()
        {
            return Err(corpus(format!("duplicate label set id {:?}", set.id)));
        }
    }
    ds.with_meta(metas, label_sets)
}

/// Loads every `*.dataset.json` in `dir`, keyed by dataset id.
pub fn load_dataset_dir(dir: &Path) -> Result<BTreeMap<String, Dataset>, EngineError> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| corpus(format!("cannot read {}: {e}", dir.display())))?;
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry
            .map_err(|e| corpus(format!("cannot read {}: {e}", dir.display())))?
            .path();
        let is_dataset = path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".dataset.json"));
        if is_dataset {
            paths.push(path);
        }
    }
    paths.sort();

    let mut datasets = BTreeMap::new();
    for path in paths {
        let json = std::fs::read_to_string(&path)
            .map_err(|e| corpus(format!("cannot read {}: {e}", path.display())))?;
        let dataset =
            load_dataset(&json).map_err(|e| corpus(format!("{}: {}", path.display(), e)))?;
        if datasets.insert(dataset.id().to_string(), dataset).is_some() {
            return Err(corpus(format!(
                "duplicate dataset id in {}",
                path.display()
            )));
        }
    }
    Ok(datasets)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DatasetJson {
    id: String,
    version: u64,
    columns: Vec<ColumnJson>,
    /// v2: value-label sets the column metas reference.
    label_sets: Option<Vec<LabelSetJson>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ColumnJson {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    values: Vec<Value>,
    /// v2: per-column metadata; never hashed but `label_set` names a
    /// set that is.
    meta: Option<ColumnMeta>,
}

/// One `label_sets[]` element: `[{"type": ..., "value": ...}, label]`
/// pairs in source order.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LabelSetJson {
    id: String,
    entries: Vec<(Value, String)>,
}

fn column_data(column: &ColumnJson) -> Result<ColumnData, EngineError> {
    let name = &column.name;
    match column.kind.as_str() {
        "integer" => Ok(ColumnData::Integer(convert(name, &column.values, |v| {
            v.as_i64()
        })?)),
        "real" => Ok(ColumnData::Real(convert(name, &column.values, |v| {
            v.as_f64().filter(|x| x.is_finite())
        })?)),
        "text" => Ok(ColumnData::Text(convert(name, &column.values, |v| {
            v.as_str().map(str::to_string)
        })?)),
        "boolean" => Ok(ColumnData::Boolean(convert(name, &column.values, |v| {
            v.as_bool()
        })?)),
        "date" => Ok(ColumnData::Date(convert(name, &column.values, |v| {
            v.as_str().and_then(|s| parse_iso_date(s).ok())
        })?)),
        "time" => Ok(ColumnData::Time(convert(name, &column.values, |v| {
            v.as_str().and_then(|s| parse_iso_time(s).ok())
        })?)),
        "date_time" => Ok(ColumnData::DateTime(convert(name, &column.values, |v| {
            v.as_str().and_then(|s| parse_iso_datetime(s).ok())
        })?)),
        other => Err(corpus(format!("column {name}: unknown type {other:?}"))),
    }
}

/// Maps `null` to `None` and converts each present JSON value, a wrong
/// JSON type being a `CorpusFormat` schema violation (spec §3.3).
fn convert<T>(
    column: &str,
    values: &[Value],
    f: impl Fn(&Value) -> Option<T>,
) -> Result<Vec<Option<T>>, EngineError> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if v.is_null() {
                Ok(None)
            } else {
                f(v).map(Some).ok_or_else(|| {
                    corpus(format!(
                        "column {column}: value {i} has the wrong type ({v})"
                    ))
                })
            }
        })
        .collect()
}

/// Strict `YYYY-MM-DD` → days since 1970-01-01 (spec §3.3).
fn parse_iso_date(s: &str) -> Result<i32, EngineError> {
    let bad = || corpus(format!("invalid ISO date {s:?}"));
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return Err(bad());
    }
    let num = |from: usize, to: usize| -> Result<i64, EngineError> {
        match s.get(from..to) {
            Some(sub) if sub.bytes().all(|c| c.is_ascii_digit()) => {
                sub.parse::<i64>().map_err(|_| bad())
            }
            _ => Err(bad()),
        }
    };
    let (year, month, day) = (num(0, 4)?, num(5, 7)?, num(8, 10)?);
    if !(1..=12).contains(&month) {
        return Err(bad());
    }
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(year) => 29,
        _ => 28,
    };
    if !(1..=days_in_month).contains(&day) {
        return Err(bad());
    }
    i32::try_from(days_from_civil(year, month, day)).map_err(|_| bad())
}

/// Strict `hh:mm:ss` → seconds since midnight (v1.1 spec §4.3).
fn parse_iso_time(s: &str) -> Result<i32, EngineError> {
    let bad = || corpus(format!("invalid time {s:?}"));
    let b = s.as_bytes();
    if b.len() != 8 || b[2] != b':' || b[5] != b':' {
        return Err(bad());
    }
    let num = |from: usize, to: usize| -> Result<i64, EngineError> {
        match s.get(from..to) {
            Some(sub) if sub.bytes().all(|c| c.is_ascii_digit()) => {
                sub.parse::<i64>().map_err(|_| bad())
            }
            _ => Err(bad()),
        }
    };
    let (h, m, sec) = (num(0, 2)?, num(3, 5)?, num(6, 8)?);
    if !(0..=23).contains(&h) || !(0..=59).contains(&m) || !(0..=59).contains(&sec) {
        return Err(bad());
    }
    i32::try_from(h * 3600 + m * 60 + sec).map_err(|_| bad())
}

/// Strict `YYYY-MM-DD hh:mm:ss` → seconds since 1970-01-01, no time
/// zone (v1.1 spec §4.3).
fn parse_iso_datetime(s: &str) -> Result<i64, EngineError> {
    let bad = || corpus(format!("invalid date-time {s:?}"));
    if s.len() != 19 || s.as_bytes()[10] != b' ' {
        return Err(bad());
    }
    let days = i64::from(parse_iso_date(&s[..10])?);
    let secs = i64::from(parse_iso_time(&s[11..])?);
    days.checked_mul(86_400)
        .and_then(|d| d.checked_add(secs))
        .ok_or_else(bad)
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// Howard Hinnant's `days_from_civil`: proleptic-Gregorian
/// `YYYY-MM-DD` → days since 1970-01-01.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}
