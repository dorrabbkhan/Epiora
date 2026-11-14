//! Rank statistics for MEANS (v1.0 spec §4.2): midranks and the
//! Kruskal–Wallis H test with R's `kruskal.test` tie correction —
//! labelled "Mann–Whitney/Wilcoxon" by Epi Info for two groups.

use crate::special::chisq_sf;
use crate::stats::summation::neumaier_sum;
use crate::stats::{Estimate, UndefinedReason};
use serde::Serialize;

/// Kruskal–Wallis one-way rank test (spec §4.2).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS), ts(export))]
pub struct KruskalWallis {
    /// The tie-corrected H statistic.
    pub h: Estimate,
    /// Groups minus one.
    pub df: Estimate,
    /// `pchisq(H, df, lower.tail = FALSE)`.
    pub p_value: Estimate,
}

/// R's `rank(x)` for MEANS: 1-based average ranks in data order; tied
/// values share their midrank.
pub(crate) fn midranks(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut ranks = vec![0.0; n];
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n && values[order[j]] == values[order[i]] {
            j += 1;
        }
        let mid = (i + 1 + j) as f64 / 2.0;
        for &o in &order[i..j] {
            ranks[o] = mid;
        }
        i = j;
    }
    ranks
}

/// `kruskal.test(x, g)`: H = `12·Σ(Rᵢ²/nᵢ) / (N(N+1)) − 3(N+1)`,
/// divided by the tie correction `1 − Σ(t³−t)/(N³−N)`. An all-tied
/// input makes the correction zero and H `Undefined { zero_variance }`.
pub(crate) fn kruskal_wallis(groups: &[&[f64]]) -> KruskalWallis {
    let n_tot: usize = groups.iter().map(|g| g.len()).sum();
    let mut all: Vec<f64> = Vec::with_capacity(n_tot);
    for g in groups {
        all.extend_from_slice(g);
    }
    let ranks = midranks(&all);
    let n_f = n_tot as f64;
    let df = groups.len() as f64 - 1.0;

    let mut at = 0;
    let mut t_sum = Vec::with_capacity(groups.len());
    for g in groups {
        let r = neumaier_sum(ranks[at..at + g.len()].iter().copied());
        t_sum.push(r * r / g.len() as f64);
        at += g.len();
    }
    let stat = neumaier_sum(t_sum.into_iter());
    let h_raw = 12.0 * stat / (n_f * (n_f + 1.0)) - 3.0 * (n_f + 1.0);

    // R's tie correction: `1 − Σ(t³−t)/(N³−N)` over distinct counts.
    let mut sorted = all.clone();
    sorted.sort_by(f64::total_cmp);
    let mut tie_sum = 0.0;
    let mut i = 0;
    while i < n_tot {
        let mut j = i + 1;
        while j < n_tot && sorted[j] == sorted[i] {
            j += 1;
        }
        let t = (j - i) as f64;
        tie_sum += t * t * t - t;
        i = j;
    }
    let correction = 1.0 - tie_sum / (n_f * n_f * n_f - n_f);

    let h = Estimate::from_f64(h_raw / correction, UndefinedReason::ZeroVariance);
    let p = match h {
        Estimate::Value(v) => Estimate::from_f64(chisq_sf(v, df), UndefinedReason::ZeroVariance),
        Estimate::Undefined { reason } => Estimate::Undefined { reason },
    };
    KruskalWallis {
        h,
        df: Estimate::Value(df),
        p_value: p,
    }
}
