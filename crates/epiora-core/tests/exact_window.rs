//! Task 11a: `NcHypergeom::weights` skips `libm::exp` for terms whose
//! `v − max` is below the underflow-to-zero point, where `exp` returns
//! exactly `0.0`. Task 11b: the whole pass is windowed — only the mass
//! window is stored, summed and divided. Every `pmf`, `mean`,
//! `lower_tail` and `upper_tail` output must stay bit-identical to the
//! pre-change full-support algorithm (`weights_probe`'s reference),
//! and the stored count must drop to the mass window.

use epiora_core::special::NcHypergeom;

/// The psi grid of the task: extreme odds ratios in both directions,
/// around 1 on both sides, and exactly 1.
const PSIS: [f64; 9] = [1e-300, 1e-12, 0.5, 0.999, 1.0, 1.001, 2.0, 1e12, 1e300];

/// The reference derivations of the public methods on the full-support
/// `weights` — the same sums `pmf`/`mean`/`tail_sum` make, computed on
/// the pre-change weights `weights_probe` returns.
struct Reference {
    w: Vec<f64>,
    lo: u64,
    hi: u64,
}

impl Reference {
    fn at(h: &NcHypergeom, psi: f64) -> Reference {
        let (w, ..) = h.weights_probe(psi);
        let (lo, hi) = h.support();
        Reference { w, lo, hi }
    }

    fn pmf(&self, x: u64) -> f64 {
        if x < self.lo || x > self.hi {
            return 0.0;
        }
        self.w[(x - self.lo) as usize]
    }

    fn mean(&self) -> f64 {
        self.w
            .iter()
            .enumerate()
            .map(|(i, &w)| (self.lo + i as u64) as f64 * w)
            .sum()
    }

    fn lower_tail(&self, x: u64) -> f64 {
        self.w
            .iter()
            .enumerate()
            .filter(|(i, _)| self.lo + *i as u64 <= x)
            .map(|(_, &w)| w)
            .sum()
    }

    fn upper_tail(&self, x: u64) -> f64 {
        self.w
            .iter()
            .enumerate()
            .filter(|(i, _)| self.lo + *i as u64 >= x)
            .map(|(_, &w)| w)
            .sum()
    }
}

#[test]
fn windowed_outputs_are_bit_identical_to_full_support() {
    // Supports of 10, 1,000 and ~490,000 values (`lo` is 0 for all of
    // these, so `hi` + 1 is the support size).
    for (m, n, k) in [
        (10, 10, 9),
        (1_000, 1_000, 999),
        (490_000, 490_000, 490_000),
    ] {
        let h = NcHypergeom::new(m, n, k);
        let (lo, hi) = h.support();
        let xs = [lo, lo + 1, (lo + hi) / 2, hi - 1, hi];
        for psi in PSIS {
            let r = Reference::at(&h, psi);
            let ctx = format!("m={m} n={n} k={k} psi={psi:e}");
            for &x in &xs {
                assert_eq!(
                    h.pmf(x, psi).to_bits(),
                    r.pmf(x).to_bits(),
                    "pmf x={x} {ctx}"
                );
                assert_eq!(
                    h.lower_tail(x, psi).to_bits(),
                    r.lower_tail(x).to_bits(),
                    "lower_tail x={x} {ctx}"
                );
                assert_eq!(
                    h.upper_tail(x, psi).to_bits(),
                    r.upper_tail(x).to_bits(),
                    "upper_tail x={x} {ctx}"
                );
            }
            assert_eq!(h.mean(psi).to_bits(), r.mean().to_bits(), "mean {ctx}");
        }
    }
}

#[test]
fn exp_terms_are_window_sized() {
    // The ~490,000-value support of the 1M-row workload at psi = 1:
    // the probability mass sits far inside the support, so the number
    // of `libm::exp` evaluations per `weights` call must be well under
    // the support size.
    let big = NcHypergeom::new(490_000, 490_000, 490_000);
    let evals = big.weights_probe(1.0).1;
    assert!(
        evals < 20_000,
        "exp evaluations {evals} must stay below 20,000"
    );
    // On a 10-value support every term is inside the window: the count
    // equals the support size.
    let small = NcHypergeom::new(10, 10, 9);
    let (lo, hi) = small.support();
    assert_eq!(small.weights_probe(1.0).1, (hi - lo + 1) as usize);
}

#[test]
fn window_is_window_sized() {
    // Task 11b: `weights` stores only the mass window. On the
    // 490,001-value support of the 1M-row workload at psi = 1 the
    // stored count must be well under the support size; on the
    // 10-value support the window is the whole support.
    let big = NcHypergeom::new(490_000, 490_000, 490_000);
    let stored = big.weights_probe(1.0).2;
    assert!(
        stored < 20_000,
        "stored weights {stored} must stay below 20,000"
    );
    let small = NcHypergeom::new(10, 10, 9);
    let (lo, hi) = small.support();
    assert_eq!(small.weights_probe(1.0).2, (hi - lo + 1) as usize);
}

#[test]
fn tails_outside_the_window_keep_their_bits() {
    // The 490,001-value support: probe `pmf` and both tails at every
    // interesting x — outside the support, inside it but outside the
    // mass window (non-empty all-zero ranges give +0.0; an empty
    // range gives −0.0), and on the window edges. The full-support
    // reference is the oracle.
    let h = NcHypergeom::new(490_000, 490_000, 490_000);
    let (lo, hi) = h.support();
    for psi in PSIS {
        let r = Reference::at(&h, psi);
        let first =
            r.w.iter()
                .position(|&w| w != 0.0)
                .expect("a window always exists");
        let last =
            r.w.iter()
                .rposition(|&w| w != 0.0)
                .expect("a window always exists");
        let (wlo, whi) = (lo + first as u64, lo + last as u64);
        let mut xs = vec![lo, wlo, wlo + 1, whi, whi + 1, hi, hi + 1];
        if lo > 0 {
            xs.push(lo - 1);
        }
        if wlo > 0 {
            xs.push(wlo - 1);
        }
        if whi > 0 {
            xs.push(whi - 1);
        }
        xs.sort_unstable();
        xs.dedup();
        let ctx = format!("lo={lo} hi={hi} window=[{wlo},{whi}] psi={psi:e}");
        for &x in &xs {
            assert_eq!(
                h.pmf(x, psi).to_bits(),
                r.pmf(x).to_bits(),
                "pmf x={x} {ctx}"
            );
            assert_eq!(
                h.lower_tail(x, psi).to_bits(),
                r.lower_tail(x).to_bits(),
                "lower_tail x={x} {ctx}"
            );
            assert_eq!(
                h.upper_tail(x, psi).to_bits(),
                r.upper_tail(x).to_bits(),
                "upper_tail x={x} {ctx}"
            );
        }
    }
}
