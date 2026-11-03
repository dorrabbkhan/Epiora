//! Category machinery for `tabulate`: distinct-value collection in the
//! spec §3.3 category order, cell→index lookups, 2×2 orientation and
//! explicit-`levels` resolution. The label of the appended missing
//! category is `"(missing)"` (spec §4.1).

use crate::dataset::{ColumnData, Value};
use crate::EngineError;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// The label of the appended missing category (spec §4.1).
pub(crate) const MISSING_LABEL: &str = "(missing)";

/// Hashable cell identity; `Real` keys are f64 bit patterns with
/// `-0.0` normalised to `0.0` (the dataset hash does the same).
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) enum Key {
    I(i64),
    R(u64),
    T(String),
    B(bool),
    D(i32),
}

/// One variable's resolved categories and a cell→index lookup.
pub(crate) struct Cats {
    pub labels: Vec<String>,
    index: Index,
    pub missing: u64,
    /// Index of the `"(missing)"` category when `include_missing`.
    pub missing_idx: Option<usize>,
}

enum Index {
    I(HashMap<i64, usize>),
    R(HashMap<u64, usize>),
    T(HashMap<String, usize>),
    B([Option<usize>; 2]),
    D(HashMap<i32, usize>),
}

fn real_bits(x: f64) -> u64 {
    if x == 0.0 {
        0
    } else {
        x.to_bits()
    }
}

fn key_cmp(a: &Key, b: &Key) -> Ordering {
    match (a, b) {
        (Key::I(x), Key::I(y)) => x.cmp(y),
        (Key::R(x), Key::R(y)) => f64::from_bits(*x).total_cmp(&f64::from_bits(*y)),
        (Key::T(x), Key::T(y)) => x.cmp(y),
        // Boolean category order is Yes, No (spec §3.3).
        (Key::B(x), Key::B(y)) => y.cmp(x),
        (Key::D(x), Key::D(y)) => x.cmp(y),
        (a, b) => rank(a).cmp(&rank(b)),
    }
}

fn rank(k: &Key) -> u8 {
    match k {
        Key::I(_) => 0,
        Key::R(_) => 1,
        Key::T(_) => 2,
        Key::B(_) => 3,
        Key::D(_) => 4,
    }
}

fn label(key: &Key) -> String {
    match key {
        Key::I(v) => v.to_string(),
        Key::R(b) => ryu::Buffer::new()
            .format_finite(f64::from_bits(*b))
            .to_string(),
        Key::T(s) => s.clone(),
        Key::B(v) => if *v { "Yes" } else { "No" }.to_string(),
        Key::D(d) => iso_date(*d),
    }
}

/// Days since 1970-01-01 → `YYYY-MM-DD` (Howard Hinnant's
/// `civil_from_days`, the inverse of the loader's `days_from_civil`).
fn iso_date(z: i32) -> String {
    let z = i64::from(z) + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", y + i64::from(m <= 2), m, d)
}

/// The distinct non-missing keys of a column, in category order, and
/// its missing count.
pub(crate) fn collect(data: &ColumnData) -> (Vec<Key>, u64) {
    fn uniques<T: Copy + Eq + std::hash::Hash>(
        cells: &[Option<T>],
        f: impl Fn(T) -> Key,
    ) -> (Vec<Key>, u64) {
        let mut seen = HashSet::new();
        let mut missing = 0_u64;
        let mut keys = Vec::new();
        for cell in cells {
            match cell {
                Some(v) => {
                    if seen.insert(*v) {
                        keys.push(f(*v));
                    }
                }
                None => missing += 1,
            }
        }
        (keys, missing)
    }
    let (mut keys, missing) = match data {
        ColumnData::Integer(v) => uniques(v, Key::I),
        ColumnData::Real(v) => {
            let mut seen = HashSet::new();
            let mut missing = 0_u64;
            let mut keys = Vec::new();
            for cell in v {
                match cell {
                    Some(x) => {
                        if seen.insert(real_bits(*x)) {
                            keys.push(Key::R(real_bits(*x)));
                        }
                    }
                    None => missing += 1,
                }
            }
            (keys, missing)
        }
        ColumnData::Text(v) => {
            let mut seen = HashSet::new();
            let mut missing = 0_u64;
            let mut keys = Vec::new();
            for cell in v {
                match cell {
                    Some(s) => {
                        if seen.insert(s.as_str()) {
                            keys.push(Key::T(s.clone()));
                        }
                    }
                    None => missing += 1,
                }
            }
            (keys, missing)
        }
        ColumnData::Boolean(v) => uniques(v, Key::B),
        ColumnData::Date(v) => uniques(v, Key::D),
    };
    keys.sort_by(key_cmp);
    (keys, missing)
}

