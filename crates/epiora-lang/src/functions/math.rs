//! Numeric functions: the `ToString()` + `TryParse` scalar family,
//! `ROUND`/`TRUNC`/`STEP`/`RND`, the Poisson bisection limits (ported
//! verbatim, `pi = 3.1416` and all), and the `TXTTONUM` overwrite
//! quirk.

use crate::expr::eval::Ctx;
use crate::expr::missing::{coerce_f64, ei_to_string, is_empty, EV};

/// `ABS`, `COS`, `EXP`, `LN`, `LOG`, `SIN`, `SQRT`, `TAN`, `TRUNC`:
/// `TryParse` the `ToString()` then apply — missing when unparseable
/// or non-finite (EI7 stores the `double`; `NaN`/`∞` cannot enter a
/// dataset, so they read as missing here).
pub(crate) fn scalar(v: &EV, f: impl Fn(f64) -> f64) -> EV {
    match coerce_f64(v) {
        Some(x) => {
            let y = f(x);
            if y.is_finite() {
                EV::Real(y)
            } else {
                EV::Missing
            }
        }
        None => EV::Missing,
    }
}

/// `ROUND(x[, digits])` — `Math.Round(x, d, AwayFromZero)`; a
/// non-integer or absent digits argument is 0 (`Int32.TryParse`'s
/// default), digits outside `0..=15` throw in EI7 → missing here.
pub(crate) fn round(v: &[EV]) -> EV {
    let digits = if v.len() == 2 {
        match coerce_f64(&v[1]) {
            Some(d) if d.fract() == 0.0 => d as i64,
            _ => 0,
        }
    } else {
        0
    };
    if !(0..=15).contains(&digits) {
        return EV::Missing;
    }
    match coerce_f64(&v[0]) {
        Some(x) => {
            let scale = libm::pow(10.0, digits as f64);
            let y = (x * scale).round() / scale;
            if y.is_finite() {
                EV::Real(y)
            } else {
                EV::Missing
            }
        }
        None => EV::Missing,
    }
}

/// `STEP(x, t)` — `x < t ? 0 : 1`; unparseable is missing.
pub(crate) fn step(x: &EV, t: &EV) -> EV {
    match (coerce_f64(x), coerce_f64(t)) {
        (Some(x), Some(t)) => {
            if x < t {
                EV::Real(0.0)
            } else {
                EV::Real(1.0)
            }
        }
        _ => EV::Missing,
    }
}

/// `RND(max)` / `RND(min, max)` — `random.Next`-shaped but seeded:
/// draws come from the expression-seeded ChaCha20 stream (hard rule
/// 7). Invalid bounds throw in EI7; here they are missing.
pub(crate) fn rnd(v: &[EV], ctx: &mut Ctx) -> EV {
    let (lo, hi) = match v.len() {
        1 => match coerce_int(&v[0]) {
            Some(max) if max > 0 => (0, max),
            _ => return EV::Missing,
        },
        _ => match (coerce_int(&v[0]), coerce_int(&v[1])) {
            (Some(lo), Some(hi)) if lo < hi => (lo, hi),
            _ => return EV::Missing,
        },
    };
    EV::Int(ctx.draw_bounded(lo, (hi - lo) as u64))
}

/// `int.TryParse` — a whole number as text.
fn coerce_int(v: &EV) -> Option<i64> {
    match coerce_f64(v)? {
        x if x.fract() == 0.0 && x.abs() <= f64::from(i32::MAX) => Some(x as i64),
        _ => None,
    }
}

/// `TXTTONUM(v)` — the two-step quirk: a `ToString()` of
/// "true"/"false" (case-insensitive) is overwritten to 1/0 *before*
/// `Double.TryParse` writes the parsed double or **0.0** — so
/// `TXTTONUM("true")` is 1, `TXTTONUM("false")` is 0 and
/// `TXTTONUM("abc")` is 0 too. Only an empty input is missing.
/// (`Rule_TxtToNum.Execute` @ 4cd353c, lines 32–44.)
pub(crate) fn txt_to_num(v: &EV) -> EV {
    if is_empty(v) {
        return EV::Missing;
    }
    let s = ei_to_string(v);
    let s = if s.eq_ignore_ascii_case("true") {
        "1".to_string()
    } else if s.eq_ignore_ascii_case("false") {
        "0".to_string()
    } else {
        s
    };
    match s.trim().parse::<f64>() {
        Ok(x) => EV::Real(x),
        Err(_) => EV::Real(0.0),
    }
}

/// `POISSONLCL(count[, level])` — the bisection lower bound, ported
/// with `pi = 3.1416` and `pVal` tolerance `1e-8` exactly as the
/// interpreter computes them.
pub(crate) fn poisson_lcl(v: &[EV]) -> EV {
    let Some(count) = coerce_f64(&v[0]) else {
        return EV::Missing;
    };
    let level = if v.len() >= 2 {
        coerce_f64(&v[1]).unwrap_or(95.0)
    } else {
        95.0
    };
    let mut low = 0.0f64;
    let mut up = level;
    let limit = level / 2.0;
    let mut p_val = 0.0;
    while limit < up {
        p_val = (low + up) / 2.0;
        if (p_val - count).abs() <= 1e-8 {
            break;
        }
        if poisson_probability_f(count, p_val, true) >= limit {
            low = p_val;
        } else {
            up = p_val;
        }
    }
    if p_val.is_finite() {
        EV::Real(p_val)
    } else {
        EV::Missing
    }
}

