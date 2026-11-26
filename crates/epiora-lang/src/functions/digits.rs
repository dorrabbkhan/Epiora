//! Decimal-string digit arithmetic for `FORMAT`'s numeric pictures.
//! Keeping the arithmetic in digit vectors means neither binary floating
//! point nor a platform rounding mode decides a displayed tie.

/// .NET Framework 4.x custom numeric formatting of a `double` starts
/// from a 15-significant-digit decimal conversion, then applies the
/// picture: the Framework entry point is `System.Number.FormatDouble`
/// (Reference Source, `mscorlib/system/number.cs`), whose CLR
/// implementation uses `DOUBLE_PRECISION` 15 in `DoubleToNumber` before
/// `NumberToStringFormat` (Shared Source CLI 2.0,
/// `clr/src/vm/comnumber.cpp`; .NET Core's `DoublePrecisionCustomFormat`
/// documents the same previous-release compatibility). This port performs
/// that pre-rounding on the decimal digit string, half away from zero, so
/// no f64 arithmetic or platform float formatting decides a tie.
pub(crate) const DOTNET_CUSTOM_DOUBLE_DIGITS: usize = 15;

/// Round positive fixed-point decimal text to `significant` significant
/// digits, half away from zero, and return fixed-point text. The digit
/// string is normalised through a scientific significand/exponent pair so
/// integer zeroes keep their place value without any float arithmetic.
pub(crate) fn round_significant_digits(text: &str, significant: usize) -> Option<String> {
    let (int_text, frac_text) = match text.split_once('.') {
        Some((i, f)) => (i, f),
        None => (text, ""),
    };
    let integer = decimal_digits(int_text)?;
    let fraction = decimal_digits(frac_text)?;
    let mut digits = Vec::with_capacity(integer.len() + fraction.len());
    digits.extend_from_slice(&integer);
    digits.extend_from_slice(&fraction);
    let Some(first) = digits.iter().position(|d| *d != 0) else {
        return Some("0".to_string());
    };
    let last = digits.iter().rposition(|d| *d != 0)? + 1;
    let mut significand = digits[first..last].to_vec();
    let mut exponent = integer.len() as isize - first as isize - 1;
    if significand.len() > significant {
        let round_digit = significand[significant];
        significand.truncate(significant);
        if round_digit >= 5 {
            increment_digits(&mut significand);
            if significand.len() > significant {
                significand.clear();
                significand.push(1);
                exponent += 1;
            }
        }
    }
    while significand.len() > 1 && significand.last() == Some(&0) {
        significand.pop();
    }
    Some(fixed_point_text(&significand, exponent))
}

/// Render `digits[0].digits[1..] × 10^exponent` without an exponent
/// marker. `digits` is normalised (its first digit is nonzero).
fn fixed_point_text(digits: &[u8], exponent: isize) -> String {
    let digits: String = digits.iter().map(|d| char::from(b'0' + *d)).collect();
    if exponent >= 0 {
        let integer_len = exponent as usize + 1;
        if digits.len() <= integer_len {
            let mut out = digits;
            out.push_str(&"0".repeat(integer_len - out.len()));
            out
        } else {
            format!("{}.{}", &digits[..integer_len], &digits[integer_len..])
        }
    } else {
        format!("0.{}{digits}", "0".repeat((-exponent - 1) as usize))
    }
}

/// ASCII decimal digits as values 0–9; anything else rejects the input.
pub(crate) fn decimal_digits(s: &str) -> Option<Vec<u8>> {
    s.bytes()
        .map(|b| match b {
            b'0'..=b'9' => Some(b - b'0'),
            _ => None,
        })
        .collect()
}

pub(crate) fn increment_digits(digits: &mut Vec<u8>) {
    for d in digits.iter_mut().rev() {
        if *d < 9 {
            *d += 1;
            return;
        }
        *d = 0;
    }
    digits.insert(0, 1);
}
