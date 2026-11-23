//! Cell → typed-value conversion and Access-datetime arithmetic
//! (ports `legacy-values.mjs`: `convertCell`, `isIntegralColumn`,
//! `parseDateTime`, `countBlocks`, and `mdb-export -T '%Y-%m-%d
//! %H:%M:%S'`'s serial formatting). All float maths goes through
//! `libm`.

use crate::source::Cell;

/// Access serial → the `YYYY-MM-DD HH:MM:SS` text `mdb-export` emits.
/// Mirrors `mdb_date_to_tm`: the day is the truncated serial, the
/// time-of-day is `round(frac * 86400)` seconds (no day carry — a
/// rounding-up fraction prints `24:00:00`, as mdbtools does), and
/// serials `< 0` or `> 1e6` early-return leaving the caller's zeroed
/// `struct tm` — strftime prints `"1900-01-00 00:00:00"` (tm_year 0 →
/// 1900, tm_mday 0 → `00`). The golden stores that text verbatim, so
/// an out-of-range serial maps to the same degenerate date, never to
/// missing.
pub fn serial_to_text(serial: f64) -> String {
    // NaN/inf serials also take the out-of-range arm: `contains` is
    // false for them, and mdbtools' `td < 0 || td > 1e6` guard
    // produces no defined output either way.
    if !(0.0..=1e6).contains(&serial) {
        return "1900-01-00 00:00:00".to_string();
    }
    let day = libm::trunc(serial) as i64;
    let secs = libm::floor((serial - day as f64) * 86400.0 + 0.5) as i64;
    let (y, m, d) = civil_from_days(day - 25569);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// Howard Hinnant's `days_from_civil` — days since 1970-01-01 (same
/// port `epiora-core` and the golden's emit.mjs use).
pub(crate) fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let yy = if m <= 2 { y - 1 } else { y };
    let era = if yy >= 0 { yy } else { yy - 399 } / 400;
    let yoe = yy - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Howard Hinnant's `civil_from_days` — inverse of [`days_from_civil`]
/// (z counts days from 1970-01-01).
pub(crate) fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// The golden's `parseDateTime`: `YYYY-MM-DD` optionally followed by
/// `[ T]HH:MM(:SS)?` and trailing whitespace → (days since epoch,
/// seconds since midnight). Trailing `\s*` tolerated, leading is not.
pub(crate) fn parse_date_time(s: &str) -> Option<(i64, i64)> {
    let b = s.as_bytes();
    let digits = |i: usize, n: usize| -> Option<i64> {
        let part = b.get(i..i + n)?;
        if part.iter().all(|c| c.is_ascii_digit()) {
            std::str::from_utf8(part).ok()?.parse().ok()
        } else {
            None
        }
    };
    if b.len() < 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    let (y, mo, d) = (digits(0, 4)?, digits(5, 2)?, digits(8, 2)?);
    // The golden's `\d{2}` fields carry no range check ("1900-01-00"
    // is a stored value); `days_from_civil` normalises out-of-range
    // month/day numbers the same way the golden's `dateToDays` does.
    let days = days_from_civil(y, mo as u32, d as u32);
    let mut i = 10;
    if i == b.len() {
        return Some((days, 0));
    }
    if b[i] != b'T' && b[i] != b' ' {
        return None;
    }
    i += 1;
    if digits(i, 2).is_none() || b.get(i + 2) != Some(&b':') || digits(i + 3, 2).is_none() {
        return None;
    }
    let h = digits(i, 2)?;
    let mi = digits(i + 3, 2)?;
    i += 5;
    let mut sec = 0;
    if b.get(i) == Some(&b':') {
        sec = digits(i + 1, 2)?;
        i += 3;
    }
    // trailing whitespace only
    if !b[i..].iter().all(|c| c.is_ascii_whitespace()) {
        return None;
    }
    Some((days, h * 3600 + mi * 60 + sec))
}

/// The golden's `time` fallback: `String(raw).slice(0, 8)` when the
/// datetime pattern fails — then that string still has to parse as a
/// clock for Epiora's `Time` (the golden stores it raw; an unparseable
/// suffix would fail its own emit, so this only accepts `HH:MM(:SS)?`).
fn parse_clock_prefix(s: &str) -> Option<i64> {
    let head = &s[..s.len().min(8)];
    let b = head.as_bytes();
    if b.len() < 5 || b.get(2) != Some(&b':') {
        return None;
    }
    let h: i64 = b[0..2]
        .iter()
        .all(|c| c.is_ascii_digit())
        .then(|| std::str::from_utf8(&b[0..2]).ok()?.parse().ok())??;
    let m: i64 = b[3..5]
        .iter()
        .all(|c| c.is_ascii_digit())
        .then(|| std::str::from_utf8(&b[3..5]).ok()?.parse().ok())??;
    let sec = if b.get(5) == Some(&b':') && b.len() >= 8 {
        if !b[6..8].iter().all(|c| c.is_ascii_digit()) {
            return None;
        }
        std::str::from_utf8(&b[6..8]).ok()?.parse().ok()?
    } else {
        0
    };
    Some(h * 3600 + m * 60 + sec)
}

/// A converted typed cell (the `ColumnData` payload types).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Typed {
    /// `integer`
    Int(i64),
    /// `real`
    Real(f64),
    /// `text`
    Text(String),
    /// `boolean`
    Bool(bool),
    /// `date` — days since epoch
    Date(i32),
    /// `time` — seconds since midnight
    Time(i32),
    /// `date_time` — seconds since epoch
    DateTime(i64),
}

