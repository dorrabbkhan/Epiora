//! Text functions — `UPPERCASE`, `SUBSTRING`, `STRLEN`, `FINDTEXT`,
//! `FORMAT` — ported with their `ToString()`/`TryParse` coercions and
//! EI7's null/empty results.

use crate::expr::missing::{ei_to_string, is_empty, EV};
use crate::functions::calendar;

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

/// `FORMAT(value[, pattern])` — upstream `Epi.Core.Interpreter/Rules/
/// Functions/Rule_Format.cs` @ `4cd353c` lines 27–45 (Apache-2.0, D2)
/// delegates to `Microsoft.VisualBasic.Strings.Format`. The digit pictures
/// below implement VB/.NET's `0`/`#`/`.` custom numeric specifiers without
/// a locale read: `.` is the decimal separator, `0` is a mandatory digit,
/// `#` is optional, and rounding is half away from zero (VB Format). The
/// named date/time formats (`GeneralDate`, `LongDate`, `ShortDate`,
/// `LongTime`, `ShortTime`) are applied as a method choice; anything else
/// keeps the raw `ToString()` fallback (decisions.md).
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
        if let Some(text) = format_digit_picture(&v[0], &pattern) {
            return EV::Text(text);
        }
    }
    EV::Text(ei_to_string(&v[0]))
}

/// A `0`/`#` digit picture split at its one decimal point; bytes are the
/// ASCII placeholder characters.
struct DigitPicture {
    integer: Vec<u8>,
    fraction: Vec<u8>,
}

/// Parse the simple custom numeric pictures this port supports. Anything
/// else (separators, scaling, percent, sections, literals, exponent
/// markers) is not a digit picture here and keeps `format`'s fallback.
fn parse_digit_picture(pattern: &str) -> Option<DigitPicture> {
    let mut integer = Vec::new();
    let mut fraction = Vec::new();
    let mut seen_point = false;
    let mut digits = 0usize;
    for b in pattern.bytes() {
        match b {
            b'0' | b'#' => {
                digits += 1;
                if seen_point {
                    fraction.push(b);
                } else {
                    integer.push(b);
                }
            }
            b'.' if !seen_point => seen_point = true,
            _ => return None,
        }
    }
    if digits == 0 || fraction.len() > 99 {
        return None;
    }
    Some(DigitPicture { integer, fraction })
}

/// Format an integer/real input with a digit picture. Non-numeric and
/// non-finite inputs have no VB numeric formatting here, so the caller
/// falls back to `ToString()`.
fn format_digit_picture(v: &EV, pattern: &str) -> Option<String> {
    let (negative, magnitude) = match v {
        // Integer formatting is exact decimal formatting in .NET — do
        // not round-trip it through `f64` and lose digits above 2^53.
        EV::Int(i) => (*i < 0, i.unsigned_abs().to_string()),
        EV::Real(x) if x.is_finite() => (
            x.is_sign_negative() && *x != 0.0,
            format!("{}", libm::fabs(*x)),
        ),
        _ => return None,
    };
    let picture = parse_digit_picture(pattern)?;
    let rounded = rounded_fixed_digits(&magnitude, picture.fraction.len())?;
    let nonzero = rounded
        .integer
        .iter()
        .chain(rounded.fraction.iter())
        .any(|d| *d != 0);
    let integer = integer_text(&picture.integer, &rounded.integer);
    let fraction = fraction_text(&picture.fraction, &rounded.fraction);
    let mut out = String::new();
    if negative && nonzero {
        out.push('-');
    }
    out.push_str(&integer);
    if !fraction.is_empty() {
        out.push('.');
        out.push_str(&fraction);
    }
    Some(out)
}

/// Decimal digits after rounding the positive decimal text to `places`
/// fraction digits, half away from zero. Real input text is Rust's
/// locale-independent, round-trippable decimal rendering; the rounding
/// itself is digit arithmetic, so no platform float formatting decides a
/// tie.
struct FixedDigits {
    integer: Vec<u8>,
    fraction: Vec<u8>,
}

fn rounded_fixed_digits(text: &str, places: usize) -> Option<FixedDigits> {
    let (int_text, frac_text) = match text.split_once('.') {
        Some((i, f)) => (i, f),
        None => (text, ""),
    };
    let mut integer = decimal_digits(int_text)?;
    let source_fraction = decimal_digits(frac_text)?;
    let mut fraction = vec![0u8; places];
    for (slot, digit) in fraction.iter_mut().zip(source_fraction.iter()) {
        *slot = *digit;
    }
    if source_fraction.get(places).is_some_and(|d| *d >= 5) {
        round_up_digits(&mut integer, &mut fraction);
    }
    Some(FixedDigits { integer, fraction })
}

/// ASCII decimal digits as values 0–9; anything else rejects the input.
fn decimal_digits(s: &str) -> Option<Vec<u8>> {
    s.bytes()
        .map(|b| match b {
            b'0'..=b'9' => Some(b - b'0'),
            _ => None,
        })
        .collect()
}

/// Increment the fixed digit pair, carrying from the fraction into the
/// integer part when every fraction digit rolls over.
fn round_up_digits(integer: &mut Vec<u8>, fraction: &mut [u8]) {
    for d in fraction.iter_mut().rev() {
        if *d < 9 {
            *d += 1;
            return;
        }
        *d = 0;
    }
    increment_digits(integer);
}

fn increment_digits(digits: &mut Vec<u8>) {
    for d in digits.iter_mut().rev() {
        if *d < 9 {
            *d += 1;
            return;
        }
        *d = 0;
    }
    digits.insert(0, 1);
}

/// Align the rounded integer digits with the picture's integer placeholders
/// from the right. Extra digits prepend; a zero integer part is significant
/// only where a `0` placeholder forces it.
fn integer_text(placeholders: &[u8], digits: &[u8]) -> String {
    let significant: &[u8] = if digits.iter().any(|d| *d != 0) {
        digits
    } else {
        &[]
    };
    let mut pos = significant.len();
    let mut reversed = Vec::new();
    for ph in placeholders.iter().rev() {
        if pos > 0 {
            pos -= 1;
            if let Some(d) = significant.get(pos) {
                reversed.push(char::from(b'0' + *d));
            }
        } else if *ph == b'0' {
            reversed.push('0');
        }
    }
    while pos > 0 {
        pos -= 1;
        if let Some(d) = significant.get(pos) {
            reversed.push(char::from(b'0' + *d));
        }
    }
    reversed.into_iter().rev().collect()
}

/// Render fraction digits through the placeholders: `0` always shows its
/// position; `#` shows significant digits and internal zeroes but drops
/// trailing zeroes. A zero placeholder also makes every `#` to its left
/// mandatory, as VB/.NET's rightmost-zero rule does.
fn fraction_text(placeholders: &[u8], digits: &[u8]) -> String {
    let mut last_significant = None;
    for (i, d) in digits.iter().enumerate() {
        if *d != 0 {
            last_significant = Some(i);
        }
    }
    let mut last_mandatory = None;
    for (i, ph) in placeholders.iter().enumerate() {
        if *ph == b'0' {
            last_mandatory = Some(i);
        }
    }
    let mut out = String::new();
    for (i, _ph) in placeholders.iter().enumerate() {
        let significant = last_significant.is_some_and(|last| last >= i);
        let mandatory = last_mandatory.is_some_and(|last| last >= i);
        if significant || mandatory {
            let digit = match digits.get(i) {
                Some(d) => *d,
                None => 0,
            };
            out.push(char::from(b'0' + digit));
        }
    }
    out
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
    let (y, m, d, h, mi, s) = calendar::civil_from_secs(secs);
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
