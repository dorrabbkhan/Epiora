//! Date and time functions; the civil-calendar and MMWR arithmetic
//! they share lives in [`super::calendar`].

use super::calendar::{
    civil_from_days, civil_from_secs, days_from_civil_checked, mmwr_start, secs_from_civil_checked,
};
use crate::expr::eval::Ctx;
use crate::expr::missing::{coerce_f64, ei_to_string, EV};

/// Civil `(y, m, d)` of a `Date`/`DateTime` operand.
fn civil(v: &EV) -> Option<(i32, u32, u32)> {
    match v {
        EV::Date(d) => Some(civil_from_days(*d)),
        EV::DateTime(s) => {
            let (y, m, d, ..) = civil_from_secs(*s);
            Some((y, m, d))
        }
        _ => None,
    }
}

/// Epoch-seconds of a `Date`/`DateTime` operand (`is DateTime`).
fn secs(v: &EV) -> Option<i64> {
    match v {
        EV::Date(d) => Some(i64::from(*d) * 86_400),
        EV::DateTime(s) => Some(*s),
        _ => None,
    }
}

/// `DAY` — `DateTime.Day`.
pub(crate) fn day(v: &EV) -> EV {
    civil(v).map_or(EV::Missing, |(_, _, d)| EV::Int(i64::from(d)))
}

/// `MONTH` — `DateTime.Month`.
pub(crate) fn month(v: &EV) -> EV {
    civil(v).map_or(EV::Missing, |(_, m, _)| EV::Int(i64::from(m)))
}

/// `YEAR` — `DateTime.Year`.
pub(crate) fn year(v: &EV) -> EV {
    civil(v).map_or(EV::Missing, |(y, _, _)| EV::Int(i64::from(y)))
}

/// `HOUR` — `DateTime.Hour`, or `Floor(TimeSpan.TotalHours)`.
pub(crate) fn hour(v: &EV) -> EV {
    match v {
        EV::Time(t) => EV::Int(i64::from(t / 3600)),
        _ => match v {
            EV::DateTime(_) | EV::Date(_) => {
                let s = secs(v).unwrap_or(0);
                EV::Int(s.rem_euclid(86_400) / 3600)
            }
            _ => EV::Missing,
        },
    }
}

/// `MINUTE` — `DateTime.Minute` (no `TimeSpan` arm in EI7).
pub(crate) fn minute(v: &EV) -> EV {
    secs(v).map_or(EV::Missing, |s| EV::Int(s.rem_euclid(86_400) / 60 % 60))
}

/// `SECOND` — `DateTime.Second` (no `TimeSpan` arm in EI7).
pub(crate) fn second(v: &EV) -> EV {
    secs(v).map_or(EV::Missing, |s| EV::Int(s.rem_euclid(60)))
}

/// `DAYS(a, b)` — `(b − a).Days`, the whole-day component.
pub(crate) fn days(a: &EV, b: &EV) -> EV {
    match (secs(a), secs(b)) {
        (Some(x), Some(y)) => EV::Int((y - x) / 86_400),
        _ => EV::Missing,
    }
}

/// `MONTHS(a, b)` — `12·(y2−y1) + m2−m1`, minus one when `d2 < d1`.
pub(crate) fn months(a: &EV, b: &EV) -> EV {
    match (civil(a), civil(b)) {
        (Some((y1, m1, d1)), Some((y2, m2, d2))) => {
            let mut apart = 12 * (i64::from(y2) - i64::from(y1)) + i64::from(m2) - i64::from(m1);
            if d2 < d1 {
                apart -= 1;
            }
            EV::Int(apart)
        }
        _ => EV::Missing,
    }
}

/// `YEARS(a, b)` — the age formula: `y2−y1`, minus one when `b`'s
/// month-day falls before `a`'s.
pub(crate) fn years(a: &EV, b: &EV) -> EV {
    match (civil(a), civil(b)) {
        (Some((y1, m1, d1)), Some((y2, m2, d2))) => {
            let mut age = i64::from(y2) - i64::from(y1);
            if m2 < m1 || (m2 == m1 && d2 < d1) {
                age -= 1;
            }
            EV::Int(age)
        }
        _ => EV::Missing,
    }
}

