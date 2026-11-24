//! The operator value semantics — `Rule_CompareExp`, `Rule_AddExp`,
//! `Rule_MultExp`, `Rule_ConcatExp` ported branch for branch from
//! `Epi.Core.AnalysisInterpreter/Rules/` at `4cd353c6` (Apache-2.0,
//! D2), over the [`super::missing`] domain. `eval.rs` dispatches
//! nodes to these; nothing here touches the dataset.

use super::ast::BinOp;
use super::missing::{as_f64, ei_to_string, is_empty, text_to_bool, EV};

/// `Rule_CompareExp`, ported branch for branch: the empty checks,
/// the LIKE arm (handled by the caller), the numeric pair, then the
/// mixed-type and same-type arms. Note the ported quirk on the
/// `RHSO is string` arm: EI7 computes `TryValue.CompareTo(LHSO)` —
/// the operands are *reversed*, so `5 < "10"` is false in Epi Info.
pub(crate) fn compare(op: &BinOp, l: &EV, r: &EV) -> bool {
    let le = is_empty(l);
    let re = is_empty(r);
    if le && re {
        return matches!(op, BinOp::Eq);
    }
    if le || re {
        return matches!(op, BinOp::Ne);
    }
    let eq = matches!(op, BinOp::Eq);
    let i = if let (Some(a), Some(b)) = (as_f64(l), as_f64(r)) {
        f64_ord(a, b)
    } else if !same_family(l, r) {
        if let (Some(b), true) = (bool_of(r), eq) {
            // `RHSO is Boolean` and `=`: numeric LHS compares by
            // `!= 0`, anything else by `!IsEmpty` (true here).
            return match as_f64(l) {
                Some(a) => b == (a != 0.0),
                None => b,
            };
        } else if let (EV::Text(s), Some(b)) = (l, as_f64(r)) {
            if let Some(a) = parse_f64(s) {
                f64_ord(a, b)
            } else {
                return false_or_str(op, l, r);
            }
        } else if let (Some(a), EV::Text(s)) = (as_f64(l), r) {
            if let Some(b) = parse_f64(s) {
                // The reversed operand order is EI7's own.
                f64_ord(b, a)
            } else {
                return false_or_str(op, l, r);
            }
        } else if eq && matches!(l, EV::Bool(_)) {
            // `(bool)LHSO == (bool)ConvertStringToBoolean(RHSO)` — an
            // unconvertible RHS crashes in EI7; here it is not equal.
            return matches!((l, bool_value(r)), (EV::Bool(a), Some(b)) if *a == b);
        } else {
            // `StringComparer.CurrentCultureIgnoreCase.Compare` —
            // case-folded ordinal approximation.
            return ord_bool(op, ci(l).cmp(&ci(r)) as i32);
        }
    } else {
        same_ordering(l, r)
    };
    ord_bool(op, i)
}

/// The non-bool mixed-type arms fall through to the culture-ignoring
/// string compare whenever a parse fails.
pub(crate) fn false_or_str(op: &BinOp, l: &EV, r: &EV) -> bool {
    if matches!(op, BinOp::Eq) && matches!(l, EV::Bool(_)) {
        return matches!((l, bool_value(r)), (EV::Bool(a), Some(b)) if *a == b);
    }
    ord_bool(op, ci(l).cmp(&ci(r)) as i32)
}

pub(crate) fn bool_of(v: &EV) -> Option<bool> {
    match v {
        EV::Bool(b) => Some(*b),
        _ => None,
    }
}

/// `ConvertStringToBoolean` on the operand domain: a `Bool` passes,
/// `Text` parses the EI7 spellings, anything else fails.
pub(crate) fn bool_value(v: &EV) -> Option<bool> {
    match v {
        EV::Bool(b) => Some(*b),
        EV::Text(s) => text_to_bool(s),
        _ => None,
    }
}

/// `StringComparer.CurrentCultureIgnoreCase` approximation: case-fold
/// then ordinal-compare the `ToString()` forms.
pub(crate) fn ci(v: &EV) -> String {
    ei_to_string(v).to_lowercase()
}

