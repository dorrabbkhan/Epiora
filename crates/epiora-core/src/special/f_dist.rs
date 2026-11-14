//! The F survival function (v1.0 spec §4.1): `P(F > x) =
//! I_z(df2/2, df1/2)` with `z = df2/(df2 + df1·x)`. The left tail
//! means `z → 0`, where `reg_inc_beta` takes its direct branch, so
//! deep tails stay accurate — but `z` also rounds to 1 when
//! `df1·x < df2` is huge, so R's `pf` picks whichever argument sits
//! below the mean (`z` when `df1·x > df2`, else `w = 1−z` with the
//! upper tail taken directly) and never forms `1 − (≈1)`.

use super::beta::{reg_inc_beta, reg_inc_beta_upper};

/// `pf(x, df1, df2, lower.tail = FALSE)`. Non-positive or infinite
/// `df1`/`df2`, or NaN `x`, give NaN; `x ≤ 0` gives 1.
pub fn f_sf(x: f64, df1: f64, df2: f64) -> f64 {
    if x.is_nan() || !(df1 > 0.0 && df1.is_finite()) || !(df2 > 0.0 && df2.is_finite()) {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 1.0;
    }
    // R's pf: `if (df1*x > df2) pbeta(z, df2/2, df1/2, lower.tail)`
    // else `pbeta(df1*x/(df2 + df1*x), df1/2, df2/2, !lower.tail)` —
    // the computed argument is always the one that does not squeeze
    // against 1.0 at huge df.
    if df1 * x > df2 {
        reg_inc_beta(df2 / (df2 + df1 * x), df2 * 0.5, df1 * 0.5)
    } else {
        reg_inc_beta_upper(df1 * x / (df2 + df1 * x), df1 * 0.5, df2 * 0.5)
    }
}
