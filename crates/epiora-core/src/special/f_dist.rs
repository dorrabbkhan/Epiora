//! Fisher's F distribution (v1.0 spec §4.1): `pf(x, df1, df2,
//! lower.tail = FALSE)` through the incomplete beta —
//! `sf = I_z(df2/2, df1/2)` with `z = df2/(df2 + df1·x)`. A deep right
//! tail means `z → 0`, where `reg_inc_beta` takes its direct branch,
//! so the small value stays accurate.

use super::beta::reg_inc_beta;

/// `pf(x, df1, df2, lower.tail = FALSE)`. `x ≤ 0` is 1; NaN inputs or
/// a non-positive/infinite `df` give NaN.
pub fn f_sf(x: f64, df1: f64, df2: f64) -> f64 {
    if x.is_nan() || !(df1 > 0.0 && df1.is_finite()) || !(df2 > 0.0 && df2.is_finite()) {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 1.0;
    }
    reg_inc_beta(df2 / (df2 + df1 * x), df2 * 0.5, df1 * 0.5)
}
