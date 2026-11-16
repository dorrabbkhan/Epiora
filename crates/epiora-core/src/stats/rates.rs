//! Person-time rate statistics (v1.0 Task 8), ported from
//! `fixtures/R/methods/rates.R` and `fixtures/R/methods/poisson.R`
//! (themselves ports of OpenEpi `PersonTime2.js` and epitools): the
//! two-rate comparison set — per-stratum rates, the pooled z-score,
//! the Wald rate ratio, the Taylor/Byar rate difference, etiologic
//! and prevented fractions, the directly/MH-adjusted estimates and
//! the Breslow–Day homogeneity tests. All functions return raw
//! `f64`s; non-finite results become `{"undefined"}` leaves at the
//! `calc` boundary.

use crate::special::{chisq_sf, normal_cdf, normal_quantile};
use crate::stats::summation::neumaier_sum;

/// `qnorm((1 + conf/100)/2)` — the two-sided normal deviate for a
/// confidence level given in percent (`oe_z` in the R methods).
fn z(conf: f64) -> f64 {
    normal_quantile((1.0 + conf / 100.0) / 2.0).unwrap_or(f64::NAN)
}

/// `epitools::pois.approx` rate CI — Taylor on the count scale,
/// `[x ∓ z√x] / pt`, unclipped (the corpus records negative lowers).
/// `[est, lower, upper]` on the rate scale.
pub(crate) fn pois_approx(cases: u64, pt: f64, conf: f64) -> [f64; 3] {
    let x = cases as f64;
    let h = z(conf) * libm::sqrt(x);
    [x / pt, (x - h) / pt, (x + h) / pt]
}

/// `poisson.R::rg_ci` — Rothman–Greenland (Boice–Monson) log-method
/// rate CI `[lower, upper]`. At `o = 0` the formula degenerates
/// exactly as in R — lower `exp(-Inf) = 0`, upper `exp(NaN) = NaN`;
/// the caller's `leaf` renders the NaN `{"undefined": "out_of_range"}`
/// rather than inventing a point upper bound.
pub(crate) fn rg_ci(o: u64, pt: f64, conf: f64) -> [f64; 2] {
    if o == 0 {
        return [0.0, f64::NAN];
    }
    let (o, z) = (o as f64, z(conf));
    let lo = libm::log(o / pt);
    let h = z / libm::sqrt(o);
    [libm::exp(lo - h), libm::exp(lo + h)]
}

/// `rates.R::rd_byar` — the rate difference `a/n1 − b/n0` and its
/// Taylor CI (labelled "Byar" in OpenEpi's PersonTime2 output). The
/// caller scales by `per_unit`.
pub(crate) fn rd_byar(a: f64, b: f64, n1: f64, n0: f64, conf: f64) -> [f64; 3] {
    let z = z(conf);
    let dp = a / n1 - b / n0;
    let se = libm::sqrt(a / (n1 * n1) + b / (n0 * n0));
    [dp, dp - z * se, dp + z * se]
}

/// `epitools::rateratio` (`method = "wald"`, `rev = "both"`): the
/// unexposed/exposed rate ratio `(b/n0)/(a/n1)` with log-scale
/// `se = sqrt(1/n1 + 1/n0)` on the person-times.
pub(crate) fn rate_ratio(a: f64, b: f64, n1: f64, n0: f64, conf: f64) -> [f64; 3] {
    let rr = (b / n0) / (a / n1);
    let h = z(conf) * libm::sqrt(1.0 / n1 + 1.0 / n0);
    [rr, rr * libm::exp(-h), rr * libm::exp(h)]
}

/// `pt_zscore` in `fixtures/R/calc.R` — the pooled z-score of two
/// rates in one stratum: `[value, p_1t, p_2t]` with
/// `p_1t = Φ(−|z|)`, `p_2t = 2·p_1t`.
pub(crate) fn pt_zscore(a: f64, b: f64, n1: f64, n0: f64) -> [f64; 3] {
    let (m1, n) = (a + b, n1 + n0);
    let z = (a - n1 * m1 / n) / libm::sqrt(m1 * n1 * n0 / (n * n));
    let p = normal_cdf(-libm::fabs(z));
    [z, p, 2.0 * p]
}