/// `POISSONUCL(count[, level])` — the bisection upper bound.
pub(crate) fn poisson_ucl(v: &[EV]) -> EV {
    let Some(count) = coerce_f64(&v[0]) else {
        return EV::Missing;
    };
    let level = if v.len() >= 2 {
        coerce_f64(&v[1]).unwrap_or(95.0)
    } else {
        95.0
    };
    let mut low = 0.0f64;
    let mut up = 3.0 * count;
    let limit = (100.0 - level) / 2.0;
    let mut p_val = 0.0;
    while p_val < count {
        p_val = (low + up) / 2.0;
        if (p_val - count).abs() <= 1e-8 {
            break;
        }
        if poisson_probability_f(count, p_val, true) >= limit {
            up = p_val;
        } else {
            low = p_val;
        }
    }
    if p_val.is_finite() {
        EV::Real(p_val)
    } else {
        EV::Missing
    }
}

/// `EpiFunctions.PoissonProbabilityF`, ported verbatim — including
/// `pi = 3.1416` and the exact `expFactor`/series truncation.
fn poisson_probability_f(count: f64, exact_value: f64, b: bool) -> f64 {
    let mut nn = (count + 1.0).floor() as i64;
    if nn == 0 {
        nn = 1;
    }
    // The literal `pi` value is EI7's own (not `f64::consts::PI`).
    #[allow(clippy::approx_constant)]
    let pi = 3.1416;
    let twopi = 2.0 * pi;
    let mut f = 1.0;
    let exp_factor = -(exact_value - count + count * libm::log(exact_value / count));
    let p = if count <= 200.0 {
        libm::exp(exp_factor)
    } else {
        let mut exp_value = count * libm::log(1.0 + 1.0 / (12.0 * count - (0.1 / count)));
        exp_value += 0.5 * libm::log(twopi * count);
        exp_value = count * libm::log(count) - exp_value;
        let ex2 = count * libm::log(count * libm::exp(-count) / exact_value);
        -ex2 - exp_value
    };
    let mut i = nn;
    while i > 0 {
        f = 1.0 + exact_value / (i as f64 + exact_value * f);
        i -= 1;
    }
    let q = 1.0 / (f * count);
    if b {
        p + q * (1.0 - p)
    } else {
        q * (1.0 - p)
    }
}

/// The en-US `double.ToString()` — "G15": up to 15 significant
/// digits, trailing zeros trimmed, scientific notation outside the
/// `1e-4 .. 1e15` window.
pub(crate) fn dotnet_g15(x: f64) -> String {
    if !x.is_finite() {
        return String::new();
    }
    if x == 0.0 {
        return "0".to_string();
    }
    let (mantissa, exp) = sci_parts(x);
    if (-4..15).contains(&exp) {
        let mut s = fixed_from_digits(&mantissa, exp);
        if x < 0.0 {
            s.insert(0, '-');
        }
        s
    } else {
        format!(
            "{}E{}{:02}",
            trim_mantissa(&mantissa),
            if exp < 0 { "-" } else { "+" },
            exp.abs()
        )
    }
}

/// Digits and base-10 exponent of `x` at 15 significant digits.
fn sci_parts(x: f64) -> (String, i32) {
    let a = x.abs();
    let exp = libm::floor(libm::log10(a)) as i32;
    let scaled = a / libm::pow(10.0, exp as f64);
    let mantissa = format!("{:.14}", scaled);
    // Rounding to 15 digits can carry to "10.000…" — renormalize.
    if mantissa.starts_with("10") {
        (format!("{:.14}", scaled / 10.0), exp + 1)
    } else {
        (mantissa, exp)
    }
}

/// `d.ddddd…` → the `G15` fixed-point string for exponent `exp`.
fn fixed_from_digits(mantissa: &str, exp: i32) -> String {
    let digits: String = mantissa.chars().filter(|c| c.is_ascii_digit()).collect();
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    if exp >= 0 {
        let int_len = (exp as usize) + 1;
        if digits.len() <= int_len {
            let mut s = digits.to_string();
            s.push_str(&"0".repeat(int_len - digits.len()));
            s
        } else {
            format!("{}.{}", &digits[..int_len], &digits[int_len..])
        }
    } else {
        let zeros = "0".repeat((-exp - 1) as usize);
        format!("0.{zeros}{digits}")
    }
}

/// `d.dddd` with trailing zeros trimmed for the scientific form.
fn trim_mantissa(mantissa: &str) -> &str {
    mantissa.trim_end_matches('0').trim_end_matches('.')
}