/// `HOURS(a, b)` — `Math.Round((b−a).TotalHours)` (round-half-even).
pub(crate) fn hours(a: &EV, b: &EV) -> EV {
    match (secs(a), secs(b)) {
        (Some(x), Some(y)) => EV::Real(round_half_even((y - x) as f64 / 3600.0)),
        _ => EV::Missing,
    }
}

/// `MINUTES(a, b)` — `(b−a).TotalMinutes`.
pub(crate) fn minutes(a: &EV, b: &EV) -> EV {
    match (secs(a), secs(b)) {
        (Some(x), Some(y)) => EV::Real((y - x) as f64 / 60.0),
        _ => EV::Missing,
    }
}

/// `SECONDS(a, b)` — `(b−a).TotalSeconds`.
pub(crate) fn seconds(a: &EV, b: &EV) -> EV {
    match (secs(a), secs(b)) {
        (Some(x), Some(y)) => EV::Real((y - x) as f64),
        _ => EV::Missing,
    }
}

/// `Math.Round`'s default — half-to-even.
fn round_half_even(x: f64) -> f64 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 && r % 2.0 != 0.0 {
        r - x.signum()
    } else {
        r
    }
}

/// `NUMTODATE(y, m, d)` — `Int32.TryParse` each part, the 0–29→2000s /
/// 30–99→1900s pivot, `new DateTime` guarded → invalid is missing.
pub(crate) fn num_to_date(y: &EV, m: &EV, d: &EV) -> EV {
    let (Some(y), Some(m), Some(d)) = (coerce_int(y), coerce_int(m), coerce_int(d)) else {
        return EV::Missing;
    };
    let y = match y {
        0..=29 => y + 2000,
        30..=99 => y + 1900,
        _ => y,
    };
    let Ok(y) = i32::try_from(y) else {
        return EV::Missing;
    };
    let (Ok(m), Ok(d)) = (u32::try_from(m), u32::try_from(d)) else {
        return EV::Missing;
    };
    days_from_civil_checked(y, m, d).map_or(EV::Missing, EV::Date)
}

/// `NUMTOTIME(h, m, s)` — `new TimeSpan(h, m, s)`; values outside the
/// `Time` column's `0..=86399` cannot exist in our model, so they are
/// missing rather than the TimeSpan EI7 would carry.
pub(crate) fn num_to_time(h: &EV, m: &EV, s: &EV) -> EV {
    // `int.TryParse` defaults unparseable parts to 0.
    let (h, m, s) = (
        coerce_int(h).unwrap_or(0),
        coerce_int(m).unwrap_or(0),
        coerce_int(s).unwrap_or(0),
    );
    if !(0..=23).contains(&h) || !(0..=59).contains(&m) || !(0..=59).contains(&s) {
        return EV::Missing;
    }
    EV::Time((h * 3600 + m * 60 + s) as i32)
}

/// `Int32.TryParse(ToString())` — accepts whole-valued reals and
/// numeric text.
fn coerce_int(v: &EV) -> Option<i64> {
    let x = coerce_f64(v)?;
    if x.fract() == 0.0 && x.abs() <= f64::from(i32::MAX) {
        Some(x as i64)
    } else {
        ei_to_string(v).trim().parse::<i64>().ok()
    }
}

/// `TXTTODATE(v)` — `DateTime.TryParse` of the `ToString()`; the
/// original value passes through when it cannot parse (EI7 keeps the
/// parameter as its result).
pub(crate) fn txt_to_date(v: &EV, ctx: &Ctx) -> EV {
    if crate::expr::missing::is_empty(v) {
        return EV::Missing;
    }
    let s = ei_to_string(v);
    match parse_ei_datetime(s.trim(), ctx) {
        Some(secs) => EV::DateTime(secs),
        None => v.clone(),
    }
}

