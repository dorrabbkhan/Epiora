//! Discrete-distribution helpers (v1.0 spec §4.1): `poisson_tail`
//! through the regularised incomplete gamma. `binomial_quantile`
//! lives in `qbinom.rs` — R's `qbinom.c` + `qDiscrete_search.h` port.

use super::gamma_lower::{inc_gamma_lower, inc_gamma_upper};

/// `ppois(k, lambda, lower.tail)` for `upper = false` (`P(X ≤ k)`),
/// `ppois(k − 1, lambda, lower.tail = FALSE)` for `upper = true`
/// (`P(X ≥ k)`), via `pgamma(λ, k)`'s two tails as R's `ppois` defines
/// them: `P(X ≤ k) = Q(k+1, λ)` and `P(X ≥ k) = P(k, λ)`.
/// Negative or NaN `lambda` gives NaN.
pub fn poisson_tail(k: u64, lambda: f64, upper: bool) -> f64 {
    if lambda.is_nan() || lambda < 0.0 {
        return f64::NAN;
    }
    if upper {
        if k == 0 {
            return 1.0;
        }
        inc_gamma_lower(k as f64, lambda)
    } else {
        // k = u64::MAX saturates at the same f64 k + 1 rounds to —
        // P(X ≤ k) = Q(k + 1, λ) → 1, the mathematically correct tail.
        inc_gamma_upper(k.saturating_add(1) as f64, lambda)
    }
}