pub(crate) fn ord_bool(op: &BinOp, i: i32) -> bool {
    match op {
        BinOp::Eq => i == 0,
        BinOp::Ne => i != 0,
        BinOp::Lt => i < 0,
        BinOp::Le => i <= 0,
        BinOp::Gt => i > 0,
        BinOp::Ge => i >= 0,
        _ => false,
    }
}

/// `double` compare with .NET's NaN rule (NaN sorts below everything).
pub(crate) fn f64_ord(a: f64, b: f64) -> i32 {
    match a.partial_cmp(&b) {
        Some(o) => o as i32,
        None => match (a.is_nan(), b.is_nan()) {
            (true, true) => 0,
            (true, false) => -1,
            _ => 1,
        },
    }
}

/// The `GetType().Equals` test: our Date/DateTime are the same CLR
/// `DateTime`; `Time` is `TimeSpan` and only pairs with itself.
pub(crate) fn same_family(l: &EV, r: &EV) -> bool {
    use EV::{Bool, Date, DateTime, Text, Time};
    matches!(
        (l, r),
        (Bool(_), Bool(_))
            | (Text(_), Text(_))
            | (Time(_), Time(_))
            | (Date(_), Date(_))
            | (Date(_), DateTime(_))
            | (DateTime(_), Date(_))
            | (DateTime(_), DateTime(_))
    )
}

/// Same-type compare: text is `Compare(Trim, Trim)` (case-sensitive
/// ordinal approximation); dates compare as instants; `Time` as a
/// duration; bools as `false < true` (IComparable).
pub(crate) fn same_ordering(l: &EV, r: &EV) -> i32 {
    match (l, r) {
        (EV::Text(a), EV::Text(b)) => a.trim().cmp(b.trim()) as i32,
        (EV::Bool(a), EV::Bool(b)) => a.cmp(b) as i32,
        (EV::Time(a), EV::Time(b)) => a.cmp(b) as i32,
        _ => instant(l).cmp(&instant(r)) as i32,
    }
}

/// Instant projection for the `DateTime` family.
pub(crate) fn instant(v: &EV) -> i64 {
    match v {
        EV::Date(d) => i64::from(*d) * 86_400,
        EV::DateTime(s) => *s,
        EV::Time(t) => i64::from(*t),
        _ => 0,
    }
}

pub(crate) fn parse_f64(s: &str) -> Option<f64> {
    let t = s.trim();
    if t.is_empty() {
        return None;
    }
    t.strip_prefix('+').unwrap_or(t).parse::<f64>().ok()
}

/// `Rule_ConcatExp`: `ToString()` both sides; when one side is `null`
/// only a *string* other side survives.
pub(crate) fn concat(l: &EV, r: &EV) -> EV {
    match (matches!(l, EV::Missing), matches!(r, EV::Missing)) {
        (false, false) => EV::Text(format!("{}{}", ei_to_string(l), ei_to_string(r))),
        (false, true) => {
            if let EV::Text(_) = l {
                l.clone()
            } else {
                EV::Missing
            }
        }
        (true, false) => {
            if let EV::Text(_) = r {
                r.clone()
            } else {
                EV::Missing
            }
        }
        (true, true) => EV::Missing,
    }
}

/// `Rule_AddExp` `+` — `Double + Double`, `DateTime + Double` (AddDays);
/// every other pair is `null`. `Time` is a `TimeSpan` and never adds.
pub(crate) fn add(l: &EV, r: &EV) -> EV {
    if let (Some(a), Some(b)) = (as_f64(l), as_f64(r)) {
        return num_add(l, a, r, b, 1.0);
    }
    if datetime(l) {
        if let Some(n) = as_f64(r) {
            return add_days(l, n);
        }
    }
    EV::Missing
}

/// `Rule_AddExp` `−` — `Double − Double`, `DateTime − Double`,
/// `DateTime − DateTime` → `TimeSpan.TotalDays`.
pub(crate) fn sub(l: &EV, r: &EV) -> EV {
    if let (Some(a), Some(b)) = (as_f64(l), as_f64(r)) {
        return num_add(l, a, r, b, -1.0);
    }
    if datetime(l) {
        if let Some(n) = as_f64(r) {
            return add_days(l, -n);
        }
        if datetime(r) {
            return EV::Real((instant(l) - instant(r)) as f64 / 86_400.0);
        }
    }
    EV::Missing
}

