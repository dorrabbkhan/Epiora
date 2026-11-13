//! Discrete-distribution helpers (v1.0 spec §4.1): `poisson_tail`
//! through the regularised incomplete gamma, `binomial_quantile` by
//! binary search on `pbinom`'s incomplete-beta form.

use super::beta::reg_inc_beta;
use super::gamma::{inc_gamma_lower, inc_gamma_upper};

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

/// `pbinom(k, n, prob)` — `P(X ≤ k)` as the `pbeta` identity R uses,
/// `I_{1−prob}(n−k, k+1)`. O(1) where `binomial_tail`'s literal sum is
/// O(n), which kept `binomial_quantile` at O(n·log n).
fn binomial_cdf(k: u64, n: u64, prob: f64) -> f64 {
    if k >= n {
        return 1.0;
    }
    reg_inc_beta(1.0 - prob, (n - k) as f64, k as f64 + 1.0)
}

/// `qbinom(p, n, prob)`: the smallest `q` in `0..=n` with
/// `P(X ≤ q) ≥ p`. As in R, `p ≤ 0` gives 0 and `p ≥ 1` gives `n`
/// before the degenerate `prob` cases are resolved; `prob ≤ 0` (mass
/// at 0) then gives 0 and `prob ≥ 1` (mass at n) gives `n`.
pub fn binomial_quantile(p: f64, n: u64, prob: f64) -> u64 {
    if p <= 0.0 || p.is_nan() {
        return 0;
    }
    if p >= 1.0 {
        return n;
    }
    if prob <= 0.0 || prob.is_nan() {
        return 0;
    }
    if prob >= 1.0 {
        return n;
    }
    let (mut lo, mut hi) = (0u64, n);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if binomial_cdf(mid, n, prob) >= p {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo
}
