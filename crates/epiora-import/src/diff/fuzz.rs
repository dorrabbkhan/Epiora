//! The `diff_any` fuzz harness (plan Task 5 Step 4): decode two small
//! arbitrary datasets plus a key choice, run [`diff`], and on `Ok`
//! check the apply property — applying the diff to `previous`'s
//! `record_id → record JSON` map reproduces `next`'s records map.

use super::{diff, Diff, FieldChange};
use crate::keys::record_ids;
use crate::records::record_json;
use crate::settings::RecordKey;
use crate::Imported;
use arbitrary::Unstructured;
use epiora_core::dataset::{Column, ColumnData, Dataset};
use std::collections::{BTreeMap, HashMap};

/// Fuzz entry point (plan Task 5): two small arbitrary datasets and a
/// key choice → [`diff`]; must return, never panic, and on `Ok` the
/// apply property holds (diff applied to `previous`'s records equals
/// `next`'s records).
pub fn diff_any(data: &[u8]) {
    let Some((prev_cols, next_cols, key)) = fuzz_decode(data) else {
        return;
    };
    let Ok(previous) = Dataset::new("p", 1, prev_cols) else {
        return;
    };
    let next = Imported {
        row_count: next_cols.first().map(|c| c.data.len() as u64).unwrap_or(0),
        columns: next_cols,
        detection: crate::Detection {
            container: crate::Container::Text,
            encoding: Some(crate::Encoding::Utf8),
            delimiter: Some(','),
            decimal: crate::DecimalSep::Dot,
            header_renames: Vec::new(),
            sheets: Vec::new(),
        },
        settings: crate::ImportSettings::default(),
        coercions: BTreeMap::new(),
        source_sha256: String::new(),
    };
    if let Ok(d) = diff(&previous, &next, &key) {
        check_apply(&previous, &next, &key, &d);
    }
}

/// Decodes two bounded column sets plus a key choice from fuzz bytes.
/// Ids are deduplicated keeping first occurrence so the `Ok` path is
/// exercised; extra columns are renamed deterministically when a name
/// collides.
fn fuzz_decode(
    data: &[u8],
) -> Option<(
    Vec<epiora_core::dataset::Column>,
    Vec<epiora_core::dataset::Column>,
    RecordKey,
)> {
    const MAX_ROWS: usize = 24;
    const MAX_COLS: usize = 8;
    const MAX_NAME: usize = 16;

    fn side(u: &mut Unstructured) -> arbitrary::Result<Vec<Column>> {
        let mut ids: Vec<i64> = u.arbitrary()?;
        ids.truncate(MAX_ROWS);
        let mut unique: std::collections::HashSet<i64> = std::collections::HashSet::new();
        ids.retain(|i| unique.insert(*i));
        let rows = ids.len();
        let n_extra = usize::from(u.arbitrary::<u8>()?) % MAX_COLS;
        let mut columns = Vec::with_capacity(n_extra + 1);
        columns.push(Column {
            name: "id".to_string(),
            data: ColumnData::Integer(ids.into_iter().map(Some).collect()),
        });
        for i in 0..n_extra {
            let mut name: String = u.arbitrary()?;
            name = name.chars().take(MAX_NAME).collect();
            // Names must stay unique (`Imported` is not validated like
            // `Dataset`): fall back to `c<i>` and keep suffixing until
            // the name is free.
            if name.is_empty() || columns.iter().any(|c: &Column| c.name == name) {
                let mut k = i;
                loop {
                    let cand = format!("c{k}");
                    if columns.iter().all(|c: &Column| c.name != cand) {
                        name = cand;
                        break;
                    }
                    k += 1;
                }
            }
            let raw: Vec<Option<i64>> = u.arbitrary()?;
            let mut cells: Vec<Option<i64>> = raw;
            cells.truncate(rows);
            cells.resize(rows, None);
            let data = match u.arbitrary::<u8>()? % 5 {
                0 => ColumnData::Integer(cells),
                1 => ColumnData::Real(cells.iter().map(|c| c.map(|v| v as f64)).collect()),
                2 => ColumnData::Text(
                    cells
                        .iter()
                        .map(|c| c.map(|v| format!("t{}", v.rem_euclid(11))))
                        .collect(),
                ),
                3 => ColumnData::Boolean(cells.iter().map(|c| c.map(|v| v % 2 == 0)).collect()),
                _ => ColumnData::Date(
                    cells
                        .iter()
                        .map(|c| {
                            c.map(|v| i32::try_from(v.rem_euclid(200_000)).unwrap_or(0) - 100_000)
                        })
                        .collect(),
                ),
            };
            columns.push(Column { name, data });
        }
        Ok(columns)
    }

    let mut u = Unstructured::new(data);
    let prev = side(&mut u).ok()?;
    let next = side(&mut u).ok()?;
    let key = match u.arbitrary::<u8>().ok()? % 8 {
        0..=3 => RecordKey::RowPosition,
        7 => RecordKey::Column {
            name: format!("k{}", u.arbitrary::<u8>().ok()? % 4),
        },
        _ => RecordKey::Column {
            name: "id".to_string(),
        },
    };
    Some((prev, next, key))
}