/// A pragmatic `DateTime.TryParse` (en-US): `M/D/YYYY` or `M-D-YYYY`
/// (the grammar's own 2-digit-year pivot), ISO `YYYY-MM-DD`, each with
/// an optional ` h:mm[:ss] [AM|PM]` tail; a bare `h:mm[:ss]` is the
/// as-of date at that time (EI7 uses today; our clock is the as-of).
fn parse_ei_datetime(s: &str, ctx: &Ctx) -> Option<i64> {
    let (date_part, time_part) = split_datetime(s);
    if let Some(days) = parse_ei_date(date_part) {
        let (h, mi, sec) = time_part.map_or(Some((0, 0, 0)), parse_ei_time)?;
        let (y, m, d) = civil_from_days(days);
        return secs_from_civil_checked(y, m, d, h, mi, sec);
    }
    // Bare time on the as-of date.
    if date_part.contains(':') {
        let (h, mi, sec) = parse_ei_time(date_part)?;
        return Some(i64::from(ctx.env.as_of.date) * 86_400 + (h * 3600 + mi * 60 + sec) as i64);
    }
    None
}

/// Split `M/D/YYYY h:mm:ss AM` into date and time parts; AM/PM keeps
/// with the time part.
fn split_datetime(s: &str) -> (&str, Option<&str>) {
    let mut parts = s.splitn(2, ' ');
    let first = parts.next().unwrap_or("");
    match parts.next() {
        Some(rest) => (first, Some(rest.trim())),
        None => (first, None),
    }
}

/// `M/D/YYYY`, `M-D-YYYY` or ISO `YYYY-MM-DD` → days since epoch.
fn parse_ei_date(s: &str) -> Option<i32> {
    // ISO: 4-digit year first.
    if let Some((y, rest)) = s.split_once('-') {
        if y.len() == 4 {
            if let Some((m, d)) = rest.split_once('-') {
                if let (Ok(y), Ok(m), Ok(d)) =
                    (y.parse::<i32>(), m.parse::<u32>(), d.parse::<u32>())
                {
                    return days_from_civil_checked(y, m, d);
                }
            }
        }
    }
    let mut it = s.split(['/', '-']);
    let (Some(m), Some(d), Some(y), None) = (it.next(), it.next(), it.next(), it.next()) else {
        return None;
    };
    let (Ok(m), Ok(d), Ok(y)) = (m.parse::<u32>(), d.parse::<u32>(), y.parse::<i32>()) else {
        return None;
    };
    let y = match y {
        0..=29 => y + 2000,
        30..=99 => y + 1900,
        _ => y,
    };
    days_from_civil_checked(y, m, d)
}

/// `h:mm[:ss][ AM|PM]` → `(h, m, s)`.
fn parse_ei_time(s: &str) -> Option<(u32, u32, u32)> {
    let (s, pm) = match s.to_ascii_uppercase() {
        t if t.ends_with("PM") => (s[..s.len() - 2].trim(), Some(true)),
        t if t.ends_with("AM") => (s[..s.len() - 2].trim(), Some(false)),
        _ => (s, None),
    };
    let mut it = s.split(':');
    let (Some(h), Some(m), sec, None) = (it.next(), it.next(), it.next(), it.next()) else {
        return None;
    };
    let (Ok(mut h), Ok(m), Ok(sec)) = (
        h.parse::<u32>(),
        m.parse::<u32>(),
        sec.unwrap_or("0").parse::<u32>(),
    ) else {
        return None;
    };
    match pm {
        Some(true) if h <= 12 => {
            if h != 12 {
                h += 12;
            }
        }
        Some(false) if h == 12 => h = 0,
        Some(_) => return None,
        None => {}
    }
    if h > 23 || m > 59 || sec > 59 {
        return None;
    }
    Some((h, m, sec))
}

/// `EPIWEEK(date[, first_day])` — the MMWR week, the pinned C# port
/// (`GetMMWRStart` on three candidate years, then `Days/7 + 1`).
/// `first_day` is 1=Sunday..7=Saturday, stored as 0-based.
pub(crate) fn epiweek(v: &[EV]) -> EV {
    let Some(d) = secs(&v[0]) else {
        return EV::Missing;
    };
    let fdw = if v.len() >= 2 {
        // `int.TryParse`: a non-integer leaves the default (0=Sunday);
        // the parsed value shifts by one with no range check.
        coerce_int(&v[1]).map_or(0, |n| (n - 1) as i32)
    } else {
        0
    };
    let days = (d.div_euclid(86_400)) as i32;
    let start = mmwr_start(days, fdw);
    EV::Int(i64::from((days - start) / 7 + 1))
}