pub(crate) fn num_add(l: &EV, a: f64, r: &EV, b: f64, sign: f64) -> EV {
    if let (EV::Int(x), EV::Int(y)) = (l, r) {
        let v = if sign > 0.0 {
            x.checked_add(*y)
        } else {
            x.checked_sub(*y)
        };
        if let Some(i) = v {
            return EV::Int(i);
        }
    }
    let v = a + sign * b;
    if v.is_finite() {
        EV::Real(v)
    } else {
        EV::Missing
    }
}

/// `DateTime.AddDays` — whole-day steps keep the operand's kind;
/// fractional steps land on `DateTime`.
pub(crate) fn add_days(v: &EV, days: f64) -> EV {
    let secs = instant(v);
    let delta = days * 86_400.0;
    if !delta.is_finite() || delta.abs() > 9e18 {
        return EV::Missing;
    }
    let Some(out) = secs.checked_add(delta.round() as i64) else {
        return EV::Missing;
    };
    match v {
        EV::Date(d) if days.fract() == 0.0 => {
            d.checked_add(days as i32).map_or(EV::Missing, EV::Date)
        }
        _ => EV::DateTime(out),
    }
}

/// `is DateTime` — the date/time family that adds and subtracts.
pub(crate) fn datetime(v: &EV) -> bool {
    matches!(v, EV::Date(_) | EV::DateTime(_))
}

/// The `MultExp` operand domain: `Double`, or `TimeSpan` measured in
/// `TotalDays`.
pub(crate) fn mult_operand(v: &EV) -> Option<f64> {
    match v {
        EV::Time(t) => Some(*t as f64 / 86_400.0),
        _ => as_f64(v),
    }
}

/// `Rule_MultExp` `*`.
pub(crate) fn mul(l: &EV, r: &EV) -> EV {
    match (mult_operand(l), mult_operand(r)) {
        (Some(a), Some(b)) => {
            if let (EV::Int(x), EV::Int(y)) = (l, r) {
                if let Some(i) = x.checked_mul(*y) {
                    return EV::Int(i);
                }
            }
            let v = a * b;
            if v.is_finite() {
                EV::Real(v)
            } else {
                EV::Missing
            }
        }
        _ => EV::Missing,
    }
}

/// `Rule_MultExp` `/`.
pub(crate) fn div(l: &EV, r: &EV) -> EV {
    match (mult_operand(l), mult_operand(r)) {
        (Some(a), Some(b)) => {
            let v = a / b;
            if v.is_finite() {
                EV::Real(v)
            } else {
                EV::Missing
            }
        }
        _ => EV::Missing,
    }
}

/// `Rule_MultExp` `MOD`/`%` — a `double` remainder.
pub(crate) fn modulo(l: &EV, r: &EV) -> EV {
    match (mult_operand(l), mult_operand(r)) {
        (Some(a), Some(b)) => {
            let v = a % b;
            if v.is_finite() {
                EV::Real(v)
            } else {
                EV::Missing
            }
        }
        _ => EV::Missing,
    }
}

/// `Rule_PowExp`: `double.Parse` on both `ToString()`s — text parses
/// here (`"2"^"3"` is 8), unparseable operands are missing.
pub(crate) fn pow(l: &EV, r: &EV) -> EV {
    match (parse_f64(&ei_to_string(l)), parse_f64(&ei_to_string(r))) {
        (Some(a), Some(b)) => {
            let v = libm::pow(a, b);
            if v.is_finite() {
                EV::Real(v)
            } else {
                EV::Missing
            }
        }
        _ => EV::Missing,
    }
}

/// `*`-wildcard match, case-insensitive — the approximation of EI7's
/// `^pattern*-to-.*$` regex documented in decisions.md.
pub(crate) fn like_match(text: &str, pattern: &str) -> bool {
    let t: Vec<char> = text.to_lowercase().chars().collect();
    let p: Vec<char> = pattern.to_lowercase().chars().collect();
    glob(&t, &p)
}

pub(crate) fn glob(t: &[char], p: &[char]) -> bool {
    if p.is_empty() {
        return t.is_empty();
    }
    if p[0] == '*' {
        return (0..=t.len()).any(|k| glob(&t[k..], &p[1..]));
    }
    !t.is_empty() && t[0] == p[0] && glob(&t[1..], &p[1..])
}
