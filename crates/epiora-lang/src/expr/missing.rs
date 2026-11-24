//! The Epi Info 7 eval domain (`Rule_*` object values) and its missing
//! rules. Two kinds of "missing" exist in EI7, exactly as in the
//! interpreter: a *cell* that is missing reads as the empty string
//! `""` (the `TryGetVariable`/Standard-var path maps `DBNull` → `""`),
//! while the `(.)` literal and failed computations are `null`.
//! [`EV::Text("")`] is the former, [`EV::Missing`] the latter; both
//! count as `IsEmpty`, but they differ in `&` and in functions that
//! branch on `is string`.

use epiora_core::dataset::{ColumnData, Value, ValueType};

/// One evaluated value — the port of the `object` the `Rule_*` code
/// passes around.
#[derive(Clone, Debug)]
pub(crate) enum EV {
    /// `null` — `(.)`, a failed parse or a no-match branch.
    Missing,
    /// `bool`.
    Bool(bool),
    /// `long`/`int` — whole numbers.
    Int(i64),
    /// `double`.
    Real(f64),
    /// `string` — including `""`, the missing-cell value.
    Text(String),
    /// `DateTime` with the time at midnight — days since 1970-01-01.
    Date(i32),
    /// `DateTime` on the time-only frame — seconds since midnight.
    Time(i32),
    /// `DateTime` — seconds since 1970-01-01.
    DateTime(i64),
}

/// `Util.IsEmpty` — `null`, `DBNull` or the empty string.
pub(crate) fn is_empty(v: &EV) -> bool {
    match v {
        EV::Missing => true,
        EV::Text(s) => s.is_empty(),
        _ => false,
    }
}

/// The `== "true"` test EI7's logic operators use on
/// `Execute().ToString()` (case-insensitive). Numbers, dates and
/// missing values are not truthy.
pub(crate) fn truthy(v: &EV) -> bool {
    match v {
        EV::Bool(b) => *b,
        EV::Text(s) => s.eq_ignore_ascii_case("true"),
        _ => false,
    }
}

/// `ConvertStringToBoolean`: the EI7 spellings. `None` for anything
/// else — where EI7 casts the result it crashes, which we surface as
/// `Bool(false)` at the compare sites that used it.
pub(crate) fn text_to_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_uppercase().as_str() {
        "(+)" | "YES" | "Y" | "TRUE" | "T" => Some(true),
        "(-)" | "NO" | "N" | "FALSE" | "F" => Some(false),
        _ => None,
    }
}

/// `ToString()` in en-US — the text EI7's comparisons, `&`, `STRLEN`
/// and friends operate on.
pub(crate) fn ei_to_string(v: &EV) -> String {
    match v {
        EV::Missing => String::new(),
        EV::Bool(b) => {
            if *b {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        EV::Int(i) => i.to_string(),
        EV::Real(x) => crate::functions::math::dotnet_g15(*x),
        EV::Text(s) => s.clone(),
        EV::Date(d) => {
            let (y, m, dd) = crate::functions::date::civil_from_days(*d);
            format!("{m}/{dd}/{y} 12:00:00 AM")
        }
        EV::DateTime(secs) => {
            let (y, m, dd, h, mi, s) = crate::functions::date::civil_from_secs(*secs);
            let (h12, ap) = hour12(h);
            format!("{m}/{dd}/{y} {h12}:{mi:02}:{s:02} {ap}")
        }
        EV::Time(t) => format!("{:02}:{:02}:{:02}", t / 3600, t / 60 % 60, t % 60),
    }
}

fn hour12(h: u32) -> (u32, &'static str) {
    match h {
        0 => (12, "AM"),
        12 => (12, "PM"),
        h if h < 12 => (h, "AM"),
        h => (h - 12, "PM"),
    }
}

/// The `is double`-domain coercion: `Int` and `Real` only — `&` and
/// the arithmetic operators branch on the CLR type, they do not
/// `TryParse` (a text `"5"` is *not* numeric there).
pub(crate) fn as_f64(v: &EV) -> Option<f64> {
    match v {
        EV::Int(i) => Some(*i as f64),
        EV::Real(x) => Some(*x),
        _ => None,
    }
}

/// The `ToString()` + `double.TryParse` coercion the scalar functions
/// use — any value renders, then the text parses (en-US float rules).
pub(crate) fn coerce_f64(v: &EV) -> Option<f64> {
    match v {
        EV::Missing => None,
        _ => {
            let s = ei_to_string(v);
            let t = s.trim();
            if t.is_empty() {
                return None;
            }
            // `double.TryParse` accepts a leading sign and exponents.
            let t = t.strip_prefix('+').unwrap_or(t);
            match t.parse::<f64>() {
                Ok(x) if x.is_finite() || t.eq_ignore_ascii_case("nan") => Some(x),
                _ => None,
            }
        }
    }
}

/// A dataset cell → the eval domain. `None` (a missing cell) reads as
/// `""`, matching `TryGetVariable`'s `DBNull` → `""` mapping.
pub(crate) fn cell_ev(data: &ColumnData, row: usize) -> EV {
    match data {
        ColumnData::Integer(v) => v[row].map_or_else(|| EV::Text(String::new()), EV::Int),
        ColumnData::Real(v) => v[row].map_or_else(|| EV::Text(String::new()), EV::Real),
        ColumnData::Text(v) => v[row]
            .clone()
            .map_or_else(|| EV::Text(String::new()), EV::Text),
        ColumnData::Boolean(v) => v[row].map_or_else(|| EV::Text(String::new()), EV::Bool),
        ColumnData::Date(v) => v[row].map_or_else(|| EV::Text(String::new()), EV::Date),
        ColumnData::Time(v) => v[row].map_or_else(|| EV::Text(String::new()), EV::Time),
        ColumnData::DateTime(v) => v[row].map_or_else(|| EV::Text(String::new()), EV::DateTime),
    }
}

/// A session global (a `Value` captured into the recipe) → the eval
/// domain.
pub(crate) fn value_ev(v: &Value) -> EV {
    match v {
        Value::Integer(i) => EV::Int(*i),
        Value::Real(x) => EV::Real(*x),
        Value::Text(s) => EV::Text(s.clone()),
        Value::Boolean(b) => EV::Bool(*b),
        Value::Date(d) => EV::Date(*d),
        Value::Time(t) => EV::Time(*t),
        Value::DateTime(s) => EV::DateTime(*s),
    }
}

/// The `ValueType` an `EV` carries when it lands in a column.
pub(crate) fn ev_type(v: &EV) -> ValueType {
    match v {
        EV::Missing => ValueType::Boolean,
        EV::Bool(_) => ValueType::Boolean,
        EV::Int(_) => ValueType::Integer,
        EV::Real(_) => ValueType::Real,
        EV::Text(_) => ValueType::Text,
        EV::Date(_) => ValueType::Date,
        EV::Time(_) => ValueType::Time,
        EV::DateTime(_) => ValueType::DateTime,
    }
}