/// `rates.R::ef_person_time` — etiologic fractions in the exposed and
/// population plus prevented fractions, Abramson & Gahlinger PEPI
/// formulas as in `PersonTime2.js`. Returns `[efp, efe, pfp, pfe]`,
/// each `[est, lower, upper]`; the caller scales by 100. EFp/EFe are
/// clamped to `[-1, 1]`; R's `max(-1, min(1, x))` propagates NaN
/// (Rust's `min`/`max` would swallow it), so `clamp1` checks first.
pub(crate) fn ef_person_time(a: f64, b: f64, n1: f64, n0: f64, conf: f64) -> [[f64; 3]; 4] {
    // `clamp` propagates NaN like R's max(-1, min(1, x)) — Rust's
    // `min`/`max` would swallow it.
    let clamp1 = |x: f64| x.clamp(-1.0, 1.0);
    let (re, ru, ro) = (a / n1, b / n0, (a + b) / (n1 + n0));
    let rr = re / ru;
    let h = z(conf) * libm::sqrt(1.0 / a + 1.0 / b);
    let (rr_lo, rr_hi) = (rr * libm::exp(-h), rr * libm::exp(h));
    let efp = clamp1((ro - ru) / ro);
    let efe = clamp1((rr - 1.0) / rr);
    let pfp = (ru - ro) / ru;
    let pfe = (ru - re) / ru;
    let share = n1 / (n1 + n0);
    let efp_lo = (share * (rr_lo - 1.0)) / (share * (rr_lo - 1.0) + 1.0);
    let efp_hi = (share * (rr_hi - 1.0)) / (share * (rr_hi - 1.0) + 1.0);
    let efe_lo = (rr_lo - 1.0) / rr_lo;
    let efe_hi = (rr_hi - 1.0) / rr_hi;
    let pfp_lo = 1.0 - 1.0 / (1.0 - efp_hi);
    let pfp_hi = 1.0 - 1.0 / (1.0 - efp_lo);
    let pfe_lo = 1.0 - rr_hi;
    let pfe_hi = 1.0 - rr_lo;
    [
        [efp, efp_lo, efp_hi],
        [efe, efe_lo, efe_hi],
        [pfp, pfp_lo, pfp_hi],
        [pfe, pfe_lo, pfe_hi],
    ]
}

/// `rates.R::adjust` + `breslow_day_rates` — the stratified
/// person-time block emitted only when `strata.len() > 1`:
/// directly adjusted RR (weights `a·b/m1`), Mantel–Haenszel RR,
/// directly adjusted RD (weights `1/(a/n1² + b/n0²)`), the pooled
/// z-score and the Breslow–Day RR/RD homogeneity tests (df = k − 1).
pub(crate) struct AdjustedRates {
    /// Directly adjusted rate ratio, Taylor CI.
    pub direct_rr: [f64; 3],
    /// Mantel–Haenszel rate ratio, Taylor CI on the log scale.
    pub mh_rr: [f64; 3],
    /// Directly adjusted rate difference, Taylor CI.
    pub direct_rd: [f64; 3],
    /// Pooled z-score value and `2·Φ(−|z|)` p.
    pub zscore: [f64; 2],
    /// Breslow–Day RR homogeneity `χ²` and p.
    pub rr: [f64; 2],
    /// Breslow–Day RD homogeneity `χ²` and p.
    pub rd: [f64; 2],
}

/// The closed-form intermediates of `adjusted_rates` — the explain
/// trace's named values (v1.0 Task 10, spec §6). The run path calls
/// `adjusted_rate_parts` and the explain path reuses it.
pub(crate) struct AdjRateParts {
    /// `sum(w_rr)` — the directly adjusted RR's inverse-variance mass.
    pub sw_rr: f64,
    /// `sum(w_rr·log rr_i) / sw_rr` — the pooled log RR.
    pub adj_ln_rr: f64,
    /// `sum(a·n0/n)` — the MH RR numerator.
    pub mh_num: f64,
    /// `sum(b·n1/n)` — the MH RR denominator.
    pub mh_den: f64,
    /// `sum(m1·n1·n0/n²)` — the GR numerator and z-score variance.
    pub mh_var: f64,
    /// `sqrt(mh_var / (mh_num·mh_den))` — the log-scale MH SE.
    pub se_mh: f64,
    /// `sum(w_rd)`.
    pub sw_rd: f64,
    /// `sum(w_rd·rd_i) / sw_rd` — the pooled RD.
    pub adj_rd: f64,
    /// `sum(a)`.
    pub sum_a: f64,
    /// `sum(m1·n1/n)` — the pooled expectation of `a`.
    pub sum_e: f64,
    /// The pooled z-score.
    pub zs: f64,
    /// Woolf χ² of the stratum log-RRs around `adj_ln_rr`.
    pub chi_rr: f64,
    /// Woolf χ² of the stratum RDs around `adj_rd`.
    pub chi_rd: f64,
}

