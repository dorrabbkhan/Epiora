//! The bit-identity test reference of `tests/exact_window.rs`
//! (Tasks 11a–11b): the pre-change full-support `weights` algorithm
//! and the `weights_probe` that exposes it, kept here so
//! `hypergeom.rs` stays under the ~400-line rule.

use super::NcHypergeom;

impl NcHypergeom {
    /// The full-support `weights` of the pre-Task-11a algorithm,
    /// verbatim — the bit-identity reference `weights_probe` returns.
    fn weights_full(&self, psi: f64) -> Vec<f64> {
        let lpsi = libm::log(psi);
        let mut max = f64::NEG_INFINITY;
        let mut d = Vec::with_capacity(self.logdc.len());
        for (i, &lc) in self.logdc.iter().enumerate() {
            let v = lc + lpsi * (self.lo + i as u64) as f64;
            if v > max {
                max = v;
            }
            d.push(v);
        }
        let mut sum = 0.0;
        for v in &mut d {
            *v = libm::exp(*v - max);
            sum += *v;
        }
        for v in &mut d {
            *v /= sum;
        }
        d
    }

    /// Test-only probe (`tests/exact_window.rs`, Tasks 11a–11b): the
    /// pre-change full-support `weights` — the bit-identity reference —
    /// the number of `libm::exp` evaluations `weights` makes at `psi`,
    /// and the number of weights it stores. `#[doc(hidden)]`, not
    /// public API.
    #[doc(hidden)]
    pub fn weights_probe(&self, psi: f64) -> (Vec<f64>, usize, usize) {
        let w = self.weights(psi);
        (self.weights_full(psi), w.evals, w.d.len())
    }
}
