//! Compensated summation for `stats` (v1.0 spec §4.2): a Neumaier sum
//! over the rows in data order keeps totals deterministic and stable
//! at 1M rows — `sum_is_order_stable_at_1m` pins the order-invariance.

/// Sums `values` with Neumaier compensation: `sum + t` where `t`
/// accumulates the low-order error of each addition. Inputs are finite
/// (`Dataset` rejects non-finite reals); the result is finite for any
/// input the engine accepts.
pub fn neumaier_sum(values: impl Iterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut t = 0.0;
    for x in values {
        let s = sum + x;
        t += if sum.abs() >= x.abs() {
            (sum - s) + x
        } else {
            (x - s) + sum
        };
        sum = s;
    }
    sum + t
}