/// The intermediates of `adjusted_rates(strata, _)` — shared by the
/// result and the explain trace.
pub(crate) fn adjusted_rate_parts(strata: &[[f64; 4]]) -> AdjRateParts {
    let sum = |f: &dyn Fn(&[f64; 4]) -> f64| neumaier_sum(strata.iter().map(f));
    let m1 = |t: &[f64; 4]| t[0] + t[1];
    let n = |t: &[f64; 4]| t[2] + t[3];
    let rr_i = |t: &[f64; 4]| (t[0] / t[2]) / (t[1] / t[3]);
    let rd_i = |t: &[f64; 4]| t[0] / t[2] - t[1] / t[3];
    let w_rr = |t: &[f64; 4]| t[0] * t[1] / m1(t);
    let w_rd = |t: &[f64; 4]| 1.0 / (t[0] / (t[2] * t[2]) + t[1] / (t[3] * t[3]));

    let sw_rr = sum(&w_rr);
    let adj_ln_rr = sum(&|t| w_rr(t) * libm::log(rr_i(t))) / sw_rr;

    let mh_num = sum(&|t| t[0] * t[3] / n(t));
    let mh_den = sum(&|t| t[1] * t[2] / n(t));
    let mh_var = sum(&|t| m1(t) * t[2] * t[3] / (n(t) * n(t)));
    let se_mh = libm::sqrt(mh_var / (mh_num * mh_den));

    let sw_rd = sum(&w_rd);
    let adj_rd = sum(&|t| w_rd(t) * rd_i(t)) / sw_rd;

    let sum_a = sum(&|t| t[0]);
    let sum_e = sum(&|t| m1(t) * t[2] / n(t));
    let zs = (sum_a - sum_e) / libm::sqrt(mh_var);

    let chi_rr = sum(&|t| {
        let d = libm::log(rr_i(t)) - adj_ln_rr;
        w_rr(t) * d * d
    });
    let chi_rd = sum(&|t| {
        let d = rd_i(t) - adj_rd;
        w_rd(t) * d * d
    });
    AdjRateParts {
        sw_rr,
        adj_ln_rr,
        mh_num,
        mh_den,
        mh_var,
        se_mh,
        sw_rd,
        adj_rd,
        sum_a,
        sum_e,
        zs,
        chi_rr,
        chi_rd,
    }
}

/// The `strata` rows are `[a, b, n1, n0]` (cases and person-time,
/// exposed then unexposed).
pub(crate) fn adjusted_rates(strata: &[[f64; 4]], conf: f64) -> AdjustedRates {
    let z = z(conf);
    let k = strata.len() as f64;
    let p = adjusted_rate_parts(strata);
    let adj_rr = libm::exp(p.adj_ln_rr);
    let h_rr = z / libm::sqrt(p.sw_rr);
    let h_rd = z / libm::sqrt(p.sw_rd);
    let mh_rr = p.mh_num / p.mh_den;
    let zs_p = 2.0 * normal_cdf(-libm::fabs(p.zs));
    AdjustedRates {
        direct_rr: [adj_rr, adj_rr * libm::exp(-h_rr), adj_rr * libm::exp(h_rr)],
        mh_rr: [
            mh_rr,
            mh_rr * libm::exp(-z * p.se_mh),
            mh_rr * libm::exp(z * p.se_mh),
        ],
        direct_rd: [p.adj_rd, p.adj_rd - h_rd, p.adj_rd + h_rd],
        zscore: [p.zs, zs_p],
        rr: [p.chi_rr, chisq_sf(p.chi_rr, k - 1.0)],
        rd: [p.chi_rd, chisq_sf(p.chi_rd, k - 1.0)],
    }
}
