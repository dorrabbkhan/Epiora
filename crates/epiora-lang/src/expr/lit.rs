//! Numeric, date and time literal scanning — the grammar's
//! `DecLiteral`/`RealLiteral`/`Date`/`Time` terminals share one
//! digit run, so `lex` hands them off to `lex_number` (length) and
//! `number`/`real` (classification), exactly as the GOLD lexer
//! prefers `Date`/`Time` before numbers (`4/18/1940` is a date,
//! not `4 / 18 / 1940`).

use super::lexer::Tok;
use crate::LangError;

/// Scan a number/date/time run starting at `chars[start]` (a digit, or
/// `.` already consumed). Returns the exclusive end index.
pub(crate) fn lex_number(chars: &[char], start: usize, after_dot: bool) -> usize {
    let i = start;
    let digit = |k: usize| chars.get(k).is_some_and(|c| c.is_ascii_digit());
    // Date: D{1,2}[-/]D{1,2}[-/]D{1,4} — contiguous.
    if digit(i) {
        let mut n = 0;
        while n < 2 && digit(i + n) {
            n += 1;
        }
        if matches!(chars.get(i + n), Some('-') | Some('/')) {
            let sep = i + n;
            let mut m = 0;
            while m < 2 && digit(sep + 1 + m) {
                m += 1;
            }
            if m > 0 && matches!(chars.get(sep + 1 + m), Some('-') | Some('/')) {
                let sep2 = sep + 1 + m;
                let mut y = 0;
                while y < 4 && digit(sep2 + 1 + y) {
                    y += 1;
                }
                return sep2 + 1 + y;
            }
        }
        // Time: D{1,2}:D{1,2}(:D{1,2})? — contiguous.
        if n > 0 && chars.get(i + n) == Some(&':') {
            let colon = i + n;
            let mut m = 0;
            while m < 2 && digit(colon + 1 + m) {
                m += 1;
            }
            if m > 0 {
                let mut end = colon + 1 + m;
                if chars.get(end) == Some(&':') {
                    let mut s = 0;
                    while s < 2 && digit(end + 1 + s) {
                        s += 1;
                    }
                    if s > 0 {
                        end += 1 + s;
                    }
                }
                return end;
            }
        }
        // Integer or real: digits [.digits] [E[+-]digits] [UL suffix].
        let mut end = i + n;
        while digit(end) {
            end += 1;
        }
        if chars.get(end) == Some(&'.') && digit(end + 1) {
            end += 1;
            while digit(end) {
                end += 1;
            }
        }
        if matches!(chars.get(end), Some('e') | Some('E'))
            && (digit(end + 1)
                || (matches!(chars.get(end + 1), Some('+') | Some('-')) && digit(end + 2)))
        {
            end += 1;
            if matches!(chars.get(end), Some('+') | Some('-')) {
                end += 1;
            }
            while digit(end) {
                end += 1;
            }
        }
        return end;
    }
    // after_dot: `.` consumed; digits [E…] remain.
    debug_assert!(after_dot);
    let mut end = i;
    while digit(end) {
        end += 1;
    }
    if matches!(chars.get(end), Some('e') | Some('E'))
        && (digit(end + 1)
            || (matches!(chars.get(end + 1), Some('+') | Some('-')) && digit(end + 2)))
    {
        end += 1;
        if matches!(chars.get(end), Some('+') | Some('-')) {
            end += 1;
        }
        while digit(end) {
            end += 1;
        }
    }
    end
}

/// Classify a run lexed by [`lex_number`]: date, time, integer or real.
pub(crate) fn number(text: &str, line: u32, column: u32) -> Result<Tok, LangError> {
    // Date?  M/D/[Y]Y — month-first en-US, like `DateTime.TryParse`.
    if let Some(days) = parse_date_text(text) {
        return Ok(Tok::DateLit(days));
    }
    if text.contains(':') {
        return match parse_time_text(text) {
            Some(secs) => Ok(Tok::TimeLit(Some(secs))),
            // Out-of-range times are not the `Time` terminal at all —
            // EI7 lexes the `:` as an unexpected character.
            None => Err(LangError::Syntax {
                line,
                column,
                message: format!("invalid time literal {text:?}"),
            }),
        };
    }
    // U/L integer suffixes.
    let lower = text.to_ascii_lowercase();
    let (digits, is_int) = match lower.as_str() {
        s if s.ends_with("ul") | s.ends_with("lu") => (&s[..s.len() - 2], true),
        s if s.ends_with('u') | s.ends_with('l') => (&s[..s.len() - 1], true),
        s => (s, false),
    };
    if is_int || digits.bytes().all(|b| b.is_ascii_digit()) {
        if let Ok(i) = digits.parse::<i64>() {
            return Ok(Tok::IntLit(i));
        }
    }
    real(text, line, column)
}

pub(crate) fn real(text: &str, line: u32, column: u32) -> Result<Tok, LangError> {
    match text.parse::<f64>() {
        Ok(x) if x.is_finite() => Ok(Tok::RealLit(x)),
        // i64 overflow and other non-finite spellings still parse as
        // doubles in EI7.
        Ok(x) => Ok(Tok::RealLit(x)),
        Err(_) => Err(LangError::Syntax {
            line,
            column,
            message: format!("malformed number {text:?}"),
        }),
    }
}

/// `M/D/YY…` → days since 1970-01-01; `Some(None)` when the text is a
/// date-shaped string with an invalid calendar date (EI7 → missing),
/// `None` when it is not date-shaped at all.
fn parse_date_text(text: &str) -> Option<Option<i32>> {
    let sep = ['/', '-'];
    let mut parts = text.split(sep.as_ref());
    let (Some(m), Some(d), Some(y), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
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
    Some(crate::functions::date::days_from_civil_checked(y, m, d))
}

/// `H:MM[:SS]` → seconds since midnight within the `Time` terminal's
/// ranges; `None` outside them.
fn parse_time_text(text: &str) -> Option<i32> {
    let mut parts = text.split(':');
    let (Some(h), Some(m), s, None) = (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let (Ok(h), Ok(m), Ok(s)) = (
        h.parse::<u32>(),
        m.parse::<u32>(),
        s.unwrap_or("0").parse::<u32>(),
    ) else {
        return None;
    };
    if h > 23 || m > 59 || s > 59 {
        return None;
    }
    Some((h * 3600 + m * 60 + s) as i32)
}
