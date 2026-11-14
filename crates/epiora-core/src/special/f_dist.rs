//! The F survival function (v1.0 spec §4.1): `P(F > x) =
//! I_z(df2/2, df1/2)` with `z = df2/(df2 + df1·x)`. The left tail
//! means `z → 0`, where `reg_inc_beta` takes its direct branch, so
//! deep tails stay accurate — but `z` also rounds to 1 when
//! `df1·x < df2` is huge, so R's `pf` picks whichever argument sits
//! below the mean (`z` when `df1·x > df2`, else `w = 1−z` with the
//! upper tail taken directly) and never forms `1 − (≈1)`. Infinite
//! df goes to `pchisq` before the beta path (pf.c L44-56): `df2 =
//! +Inf` gives `pchisq(x·df1, df1, lo=F)`, `df1 = +Inf` gives
//! `pchisq(df2/x, df2, lo=T)`, and both infinite is the `x < 1` /
//! `x = 1` / `x > 1` piecewise.

use super::beta::{reg_inc_beta, reg_inc_beta_upper};
use super::chisq::{chisq_cdf, chisq_sf};

/// `pf(x, df1, df2, lower.tail = FALSE)`. NaN inputs or a
/// non-positive/`−Inf` `df1`/`df2` give NaN; `x ≤ 0` gives 1,
/// `x = +Inf` gives 0.
pub fn f_sf(x: f64, df1: f64, df2: f64) -> f64 {
    if x.is_nan() || df1.is_nan() || df2.is_nan() {
        return f64::NAN;
    }
    if df1 <= 0.0 || df2 <= 0.0 {
        return f64::NAN; // pf.c L34: covers -Inf
    }
    // R_P_bounds_01(x, 0., +Inf) — before the df checks, as in pf.c.
    if x <= 0.0 {
        return 1.0;
    }
    if x.is_infinite() {
        return 0.0;
    }
    // pf.c L44-56: infinite df delegates to pchisq — only needed at
    // df = +Inf "since pbeta(0,*) = 0" per R's comment.
    if df2.is_infinite() {
        if df1.is_infinite() {
            return if x < 1.0 {
                1.0 // R_DT_0 for lower.tail = FALSE
            } else if x == 1.0 {
                0.5
            } else {
                0.0 // R_DT_1
            };
        }
        return chisq_sf(x * df1, df1);
    }
    if df1.is_infinite() {
        return chisq_cdf(df2 / x, df2);
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
