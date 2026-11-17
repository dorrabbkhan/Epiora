//! `two_by_two::parts` — the closed-form intermediates of one 2×2
//! table (v1.0 Task 10, spec §6), split from `two_by_two.rs` so the
//! module stays under the ~400-line limit. The run path calls
//! `parts` and the explain path reuses it, so a trace's `values` and
//! the result come from the same computation.

use super::two_by_two::Table2x2;
use crate::special::normal_quantile;

/// `qnorm(0.975)`; `normal_quantile` is `Some` for every p in (0, 1),
/// the fallback is the same correctly-rounded constant.
fn z_975() -> f64 {
    normal_quantile(0.975).unwrap_or(1.959963984540054)
}

/// The closed-form intermediates `two_by_two` computes — the explain
/// trace's named values (v1.0 Task 10, spec §6).
pub(crate) struct Parts2x2 {
    /// Exposed row total `a + b`.
    pub n1: f64,
    /// Unexposed row total `c + d`.
    pub n0: f64,
    /// Ill column total `a + c`.
    pub m1: f64,
    /// Well column total `b + d`.
    pub m0: f64,
    /// Grand total.
    pub n: f64,
    /// `qnorm(0.975)`.
    pub z: f64,
    /// `a / n1`.
    pub pe: f64,
    /// `c / n0`.
    pub pu: f64,
    /// `pe - pu`.
    pub rd: f64,
    /// Wald SE of `rd`.
    pub se_rd: f64,
    /// `pe / pu`.
    pub rr: f64,
    /// Log-scale SE of `rr`.
    pub se_rr: f64,
    /// `pe / ((c+1)/(n0+1))` (epitools small-sample).
    pub rr_small: f64,
    /// `a*d / (b*c)`.
    pub or: f64,
    /// Log-scale SE of `or`.
    pub se_or: f64,
    /// Expected counts `[ea, eb, ec, ed]` under independence.
    pub e: [f64; 4],
    /// `|a - n1*m1/n|`.
    pub delta: f64,
    /// Yates correction `0.5.min(delta)`.
    pub yates_adj: f64,
    /// Pearson chi-square (NaN when an expected count is 0).
    pub chi_uncorrected: f64,
    /// Yates chi-square.
    pub chi_yates: f64,
    /// Mantel–Haenszel (N−1) chi-square.
    pub chi_mh: f64,
}

/// The intermediates of `two_by_two(t)` — shared by the result and the
/// explain trace (spec §6).
pub(crate) fn parts(t: &Table2x2) -> Parts2x2 {
    let (a, b, c, d) = (t.a as f64, t.b as f64, t.c as f64, t.d as f64);
    let n1 = a + b;
    let n0 = c + d;
    let n = n1 + n0;
    let m1 = a + c;
    let m0 = b + d;
    let z = z_975();

    let pe = a / n1;
    let pu = c / n0;
    let rd = pe - pu;
    let se_rd = libm::sqrt(pe * (1.0 - pe) / n1 + pu * (1.0 - pu) / n0);
    let rr = pe / pu;
    let se_rr = libm::sqrt(1.0 / a - 1.0 / n1 + 1.0 / c - 1.0 / n0);
    let rr_small = pe / ((c + 1.0) / (n0 + 1.0));
    let or = a * d / (b * c);
    let se_or = libm::sqrt(1.0 / a + 1.0 / b + 1.0 / c + 1.0 / d);
    let delta = (a - n1 * m1 / n).abs();
    let e = [n1 * m1 / n, n1 * m0 / n, n0 * m1 / n, n0 * m0 / n];
    let o = [a, b, c, d];
    let (chi_uncorrected, chi_yates, chi_mh) = if e.iter().all(|&ei| ei > 0.0) {
        let uncorrected: f64 = o
            .iter()
            .zip(&e)
            .map(|(oi, ei)| (oi - ei) * (oi - ei) / ei)
            .sum();
        let yates: f64 = o
            .iter()
            .zip(&e)
            .map(|(oi, ei)| {
                ((oi - ei).abs() - yates_adj_of(delta)) * ((oi - ei).abs() - yates_adj_of(delta))
                    / ei
            })
            .sum();
        let mh = delta * delta * n * n * (n - 1.0) / (n1 * n0 * m1 * m0);
        (uncorrected, yates, mh)
    } else {
        (f64::NAN, f64::NAN, f64::NAN)
    };
    Parts2x2 {
        n1,
        n0,
        m1,
        m0,
        n,
        z,
        pe,
        pu,
        rd,
        se_rd,
        rr,
        se_rr,
        rr_small,
        or,
        se_or,
        e,
        delta,
        yates_adj: yates_adj_of(delta),
        chi_uncorrected,
        chi_yates,
        chi_mh,
    }
}

fn yates_adj_of(delta: f64) -> f64 {
    0.5_f64.min(delta)
}