/// `convertCell(raw, type)` — the golden's per-type conversion, but
/// returning engine payloads (ints/epoch counts) instead of JSON
/// surface values. `raw` `None`/empty-string → missing.
pub(crate) fn convert_cell(raw: Option<&Cell>, ty: &str) -> Option<Typed> {
    let c = raw?;
    if matches!(c, Cell::Text(s) if s.is_empty()) {
        return None;
    }
    match ty {
        "integer" => {
            let n = c.as_number();
            (n.is_finite() && n >= i64::MIN as f64 && n <= i64::MAX as f64)
                .then(|| Typed::Int(libm::trunc(n) as i64))
        }
        "real" => {
            let n = c.as_number();
            n.is_finite()
                .then_some(Typed::Real(if n == 0.0 { 0.0 } else { n }))
        }
        "boolean" => {
            let n = c.as_number();
            n.is_finite().then_some(Typed::Bool(n != 0.0))
        }
        "text" => Some(Typed::Text(c.as_string())),
        "date" => {
            let s = c.as_string();
            parse_date_time(&s)
                .and_then(|(d, _)| i32::try_from(d).ok())
                .map(Typed::Date)
        }
        "time" => {
            let s = c.as_string();
            match parse_date_time(&s) {
                Some((_, t)) => i32::try_from(t).ok().map(Typed::Time),
                None => parse_clock_prefix(&s)
                    .and_then(|t| i32::try_from(t).ok())
                    .map(Typed::Time),
            }
        }
        "date_time" => {
            let s = c.as_string();
            parse_date_time(&s).map(|(d, t)| Typed::DateTime(d * 86400 + t))
        }
        _ => None,
    }
}

/// `isIntegralColumn(values)`: every value missing/empty or an
/// integer-valued `Number(·)` — drives the Number field integer/real
/// split.
pub(crate) fn is_integral_column(values: &[Option<Cell>]) -> bool {
    values.iter().all(|v| match v {
        None => true,
        Some(Cell::Text(s)) if s.is_empty() => true,
        Some(c) => {
            let n = c.as_number();
            n.is_finite() && libm::trunc(n) == n
        }
    })
}

/// `countBlocks(text)` — the golden's `BLOCK_RE` count: per line,
/// optional whitespace then `DefineVariables`/`Form`/`View`/`Record`
/// (word boundary), `Page [<…>]`, or `Field <word>`, case-insensitive.
pub(crate) fn count_blocks(text: &str) -> u32 {
    let mut n = 0;
    for line in text.lines() {
        if is_block_header(line.trim_start()) {
            n += 1;
        }
    }
    n
}

fn is_block_header(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    let word = |w: &str| -> bool {
        match lower.strip_prefix(w) {
            None => false,
            Some(rest) => match rest.chars().next() {
                None => true,
                Some(c) => !c.is_alphanumeric() && c != '_',
            },
        }
    };
    if word("definevariables") || word("form") || word("view") || word("record") {
        return true;
    }
    if let Some(rest) = lower.strip_prefix("page") {
        let rest = rest.trim_start();
        return rest.starts_with('[') && rest[1..].find(']').is_some();
    }
    if let Some(rest) = lower.strip_prefix("field") {
        // `Field\s+\S+`: whitespace, then at least one non-space char.
        return rest.starts_with(|c: char| c.is_whitespace()) && !rest.trim_start().is_empty();
    }
    false
}
