//! `pchisq(x, df, lower.tail = FALSE)` (spec §3.4) through the
//! regularised upper incomplete gamma Q(df/2, x/2).

use super::gamma::inc_gamma_upper;

/// Chi-squared survival function: `pchisq(x, df, lower.tail=FALSE)`.
pub fn chisq_sf(x: f64, df: f64) -> f64 {
    if x.is_nan() || df.is_nan() {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 1.0;
    }
    inc_gamma_upper(df * 0.5, x * 0.5)
}
