//! `pchisq(x, df, lower.tail = FALSE)` (spec §3.4) through the
//! regularised upper incomplete gamma Q(df/2, x/2), and `pchisq(x, df)`
//! as R's `pgamma` computes the lower tail, so `1 - chisq_cdf`
//! reproduces `1 - pchisq` — the form `DescTools::BreslowDayTest` and
//! R code returning tail probabilities by complement use.

use super::gamma::inc_gamma_upper;
use super::gamma_lower::inc_gamma_lower;

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

/// Chi-squared CDF: `pchisq(x, df)` — R `pgamma`'s lower-tail branches
/// (`pgamma_raw` port), not merely `1 - chisq_sf`. `df = 0` is the
/// point mass at 0, as in R.
pub fn chisq_cdf(x: f64, df: f64) -> f64 {
    if x.is_nan() || df.is_nan() || df < 0.0 {
        return f64::NAN;
    }
    if df == 0.0 {
        return if x <= 0.0 { 0.0 } else { 1.0 };
    }
    if x <= 0.0 {
        return 0.0;
    }
    inc_gamma_lower(df * 0.5, x * 0.5)
}
