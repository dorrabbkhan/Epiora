//! Type inference over all rows of a column (spec §6.2): candidates in
//! order Boolean → Integer → Real → Date → Text; the first that every
//! non-missing value fits wins (so an all-missing column is Boolean).
//! These predicates are also the coercion rules — inference and
//! coercion can never disagree.

use crate::settings::{DateFormat, DecimalSep};
use epiora_core::dataset::ValueType;

/// Inference/coercion context: decimal separator and date grammar.
pub(crate) struct InferCtx {
    /// Decimal separator for real literals.
    pub decimal: DecimalSep,
    /// Accepted date grammar.
    pub date_format: DateFormat,
}

/// ASCII-whitespace trim — missing codes match after it; stored values
/// keep their original whitespace.
pub(crate) fn ascii_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_ascii_whitespace())
}

/// Whether a raw field is a missing value: empty after trim, or equal
/// (after trim) to one of `missing_codes`.
pub(crate) fn is_missing(s: &str, missing_codes: &[String]) -> bool {
    let t = ascii_trim(s);
    t.is_empty() || missing_codes.iter().any(|c| c == t)
}

/// Infers the column type from its non-missing values (spec §6.2).
pub(crate) fn infer(non_missing: &[&str], ctx: &InferCtx) -> ValueType {
    if non_missing.iter().all(|v| fits_boolean(v)) {
        ValueType::Boolean
    } else if non_missing.iter().all(|v| fits_integer(v)) {
        ValueType::Integer
    } else if non_missing
        .iter()
        .all(|v| parse_real(v, ctx.decimal).is_some())
    {
        ValueType::Real
    } else if non_missing
        .iter()
        .all(|v| parse_date(v, ctx.date_format).is_some())
    {
        ValueType::Date
    } else {
        ValueType::Text
    }
}

/// `true|TRUE|yes` → `true`; `false|FALSE|no` → `false` (spec §6.2).
/// `0/1`, `Yes`, `on` and friends are not booleans.
pub(crate) fn fits_boolean(s: &str) -> bool {
    matches!(s, "true" | "TRUE" | "yes" | "false" | "FALSE" | "no")
}

/// Parses a boolean literal; `None` when it is not one.
pub(crate) fn parse_boolean(s: &str) -> Option<bool> {
    match s {
        "true" | "TRUE" | "yes" => Some(true),
        "false" | "FALSE" | "no" => Some(false),
        _ => None,
    }
}

/// Optional sign + digits fitting i64; overflow falls through to Real.
pub(crate) fn fits_integer(s: &str) -> bool {
    s.parse::<i64>().is_ok()
}

/// Parses a real literal (spec §6.2): after decimal normalisation only
/// `[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?` qualifies — `inf`, `nan`,
/// thousands separators and non-finite results are not reals.
pub(crate) fn parse_real(s: &str, decimal: DecimalSep) -> Option<f64> {
    let owned;
    let s = match decimal {
        DecimalSep::Comma if s.contains(',') => {
            owned = s.replace(',', ".");
            &owned
        }
        _ => s,
    };
    if !is_real_literal(s) {
        return None;
    }
    s.parse::<f64>().ok().filter(|x| x.is_finite())
}

/// The literal grammar `[+-]?(\d+\.?\d*|\.\d+)([eE][+-]?\d+)?`.
fn is_real_literal(s: &str) -> bool {
    let s = s
        .strip_prefix('+')
        .or_else(|| s.strip_prefix('-'))
        .unwrap_or(s);
    let (mantissa, exp) = match s.find(['e', 'E']) {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    if let Some(e) = exp {
        let e = e
            .strip_prefix('+')
            .or_else(|| e.strip_prefix('-'))
            .unwrap_or(e);
        if e.is_empty() || !e.bytes().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    if let Some(frac) = mantissa.strip_prefix('.') {
        // \.\d+
        return !frac.is_empty() && frac.bytes().all(|c| c.is_ascii_digit());
    }
    match mantissa.find('.') {
        // \d+
        None => !mantissa.is_empty() && mantissa.bytes().all(|c| c.is_ascii_digit()),
        // \d+\.\d*
        Some(i) => {
            let (int, frac) = (&mantissa[..i], &mantissa[i + 1..]);
            !int.is_empty()
                && int.bytes().all(|c| c.is_ascii_digit())
                && frac.bytes().all(|c| c.is_ascii_digit())
        }
    }
}

/// Parses a date literal to days since 1970-01-01 (spec §6.2): ISO
/// `YYYY-MM-DD`; `dmy`/`mdy` accept 1–2 digit day/month, 4-digit year,
/// one consistent `/`, `-` or `.` separator — and only when picked.
pub(crate) fn parse_date(s: &str, format: DateFormat) -> Option<i32> {
    match format {
        DateFormat::Iso => {
            let b = s.as_bytes();
            if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
                return None;
            }
            let (y, m, d) = (
                fixed_num(&s[0..4])?,
                fixed_num(&s[5..7])?,
                fixed_num(&s[8..10])?,
            );
            days_from_parts(y, m, d)
        }
        DateFormat::Dmy => parse_dmy_mdy(s, true),
        DateFormat::Mdy => parse_dmy_mdy(s, false),
    }
}

fn parse_dmy_mdy(s: &str, dmy: bool) -> Option<i32> {
    for sep in ['/', '-', '.'] {
        let parts: Vec<&str> = s.split(sep).collect();
        if parts.len() != 3 {
            continue;
        }
        let a = flex_num(parts[0], 1, 2)?;
        let b = flex_num(parts[1], 1, 2)?;
        let y = flex_num(parts[2], 4, 4)?;
        let (d, m) = if dmy { (a, b) } else { (b, a) };
        return days_from_parts(y, m, d);
    }
    None
}

/// `len` exactly digits.
fn fixed_num(s: &str) -> Option<i64> {
    if s.bytes().all(|c| c.is_ascii_digit()) {
        s.parse::<i64>().ok()
    } else {
        None
    }
}

/// `min..=max` digits.
fn flex_num(s: &str, min: usize, max: usize) -> Option<i64> {
    if (min..=max).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_digit()) {
        s.parse::<i64>().ok()
    } else {
        None
    }
}

fn days_from_parts(y: i64, m: i64, d: i64) -> Option<i32> {
    if !(1..=12).contains(&m) {
        return None;
    }
    let days_in_month = match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(y) => 29,
        _ => 28,
    };
    if !(1..=days_in_month).contains(&d) {
        return None;
    }
    i32::try_from(days_from_civil(y, m, d)).ok()
}

fn is_leap(y: i64) -> bool {
    y % 4 == 0 && (y % 100 != 0 || y % 400 == 0)
}

/// Howard Hinnant's `days_from_civil`: proleptic-Gregorian
/// `YYYY-MM-DD` → days since 1970-01-01 (same formula as epiora-core).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}