/// Parses a `record_json` row back into its object map; `record_json`
/// always emits an object.
fn record_map(columns: &[Column], row: usize) -> serde_json::Map<String, serde_json::Value> {
    match serde_json::from_str::<serde_json::Value>(&record_json(columns, row))
        .unwrap_or(serde_json::Value::Null)
    {
        serde_json::Value::Object(m) => m,
        _ => serde_json::Map::new(),
    }
}

/// The apply-property checker (plan Task 5 Step 4): rebuild `next`'s
/// records map from `previous`'s via the diff alone — `next` supplies
/// only the ids and records behind `d.added`'s row indices — then
/// compare the whole maps. A spurious `added`/`removed`, a missing
/// `removed` entry or a wrong field change fails the assert, which
/// under cargo-fuzz is the crash signal — the crate's `clippy::panic`
/// deny is lifted for exactly this assertion.
#[allow(clippy::panic)]
fn check_apply(previous: &Dataset, next: &Imported, key: &RecordKey, d: &Diff) {
    let prev_ids = record_ids(previous.columns(), key).unwrap_or_default();
    let next_ids = record_ids(&next.columns, key).unwrap_or_default();
    let mut map: BTreeMap<String, serde_json::Map<String, serde_json::Value>> = prev_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), record_map(previous.columns(), i)))
        .collect();
    for id in &d.removed {
        assert!(map.remove(id).is_some(), "removed id {id} not in previous");
    }
    let mut by_id: HashMap<&str, Vec<&FieldChange>> = HashMap::new();
    for fc in &d.changed {
        assert!(
            map.contains_key(fc.record_id.as_str()),
            "field change for {} names no remaining record",
            fc.record_id
        );
        by_id.entry(fc.record_id.as_str()).or_default().push(fc);
    }
    for (id, rec) in &mut map {
        for name in &d.schema.removed_columns {
            rec.remove(name);
        }
        if let Some(changes) = by_id.get(id.as_str()) {
            for fc in changes {
                let cell = match &fc.new {
                    Some(v) => serde_json::to_value(v).unwrap_or(serde_json::Value::Null),
                    None => serde_json::Value::Null,
                };
                rec.insert(fc.field.clone(), cell);
            }
        }
    }
    for &j in &d.added {
        let id = &next_ids[j as usize];
        assert!(!map.contains_key(id), "added id {id} already present");
        map.insert(id.clone(), record_map(&next.columns, j as usize));
    }
    // A column the schema added with no recorded value is missing in
    // the surviving records.
    for rec in map.values_mut() {
        for c in &next.columns {
            rec.entry(c.name.clone()).or_insert(serde_json::Value::Null);
        }
    }
    let got: BTreeMap<String, String> = map
        .into_iter()
        .map(|(id, m)| {
            (
                id,
                serde_json::to_string(&serde_json::Value::Object(m))
                    .unwrap_or_else(|_| "{}".to_string()),
            )
        })
        .collect();
    let expected: BTreeMap<String, String> = next_ids
        .iter()
        .enumerate()
        .map(|(j, id)| (id.clone(), record_json(&next.columns, j)))
        .collect();
    assert!(
        got == expected,
        "apply property violated: {got:?} != {expected:?}"
    );
}
