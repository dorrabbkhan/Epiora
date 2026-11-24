//! Text functions — `UPPERCASE`, `SUBSTRING`, `STRLEN`, `FINDTEXT`,
//! `FORMAT` — ported with their `ToString()`/`TryParse` coercions and
//! EI7's null/empty results.

use crate::expr::missing::{ei_to_string, is_empty, EV};
use crate::functions::date;

/// `UPPERCASE(v)` — `ToString().ToUpperInvariant()`; missing when the
/// operand is empty.
pub(crate) fn uppercase(v: &EV) -> EV {
    if is_empty(v) {
        return EV::Missing;
    }
    EV::Text(ei_to_string(v).to_uppercase())
}

/// `SUBSTRING(text, start[, len])` — 1-based; `int.Parse` the numeric
/// arguments (a real throws → missing). Length defaults to the whole
/// string; `start > len` yields `""` → missing.
pub(crate) fn substring(v: &[EV]) -> EV {
    let s = ei_to_string(&v[0]);
    let Some(start) = coerce_int_strict(&v[1]) else {
        return EV::Missing;
    };
    let Some(len) = (match v.len() {
        2 => Some(i64::try_from(s.chars().count()).unwrap_or(i64::MAX)),
        _ => coerce_int_strict(&v[2]),
    }) else {
        return EV::Missing;
    };
    let chars: Vec<char> = s.chars().collect();
    if start < 1 || start as usize > chars.len() || len <= 0 {
        return EV::Missing;
    }
    let take = (chars.len() - (start as usize - 1)).min(len as usize);
    EV::Text(
        chars[(start as usize - 1)..(start as usize - 1 + take)]
            .iter()
            .collect(),
    )
}

/// `STRLEN(v)` — `.Length` of a **string** operand only (the cast
/// throws for anything else → missing).
pub(crate) fn strlen(v: &EV) -> EV {
    match v {
        EV::Text(s) => EV::Int(s.chars().count() as i64),
        _ => EV::Missing,
    }
}

/// `FINDTEXT(needle, haystack)` — `IndexOf(OrdinalIgnoreCase) + 1`.
pub(crate) fn findtext(a: &EV, b: &EV) -> EV {
    if is_empty(a) || is_empty(b) {
        return EV::Missing;
    }
    let needle = ei_to_string(a).to_lowercase();
    let hay = ei_to_string(b).to_lowercase();
    match hay.find(&needle) {
        Some(byte_idx) => EV::Int((hay[..byte_idx].chars().count() + 1) as i64),
        None => EV::Int(0),
    }
}

/// `FORMAT(value[, pattern])` — EI7 calls
/// `Microsoft.VisualBasic.Strings.Format`; the v1.1 corpus pins
/// `FORMAT(x, "0.00")` → `x.ToString()` ("54", "46.1"), so numeric
/// pictures return the raw `ToString()`. The named date/time formats
/// (`GeneralDate`, `LongDate`, `ShortDate`, `LongTime`, `ShortTime`)
/// are applied as a method choice; everything else is the raw
/// `ToString()` too (decisions.md).
pub(crate) fn format(v: &[EV]) -> EV {
    if is_empty(&v[0]) {
        return EV::Missing;
    }
    if v.len() >= 2 {
        let pattern = ei_to_string(&v[1]);
        match pattern.as_str() {
            "GeneralDate" => return fmt_date(&v[0], DateFmt::General),
            "LongDate" => return fmt_date(&v[0], DateFmt::Long),
            "ShortDate" => return fmt_date(&v[0], DateFmt::Short),
            "LongTime" => return fmt_time(&v[0], false),
            "ShortTime" => return fmt_time(&v[0], true),
            _ => {}
        }
    }
    EV::Text(ei_to_string(&v[0]))
}

/// The date renderings the en-US `string.Format` produces.
enum DateFmt {
    /// `GeneralDate`: `M/d/yyyy h:mm:ss tt` (no seconds → `M/d/yyyy`).
    General,
    /// `LongDate`: `dddd, MMMM d, yyyy` — approximated without the
    /// weekday/month names table for `MMMM d, yyyy`.
    Long,
    /// `ShortDate`: `M/d/yyyy`.
    Short,
}

/// Render `Date`/`DateTime` per `fmt`; anything else is missing.
fn fmt_date(v: &EV, fmt: DateFmt) -> EV {
    let secs = match v {
        EV::Date(d) => Some(i64::from(*d) * 86_400),
        EV::DateTime(s) => Some(*s),
        _ => None,
    };
    let Some(secs) = secs else { return EV::Missing };
    let (y, m, d, h, mi, s) = date::civil_from_secs(secs);
    let month_name = MONTH_NAMES[(m - 1) as usize];
    EV::Text(match fmt {
        DateFmt::Short => format!("{m}/{d}/{y}"),
        DateFmt::Long => format!("{month_name} {d}, {y}"),
        DateFmt::General => {
            let tod = secs.rem_euclid(86_400);
            if tod == 0 {
                format!("{m}/{d}/{y}")
            } else {
                let (hh, tt) = am_pm(h);
                format!("{m}/{d}/{y} {hh}:{mi:02}:{s:02} {tt}")
            }
        }
    })
}

/// `LongTime` (`h:mm:ss tt`) / `ShortTime` (`h:mm tt`).
fn fmt_time(v: &EV, short: bool) -> EV {
    let secs = match v {
        EV::Time(t) => Some(i64::from(*t)),
        EV::DateTime(s) => Some(s.rem_euclid(86_400)),
        EV::Date(_) => Some(0),
        _ => None,
    };
    let Some(t) = secs else { return EV::Missing };
    let (h, mi, s) = ((t / 3600) as u32, (t / 60 % 60) as u32, (t % 60) as u32);
    let (hh, tt) = am_pm(h);
    EV::Text(if short {
        format!("{hh}:{mi:02} {tt}")
    } else {
        format!("{hh}:{mi:02}:{s:02} {tt}")
    })
}

/// `h` (0–23) → `(hour-of-12, "AM"/"PM")`.
fn am_pm(h: u32) -> (u32, &'static str) {
    match h {
        0 => (12, "AM"),
        12 => (12, "PM"),
        1..=11 => (h, "AM"),
        _ => (h - 12, "PM"),
    }
}

const MONTH_NAMES: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// `int.Parse` — the strict parse `SUBSTRING` uses: integer text or a
/// whole-valued number; reals with a fraction fail.
fn coerce_int_strict(v: &EV) -> Option<i64> {
    match v {
        EV::Int(i) => Some(*i),
        EV::Real(x) if x.fract() == 0.0 => Some(*x as i64),
        EV::Text(s) => s.trim().parse::<i64>().ok(),
        _ => None,
    }
}