fn value_key(v: &Value) -> Key {
    match v {
        Value::Integer(x) => Key::I(*x),
        Value::Real(x) => Key::R(real_bits(*x)),
        Value::Text(s) => Key::T(s.clone()),
        Value::Boolean(b) => Key::B(*b),
        Value::Date(d) => Key::D(*d),
    }
}

impl Cats {
    /// Builds the category list and lookup for one column. `keys` is
    /// the already-ordered non-missing categories; `"(missing)"` is
    /// appended only when `include_missing` and missing rows exist.
    pub(crate) fn build(
        keys: &[Key],
        missing: u64,
        data: &ColumnData,
        include_missing: bool,
    ) -> Cats {
        let mut labels: Vec<String> = keys.iter().map(label).collect();
        let missing_idx = if include_missing && missing > 0 {
            labels.push(MISSING_LABEL.to_string());
            Some(labels.len() - 1)
        } else {
            None
        };
        let pos = |k: &Key| keys.iter().position(|x| x == k);
        let index = match data {
            ColumnData::Integer(_) => Index::I(
                keys.iter()
                    .enumerate()
                    .filter_map(|(i, k)| match k {
                        Key::I(v) => Some((*v, i)),
                        _ => None,
                    })
                    .collect(),
            ),
            ColumnData::Real(_) => Index::R(
                keys.iter()
                    .enumerate()
                    .filter_map(|(i, k)| match k {
                        Key::R(v) => Some((*v, i)),
                        _ => None,
                    })
                    .collect(),
            ),
            ColumnData::Text(_) => Index::T(
                keys.iter()
                    .enumerate()
                    .filter_map(|(i, k)| match k {
                        Key::T(s) => Some((s.clone(), i)),
                        _ => None,
                    })
                    .collect(),
            ),
            ColumnData::Boolean(_) => Index::B([pos(&Key::B(true)), pos(&Key::B(false))]),
            ColumnData::Date(_) => Index::D(
                keys.iter()
                    .enumerate()
                    .filter_map(|(i, k)| match k {
                        Key::D(v) => Some((*v, i)),
                        _ => None,
                    })
                    .collect(),
            ),
        };
        Cats {
            labels,
            index,
            missing,
            missing_idx,
        }
    }

    /// The category index of row `i`'s cell, or `None` when missing.
    pub(crate) fn idx(&self, data: &ColumnData, i: usize) -> Option<usize> {
        match (&self.index, data) {
            (Index::I(m), ColumnData::Integer(v)) => v[i].and_then(|x| m.get(&x).copied()),
            (Index::R(m), ColumnData::Real(v)) => v[i].and_then(|x| m.get(&real_bits(x)).copied()),
            (Index::T(m), ColumnData::Text(v)) => v[i].as_deref().and_then(|s| m.get(s).copied()),
            (Index::B([t, f]), ColumnData::Boolean(v)) => {
                v[i].and_then(|b| if b { *t } else { *f })
            }
            (Index::D(m), ColumnData::Date(v)) => v[i].and_then(|x| m.get(&x).copied()),
            _ => None,
        }
    }
}

/// The category list of one variable for `cross`: explicit `levels`
/// must equal the non-missing category set (spec §3.3 names them an
/// orientation aid, not a filter — a non-matching set is
/// `InvalidOptions`, never a silent drop).
pub(crate) fn resolved(
    name: &str,
    data: &ColumnData,
    levels: Option<&[Value; 2]>,
) -> Result<(Vec<Key>, u64), EngineError> {
    let (keys, missing) = collect(data);
    let Some(levels) = levels else {
        return Ok((keys, missing));
    };
    let lv = [value_key(&levels[0]), value_key(&levels[1])];
    let ok = lv[0] != lv[1] && keys.len() == 2 && keys.contains(&lv[0]) && keys.contains(&lv[1]);
    if ok {
        Ok((lv.to_vec(), missing))
    } else {
        Err(EngineError::InvalidOptions {
            reason: format!("levels of {name} do not match its categories"),
        })
    }
}

/// 2×2 orientation for a two-category variable without explicit
/// `levels` (spec §3.3): Boolean puts Yes first, Integer `{0, 1}` puts
/// 1 first; anything else is `AmbiguousLevels`, never a guess.
pub(crate) fn orient(keys: &[Key], data: &ColumnData, name: &str) -> Result<Vec<Key>, EngineError> {
    match data {
        ColumnData::Boolean(_) => Ok(keys.to_vec()),
        ColumnData::Integer(_) if keys == [Key::I(0), Key::I(1)] => Ok(vec![Key::I(1), Key::I(0)]),
        _ => Err(EngineError::AmbiguousLevels {
            variable: name.to_string(),
        }),
    }
}
