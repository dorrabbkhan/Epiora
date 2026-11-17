//! `rules` — the five assumption rules of spec §6.1: one pure
//! function per note code, each a predicate over values the run
//! already computed. Every threshold is a named constant whose doc
//! comment carries the literature source; `docs/decisions.md`
//! (v1.0 Task 22) records the same list for owner review.

use super::Note;
use std::collections::BTreeMap;

/// Minimum expected cell count for the chi-square approximation —
/// Cochran (1954), "Some Methods for Strengthening the Common χ²
/// Tests", *Biometrics* 10(2): 417–451 (the engine's `rxc` module
/// applies the same 5 to `low_expected`).
pub const EXPECTED_CELL_MIN: f64 = 5.0;

/// Minimum of `n·p` and `n·(1−p)` for the Wald (normal-approximation)
/// binomial interval — the classical `np ≥ 5` rule of thumb; Brown,
/// Cai & DasGupta (2001), "Interval Estimation for a Binomial
/// Proportion", *Statistical Science* 16(2): 101–133, show the Wald
/// interval's coverage is erratic below it and recommend the Wilson
/// score interval.
pub const NP_MIN: f64 = 5.0;

/// Significance level for the homogeneity diagnostics (Bartlett and
/// Breslow–Day–Tarone): the conventional α = 0.05, matching every
/// other two-sided flag in the engine.
pub const SIGNIFICANCE: f64 = 0.05;

/// One [`Note`]: `statistics` sorted and deduplicated, `values`
/// restricted to finite numbers (a non-finite trigger can never fire
/// — the guards below refuse it before `note` is called).
fn note(code: &str, mut statistics: Vec<String>, values: &[(&str, f64)], suggests: &str) -> Note {
    statistics.sort();
    statistics.dedup();
    Note {
        code: code.into(),
        statistics,
        values: values
            .iter()
            .filter(|(_, v)| v.is_finite())
            .map(|(k, v)| ((*k).to_string(), *v))
            .collect::<BTreeMap<_, _>>(),
        suggests: (!suggests.is_empty()).then(|| suggests.to_string()),
    }
}

/// `expected_count_below_5` — at least one expected cell below
/// [`EXPECTED_CELL_MIN`]. `expected` is the engine's own expected-cell
/// set of the table the chi-square statistics were computed on
/// (`two_by_two_parts::e`, `oe_cells`, `rxc_parts::expected`); the
/// `fisher` argument is the exact p the 2×2 blocks suggest, `""` on
/// an R×C where the result carries no exact alternative.
pub(super) fn expected_below_5(
    expected: &[f64],
    statistics: Vec<String>,
    fisher: &str,
    extra: &[(&str, f64)],
) -> Option<Note> {
    let lt5 = expected
        .iter()
        .filter(|e| e.is_finite() && **e < EXPECTED_CELL_MIN)
        .count() as f64;
    if lt5 == 0.0 {
        return None;
    }
    let min = expected
        .iter()
        .copied()
        .filter(|e| e.is_finite())
        .fold(f64::INFINITY, f64::min);
    let mut values = vec![("expected_lt5", lt5), ("expected_min", min)];
    values.extend_from_slice(extra);
    Some(note("expected_count_below_5", statistics, &values, fisher))
}

/// `zero_cell` — an observed zero cell in the table the Wald log-scale
/// OR/RR intervals were computed on: the `1/cell` terms of their
/// standard errors make the intervals undefined or unreliable
/// (Agresti 2003, *Categorical Data Analysis* 2nd ed., §2.3.3, on the
/// Wald OR interval's failure at zero counts). `cells` are the
/// observed counts the interval's SE sums over — `[a, b, c, d]` for a
/// full 2×2, the two discordant cells for a matched 1:1 pair.
pub(super) fn zero_cell(
    cells: &[u64],
    statistics: Vec<String>,
    exact: &str,
    extra: &[(&str, f64)],
) -> Option<Note> {
    let zeros = cells.iter().filter(|c| **c == 0).count() as f64;
    if zeros == 0.0 {
        return None;
    }
    let mut values = vec![("zero_cells", zeros)];
    values.extend_from_slice(extra);
    Some(note("zero_cell", statistics, &values, exact))
}

/// `normal_approx_small` — `n·p` or `n·(1−p)` below [`NP_MIN`] for the
/// Wald proportion interval of `x`/`n` (`x` is `n·p`; `n − x` is
/// `n·(1−p)` — the engine's own counts).
pub(super) fn normal_approx_small(
    x: u64,
    n: u64,
    statistics: Vec<String>,
    alternative: &str,
) -> Option<Note> {
    if x > n {
        return None;
    }
    let (np, nq) = (x as f64, (n - x) as f64);
    (np.min(nq) < NP_MIN).then(|| {
        note(
            "normal_approx_small",
            statistics,
            &[("np", np), ("n_one_minus_p", nq)],
            alternative,
        )
    })
}

/// `unequal_variances` — Bartlett's test of equal variances rejects at
/// [`SIGNIFICANCE`] (Bartlett 1937, "Properties of Sufficiency and
/// Statistical Tests", *Proc. R. Soc. London* A 160: 268–282), so the
/// equal-variance statistics (pooled t, ANOVA, pooled group CIs) are
/// suspect.
pub(super) fn unequal_variances(
    bartlett_p: f64,
    statistics: Vec<String>,
    alternative: &str,
) -> Option<Note> {
    (bartlett_p.is_finite() && bartlett_p < SIGNIFICANCE).then(|| {
        note(
            "unequal_variances",
            statistics,
            &[("bartlett_p", bartlett_p)],
            alternative,
        )
    })
}

/// `heterogeneous_strata` — the Breslow–Day homogeneity p below
/// [`SIGNIFICANCE`] (Breslow & Day 1980, *Statistical Methods in
/// Cancer Research* I, IARC, §4.4; Tarone 1985): the common-odds-ratio
/// assumption fails, so the pooled Mantel–Haenszel estimates are
/// suspect and the stratum-specific ones are the reportable result.
/// `trigger` names the surface's own p (`breslow_day_tarone_p` for
/// the procedure's Tarone-corrected test, `breslow_day_or_p` for
/// the calculator's Woolf chi-square).
pub(super) fn heterogeneous_strata(
    trigger: (&str, f64),
    statistics: Vec<String>,
    alternative: &str,
) -> Option<Note> {
    let (key, p) = trigger;
    (p.is_finite() && p < SIGNIFICANCE)
        .then(|| note("heterogeneous_strata", statistics, &[(key, p)], alternative))
}
