#![cfg(feature = "corpus")]

use epiora_core::corpus::compare::compare;
use epiora_core::special::{
    beta_quantile, binomial_tail, brent_root, chisq_cdf, chisq_sf, hypergeom_pmf, ln_gamma,
    normal_cdf, normal_quantile, normal_sf, NcHypergeom, RootError,
};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/special")
        .join(format!("{name}.json"))
}

fn load_fixture(name: &str) -> Value {
    let path = fixture_path(name);
    let text = std::fs::read_to_string(&path).expect("read fixture");
    serde_json::from_str(&text).expect("parse fixture")
}

fn num(v: &Value) -> f64 {
    v.as_f64().expect("fixture point must be numeric")
}

fn assert_matches(fixture: &Value, actual: &Value) {
    let mismatches = compare(fixture, actual).expect("compare");
    assert!(
        mismatches.is_empty(),
        "{}: {} mismatches:\n{}",
        fixture["id"],
        mismatches.len(),
        mismatches
            .iter()
            .map(|m| format!("  {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

fn check_scalar(name: &str, f: impl Fn(&[f64]) -> f64) {
    let fixture = load_fixture(name);
    let mut values = Vec::new();
    for point in fixture["inputs"]["points"]
        .as_array()
        .expect("points array")
    {
        let args: Vec<f64> = point
            .as_array()
            .expect("argument array")
            .iter()
            .map(num)
            .collect();
        values.push(f(&args));
    }
    assert_matches(&fixture, &json!({ "values": values }));
}

#[test]
fn special_ln_gamma_matches_fixture() {
    check_scalar("ln_gamma", |a| ln_gamma(a[0]));
}

#[test]
fn special_normal_cdf_matches_fixture() {
    check_scalar("normal_cdf", |a| normal_cdf(a[0]));
}

#[test]
fn special_normal_sf_matches_fixture() {
    check_scalar("normal_sf", |a| normal_sf(a[0]));
}

#[test]
fn special_normal_quantile_matches_fixture() {
    check_scalar("normal_quantile", |a| {
        normal_quantile(a[0]).expect("fixture points are in range")
    });
}

#[test]
fn special_chisq_sf_matches_fixture() {
    check_scalar("chisq_sf", |a| chisq_sf(a[0], a[1]));
}

#[test]
fn special_hypergeom_pmf_matches_fixture() {
    check_scalar("hypergeom_pmf", |a| {
        hypergeom_pmf(a[0] as u64, a[1] as u64, a[2] as u64, a[3] as u64)
    });
}

#[test]
fn special_binomial_tail_matches_fixture() {
    check_scalar("binomial_tail", |a| {
        binomial_tail(a[0] as u64, a[1] as u64, a[2], a[3] == 1.0)
    });
}

#[test]
fn special_beta_quantile_matches_fixture() {
    check_scalar("beta_quantile", |a| {
        beta_quantile(a[0], a[1], a[2]).expect("fixture points are in range")
    });
}

#[test]
fn special_nc_hypergeom_matches_fixture() {
    let fixture = load_fixture("nc_hypergeom");
    let mut values = Vec::new();
    for point in fixture["inputs"]["points"]
        .as_array()
        .expect("points array")
    {
        let a: Vec<f64> = point
            .as_array()
            .expect("argument array")
            .iter()
            .map(num)
            .collect();
        let h = NcHypergeom::new(a[0] as u64, a[1] as u64, a[2] as u64);
        let (x, psi) = (a[3] as u64, a[4]);
        values.push(json!({
            "pmf": h.pmf(x, psi),
            "mean": h.mean(psi),
            "lower_tail": h.lower_tail(x, psi),
            "upper_tail": h.upper_tail(x, psi),
        }));
    }
    assert_matches(&fixture, &json!({ "values": values }));
}

/// `chisq_cdf` against R 4.6.2 `pchisq` — literals copied from the
/// conductor's reference grid `target/conductor/chisq_cdf_ref.csv`
/// (gitignored), produced by
/// `Rscript -e 'g <- expand.grid(x=..., df=...); ... pchisq(g$x, g$df) ...'`.
/// The last case has no grid row (the "x near a" region needs df > 100):
/// `pchisq(500, 490)` is the exact integer-shape Poisson-tail identity
/// `P(245, 250) = 1 − e^−250·Σ_{k≤244} 250^k/k!` evaluated in Decimal.
/// Tolerance is the `closed_form` class (E7): |a−e| ≤ 1e-9·|e|.
#[test]
fn special_chisq_cdf_branches() {
    for (x, df, expected) in [
        // gamma-x < 1: convergent P series.
        (1.0, 4.0, 0.09020401043104988),
        // x <= a−1, x < 0.8(a+50): pd_upper_series × dpois_wrap.
        (30.0, 50.0, 0.011164780271550285),
        // a < 1 branch: pd_lower_cf for the scaled upper tail.
        (10.0, 1.0, 0.9984345977419975),
        // a ≥ 1 branch: pd_lower_series; fractional y = 2.5 also
        // reaches pd_lower_cf.
        (20.0, 7.0, 0.9944303169270544),
        // else region (x ≥ 1 near a): complement of the validated
        // upper tail, R's ppois_asymp region.
        (500.0, 490.0, 0.632526655552547),
    ] {
        let actual = chisq_cdf(x, df);
        assert!(
            (actual - expected).abs() <= 1e-9 * expected.abs(),
            "chisq_cdf({x}, {df}) = {actual:e}, expected {expected:e}"
        );
    }
    // `1 - pchisq` — the quantity DescTools' Breslow-Day uses — is
    // reproduced, not computed from the upper tail.
    let (x, df, expected) = (10.0, 1.0, 1.5654022580025018e-3);
    let actual = 1.0 - chisq_cdf(x, df);
    assert!(
        (actual - expected).abs() <= 1e-9 * expected.abs(),
        "1 - chisq_cdf({x}, {df}) = {actual:e}, expected {expected:e}"
    );
}

#[test]
fn special_chisq_cdf_edges() {
    // R: pchisq(x, df=0) is the point mass at 0; x <= 0 is 0.
    assert_eq!(chisq_cdf(0.0, 1.0), 0.0);
    assert_eq!(chisq_cdf(-1.0, 1.0), 0.0);
    assert_eq!(chisq_cdf(0.0, 0.0), 0.0);
    assert_eq!(chisq_cdf(5.0, 0.0), 1.0);
    assert!(chisq_cdf(f64::NAN, 1.0).is_nan());
    assert!(chisq_cdf(1.0, f64::NAN).is_nan());
    assert!(chisq_cdf(1.0, -1.0).is_nan());
}

#[test]
fn brent_finds_sqrt2() {
    let root = brent_root(|x| x * x - 2.0, 0.0, 2.0).expect("bracketed root");
    assert!((root - std::f64::consts::SQRT_2).abs() <= 1e-14);
}

#[test]
fn brent_not_bracketed() {
    assert_eq!(
        brent_root(|x| x * x - 2.0, 2.0, 3.0),
        Err(RootError::NotBracketed)
    );
}

#[test]
fn quantile_rejects_out_of_range() {
    for p in [0.0, 1.0, -1.0, f64::NAN] {
        assert_eq!(normal_quantile(p), None, "p = {p}");
    }
}
