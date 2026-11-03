//! Loader for `fixtures/data/*.dataset.json` (spec §3.3), used by tests
//! and `corpus-runner` only — not a product importer. Malformed JSON or
//! a schema violation is `CorpusFormat`; `Dataset::new` rules keep their
//! own errors.

use crate::dataset::{Column, ColumnData, Dataset};
use crate::EngineError;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

/// Parses one `*.dataset.json` document into a validated `Dataset`.
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
    Dataset::new(raw.id, raw.version, columns)
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
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ColumnJson {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    values: Vec<Value>,
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
