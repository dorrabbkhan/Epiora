#![cfg(feature = "corpus")]

use epiora_core::corpus::compare::compare;
use epiora_core::corpus::run_special::run_special;
use epiora_core::special::{
    beta_quantile, binomial_quantile, binomial_tail, brent_root, chisq_cdf, chisq_quantile,
    chisq_sf, f_sf, gamma_quantile, hypergeom_pmf, ln_gamma, normal_cdf, normal_quantile,
    normal_sf, poisson_tail, t_cdf, t_quantile, t_sf, NcHypergeom, RootError,
};
use proptest::prelude::*;
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

// ---------------------------------------------------------------------------
// v1.0 Task 5: reference tests for the seven new `special` functions. The
// plan's `*_matches_grid` corpus tests land with the follow-up fixtures-only
// PR that adds `fixtures/special/*.json` (owner decision 2026-11-13, option
// (a): this PR touches nothing under `fixtures/`). The reference values
// below were produced on R 4.6.2 by this ONE command — never typed by hand:
//
//   Rscript -e 'p <- function(tag, g) cat(tag, "|", paste(sprintf("%.17g", g), collapse = " "), "\n", sep = "")
//   ts <- c(-10, -2, 0, 0.5, 2, 10)
//   for (df in c(0.5, 1, 4, 30, 1e6)) p(paste0("t_cdf df=", df), pt(ts, df))
//   for (df in c(0.5, 1, 30)) p(paste0("t_sf df=", df), pt(ts, df, lower.tail = FALSE))
//   ps <- c(1e-6, 0.025, 0.5, 0.975, 1 - 1e-6)
//   for (df in c(0.5, 1, 4, 30, 1e6)) p(paste0("t_quantile df=", df), qt(ps, df))
//   xs <- c(0, 0.5, 1, 3, 10, 1e4)
//   for (d in list(c(1, 1), c(4, 10), c(30, 60), c(1e6, 2))) p(paste0("f_sf df1=", d[1], " df2=", d[2]), pf(xs, d[1], d[2], lower.tail = FALSE))
//   gp <- c(1e-6, 0.01, 0.5, 0.99, 1 - 1e-6)
//   for (a in c(0.1, 1, 5, 100)) p(paste0("gamma_quantile shape=", a, " scale=1"), qgamma(gp, a, 1))
//   p("gamma_quantile shape=5 scale=2.5", qgamma(gp, 5, scale = 2.5))
//   cp <- c(1e-6, 0.025, 0.5, 0.975, 1 - 1e-6)
//   for (df in c(0, 0.5, 1, 4, 30, 490)) p(paste0("chisq_quantile df=", df), qchisq(cp, df))
//   ks <- c(0, 2, 5, 50)
//   for (l in c(0, 0.5, 3, 20, 100)) p(paste0("poisson_tail lower lambda=", l), ppois(ks, l))
//   for (l in c(0, 0.5, 3, 20, 100)) p(paste0("poisson_tail upper lambda=", l), ppois(ks - 1, l, lower.tail = FALSE))
//   bq <- rbind(c(0.5, 10, 0.5), c(0.025, 10, 0.5), c(0.975, 10, 0.5), c(0.5, 54, 0.5), c(0.25, 20, 0.25), c(0.9, 30, 0.4), c(0.5, 1, 0.5), c(0.99, 50, 0.9), c(0.05, 100, 0.01), c(0.5, 0, 0.5), c(0.5, 10, 0), c(0.5, 10, 1), c(0, 10, 0.5), c(1, 10, 0.5), c(1, 10, 0))
//   for (i in seq_len(nrow(bq))) p(sprintf("binomial_quantile p=%g n=%g prob=%g", bq[i, 1], bq[i, 2], bq[i, 3]), qbinom(bq[i, 1], bq[i, 2], bq[i, 3]))'
//
// Verbatim output (each row's args are the R grid above, in order):
//
//   t_cdf df=0.5|0.10133867638566559 0.22275744509156564 0.5 0.62134096353528168 0.7772425549084343 0.89866132361433437
//   t_cdf df=1|0.03172551743055356 0.14758361765043321 0.5 0.64758361765043326 0.85241638234956674 0.96827448256944648
//   t_cdf df=4|0.00028100181135799556 0.058058261758407753 0.5 0.6783350184090684 0.94194173824159222 0.99971899818864207
//   t_cdf df=30|2.2876257041148048e-11 0.02731252248149155 0.5 0.68963849755743634 0.97268747751850837 0.99999999997712374
//   t_cdf df=1e+06|7.6393053840891504e-24 0.022750266925659592 0.5 0.69146240626381417 0.97724973307434038 1
//   t_sf df=0.5|0.89866132361433437 0.7772425549084343 0.5 0.37865903646471832 0.22275744509156564 0.10133867638566559
//   t_sf df=1|0.96827448256944648 0.85241638234956674 0.5 0.35241638234956679 0.14758361765043321 0.03172551743055356
//   t_sf df=30|0.99999999997712374 0.97268747751850837 0.5 0.31036150244256366 0.02731252248149155 2.2876257041148048e-11
//   t_quantile df=0.5|-102849115631.63504 -164.55767348049446 2.5730454951430586e-16 164.55767348049446 102849115631.43192
//   (p = 0.5, df = 0.5 prints R's Newton residual 2.57e-16; the exact
//   median of a symmetric distribution is 0 — the table below uses 0)
//   t_quantile df=1|-318309.88618274347 -12.706204736174707 0 12.706204736174694 318309.88617359026
//   t_quantile df=4|-41.577854150450989 -2.7764451051977943 0 2.7764451051977934 41.577854150151516
//   t_quantile df=30|-5.8711171204188828 -2.0422724563012382 0 2.0422724563012378 5.8711171204086243
//   t_quantile df=1e+06|-4.75345234827968 -1.9599663568141066 0 1.9599663568141066 4.7534523482738704
//   f_sf df1=1 df2=1|1 0.60817344796939288 0.50000000000000044 0.33333333333333326 0.19498222904213663 0.0063659855298165095
//   f_sf df1=4 df2=10|1 0.73677554869684481 0.45155504934168589 0.072323222288140254 0.0015999999999999999 5.8508373975398936e-18
//   f_sf df1=30 df2=60|1 0.9796691300668291 0.48593666548465297 0.00014619508009797787 4.4478477233513007e-14 1.2236257535441111e-100
//   f_sf df1=1e+06 df2=2|1 0.86466417542261531 0.63212019094942296 0.28346860981165128 0.095162572915667376 9.9995000156663589e-05
//   gamma_quantile shape=0.1 scale=1|6.073048362407954e-61 6.0730483624079208e-21 0.0005933911044602256 1.5884778179295074 9.4570270604574418
//   gamma_quantile shape=1 scale=1|1.0000005000003334e-06 0.010050335853501442 0.69314718055994529 4.6051701859880909 13.815510557935518
//   gamma_quantile shape=5 scale=1|0.16906300162147725 1.2791060800936029 4.6709088827959846 11.604625579477178 23.431523423357842
//   gamma_quantile shape=100 scale=1|59.436320698122898 78.215983053795824 99.666864919315429 124.72256149072079 154.91904599496155
//   gamma_quantile shape=5 scale=2.5|0.42265750405369312 3.1977652002340076 11.677272206989961 29.011563948692945 58.578808558394606
//   chisq_quantile df=0|0 0 0 0 0
//   chisq_quantile df=0.5|1.3499395786223463e-24 5.2732025912599957e-07 0.087347604705746873 3.4332352999607787 21.37563515282196
//   chisq_quantile df=1|1.5707963267957164e-12 0.00098206911717525617 0.45493642311957283 5.0238861873148846 23.928126976879472
//   chisq_quantile df=4|0.0028297613229586859 0.48441855708792975 3.356693980033322 11.143286781877798 33.37684158165888
//   chisq_quantile df=30|6.2002937383435928 16.790772265566627 29.336031516661588 46.979242243671152 82.044143136957018
//   chisq_quantile df=490|355.3103661317304 430.55912579827441 489.33349482338781 553.22837250687246 653.4492860940436
//   poisson_tail lower lambda=0|1 1 1 1
//   poisson_tail lower lambda=0.5|0.60653065971263342 0.98561232203302929 0.99998583506267757 0.99999999999999989
//   poisson_tail lower lambda=3|0.049787068367863944 0.42319008112684348 0.91608205796869657 1
//   poisson_tail lower lambda=20|2.0611536224385579e-09 4.5551495055892125e-07 7.1908840528428925e-05 0.99999999517126248
//   poisson_tail lower lambda=100|3.7200759760208361e-44 1.8976107553682281e-40 3.2614563667204693e-36 2.4015922356168156e-08
//   poisson_tail upper lambda=0|1 0 0 0
//   poisson_tail upper lambda=0.5|1 0.090204010431049877 0.00017211562995584072 1.7887765104351365e-80
//   poisson_tail upper lambda=3|1 0.80085172652854419 0.18473675547622787 1.248535487154193e-42
//   poisson_tail upper lambda=20|1 0.99999995671577391 0.9999830552560699 1.2458926079719381e-08
//   poisson_tail upper lambda=100|1 1 1 0.99999998821549929
//   binomial_quantile p=0.5 n=10 prob=0.5|5
//   binomial_quantile p=0.025 n=10 prob=0.5|2
//   binomial_quantile p=0.975 n=10 prob=0.5|8
//   binomial_quantile p=0.5 n=54 prob=0.5|27
//   binomial_quantile p=0.25 n=20 prob=0.25|4
//   binomial_quantile p=0.9 n=30 prob=0.4|15
//   binomial_quantile p=0.5 n=1 prob=0.5|0
//   binomial_quantile p=0.99 n=50 prob=0.9|49
//   binomial_quantile p=0.05 n=100 prob=0.01|0
//   binomial_quantile p=0.5 n=0 prob=0.5|0
//   binomial_quantile p=0.5 n=10 prob=0|0
//   binomial_quantile p=0.5 n=10 prob=1|10
//   binomial_quantile p=0 n=10 prob=0.5|0
//   binomial_quantile p=1 n=10 prob=0.5|10
//   binomial_quantile p=1 n=10 prob=0|10
//
// Tolerances: closed-form-style evaluations (t_cdf, t_sf, f_sf,
// poisson_tail) at the `closed_form` class 1e-9; root-found quantiles
// (t_quantile, gamma_quantile, chisq_quantile) at `iterative` 1e-6;
// binomial_quantile exact (rule E7: |a−e| ≤ tol·|e|, |a| ≤ tol if e = 0).

fn ref_check(expected: f64, actual: f64, tol: f64, what: String) {
    let ok = if expected == 0.0 {
        actual.abs() <= tol
    } else {
        (actual - expected).abs() <= tol * expected.abs()
    };
    assert!(ok, "{what}: actual {actual:e}, expected {expected:e}");
}

const TS: [f64; 6] = [-10.0, -2.0, 0.0, 0.5, 2.0, 10.0];

#[rustfmt::skip]
const T_CDF_REF: [(f64, [f64; 6]); 5] = [
    (0.5, [0.10133867638566559, 0.22275744509156564, 0.5, 0.6213409635352817, 0.7772425549084343, 0.8986613236143344]),
    (1.0, [0.03172551743055356, 0.1475836176504332, 0.5, 0.6475836176504333, 0.8524163823495667, 0.9682744825694465]),
    (4.0, [0.00028100181135799556, 0.05805826175840775, 0.5, 0.6783350184090684, 0.9419417382415922, 0.9997189981886421]),
    (30.0, [2.2876257041148048e-11, 0.02731252248149155, 0.5, 0.6896384975574363, 0.9726874775185084, 0.9999999999771237]),
    (1e6, [7.63930538408915e-24, 0.022750266925659592, 0.5, 0.6914624062638142, 0.9772497330743404, 1.0]),
];

#[test]
fn t_cdf_matches_reference() {
    for (df, expected) in T_CDF_REF {
        for (t, e) in TS.iter().zip(expected) {
            ref_check(e, t_cdf(*t, df), 1e-9, format!("t_cdf({t}, {df})"));
        }
    }
}

#[rustfmt::skip]
const T_SF_REF: [(f64, [f64; 6]); 3] = [
    (0.5, [0.8986613236143344, 0.7772425549084343, 0.5, 0.3786590364647183, 0.22275744509156564, 0.10133867638566559]),
    (1.0, [0.9682744825694465, 0.8524163823495667, 0.5, 0.3524163823495668, 0.1475836176504332, 0.03172551743055356]),
    (30.0, [0.9999999999771237, 0.9726874775185084, 0.5, 0.31036150244256366, 0.02731252248149155, 2.2876257041148048e-11]),
];

#[test]
fn t_sf_matches_reference() {
    for (df, expected) in T_SF_REF {
        for (t, e) in TS.iter().zip(expected) {
            ref_check(e, t_sf(*t, df), 1e-9, format!("t_sf({t}, {df})"));
        }
    }
}

const QUANTILE_PS: [f64; 5] = [1e-6, 0.025, 0.5, 0.975, 0.999999];

#[rustfmt::skip]
const T_QUANTILE_REF: [(f64, [f64; 5]); 5] = [
    // df=0.5, p=0.5 uses the exact median 0, not R's printed residual.
    (0.5, [-102849115631.63504, -164.55767348049446, 0.0, 164.55767348049446, 102849115631.43192]),
    (1.0, [-318309.8861827435, -12.706204736174707, 0.0, 12.706204736174694, 318309.88617359026]),
    (4.0, [-41.57785415045099, -2.7764451051977943, 0.0, 2.7764451051977934, 41.577854150151516]),
    (30.0, [-5.871117120418883, -2.042272456301238, 0.0, 2.0422724563012378, 5.871117120408624]),
    (1e6, [-4.75345234827968, -1.9599663568141066, 0.0, 1.9599663568141066, 4.75345234827387]),
];

#[test]
fn t_quantile_matches_reference() {
    for (df, expected) in T_QUANTILE_REF {
        for (p, e) in QUANTILE_PS.iter().zip(expected) {
            let actual = t_quantile(*p, df).expect("t_quantile in domain");
            ref_check(e, actual, 1e-6, format!("t_quantile({p}, {df})"));
        }
    }
}

#[rustfmt::skip]
const F_SF_XS: [f64; 6] = [0.0, 0.5, 1.0, 3.0, 10.0, 1e4];
#[rustfmt::skip]
const F_SF_REF: [((f64, f64), [f64; 6]); 4] = [
    ((1.0, 1.0), [1.0, 0.6081734479693929, 0.5000000000000004, 0.33333333333333326, 0.19498222904213663, 0.0063659855298165095]),
    ((4.0, 10.0), [1.0, 0.7367755486968448, 0.4515550493416859, 0.07232322228814025, 0.0015999999999999999, 5.8508373975398936e-18]),
    ((30.0, 60.0), [1.0, 0.9796691300668291, 0.485936665484653, 0.00014619508009797787, 4.447847723351301e-14, 1.223625753544111e-100]),
    ((1e6, 2.0), [1.0, 0.8646641754226153, 0.632120190949423, 0.2834686098116513, 0.09516257291566738, 9.999500015666359e-05]),
];

#[test]
fn f_sf_matches_reference() {
    for ((df1, df2), expected) in F_SF_REF {
        for (x, e) in F_SF_XS.iter().zip(expected) {
            ref_check(
                e,
                f_sf(*x, df1, df2),
                1e-9,
                format!("f_sf({x}, {df1}, {df2})"),
            );
        }
    }
}

#[rustfmt::skip]
const GAMMA_QUANTILE_REF: [((f64, f64), [f64; 5]); 5] = [
    ((0.1, 1.0), [6.073048362407954e-61, 6.073048362407921e-21, 0.0005933911044602256, 1.5884778179295074, 9.457027060457442]),
    ((1.0, 1.0), [1.0000005000003334e-06, 0.010050335853501442, std::f64::consts::LN_2, 4.605170185988091, 13.815510557935518]),
    ((5.0, 1.0), [0.16906300162147725, 1.279106080093603, 4.670908882795985, 11.604625579477178, 23.43152342335784]),
    ((100.0, 1.0), [59.4363206981229, 78.21598305379582, 99.66686491931543, 124.72256149072079, 154.91904599496155]),
    ((5.0, 2.5), [0.4226575040536931, 3.1977652002340076, 11.677272206989961, 29.011563948692945, 58.578808558394606]),
];

const GAMMA_PS: [f64; 5] = [1e-6, 0.01, 0.5, 0.99, 0.999999];

#[test]
fn gamma_quantile_matches_reference() {
    for ((shape, scale), expected) in GAMMA_QUANTILE_REF {
        for (p, e) in GAMMA_PS.iter().zip(expected) {
            let actual = gamma_quantile(*p, shape, scale)
                .unwrap_or_else(|e| panic!("gamma_quantile({p}, {shape}, {scale}): {e:?}"));
            ref_check(
                e,
                actual,
                1e-6,
                format!("gamma_quantile({p}, {shape}, {scale})"),
            );
        }
    }
}

#[rustfmt::skip]
const CHISQ_QUANTILE_REF: [(f64, [f64; 5]); 6] = [
    (0.0, [0.0; 5]),
    (0.5, [1.3499395786223463e-24, 5.273202591259996e-07, 0.08734760470574687, 3.4332352999607787, 21.37563515282196]),
    (1.0, [1.5707963267957164e-12, 0.0009820691171752562, 0.45493642311957283, 5.023886187314885, 23.928126976879472]),
    (4.0, [0.002829761322958686, 0.48441855708792975, 3.356693980033322, 11.143286781877798, 33.37684158165888]),
    (30.0, [6.200293738343593, 16.790772265566627, 29.33603151666159, 46.97924224367115, 82.04414313695702]),
    (490.0, [355.3103661317304, 430.5591257982744, 489.3334948233878, 553.2283725068725, 653.4492860940436]),
];

#[test]
fn chisq_quantile_matches_reference() {
    for (df, expected) in CHISQ_QUANTILE_REF {
        for (p, e) in QUANTILE_PS.iter().zip(expected) {
            let actual = chisq_quantile(*p, df).expect("chisq_quantile in domain");
            ref_check(e, actual, 1e-6, format!("chisq_quantile({p}, {df})"));
        }
    }
}

const POISSON_KS: [u64; 4] = [0, 2, 5, 50];

#[rustfmt::skip]
const POISSON_REF: [(f64, [f64; 4], [f64; 4]); 5] = [
    (0.0, [1.0, 1.0, 1.0, 1.0], [1.0, 0.0, 0.0, 0.0]),
    (0.5, [0.6065306597126334, 0.9856123220330293, 0.9999858350626776, 0.9999999999999999],
        [1.0, 0.09020401043104988, 0.00017211562995584072, 1.7887765104351365e-80]),
    (3.0, [0.049787068367863944, 0.4231900811268435, 0.9160820579686966, 1.0],
        [1.0, 0.8008517265285442, 0.18473675547622787, 1.248535487154193e-42]),
    (20.0, [2.061153622438558e-09, 4.5551495055892125e-07, 7.190884052842893e-05, 0.9999999951712625],
        [1.0, 0.9999999567157739, 0.9999830552560699, 1.2458926079719381e-08]),
    (100.0, [3.720075976020836e-44, 1.897610755368228e-40, 3.261456366720469e-36, 2.4015922356168156e-08],
        [1.0, 1.0, 1.0, 0.9999999882154993]),
];

#[test]
fn poisson_tail_matches_reference() {
    for (lambda, lo_exp, up_exp) in POISSON_REF {
        for ((k, e_lo), e_up) in POISSON_KS.iter().zip(lo_exp).zip(up_exp) {
            ref_check(
                e_lo,
                poisson_tail(*k, lambda, false),
                1e-9,
                format!("poisson_tail({k}, {lambda}, lower)"),
            );
            ref_check(
                e_up,
                poisson_tail(*k, lambda, true),
                1e-9,
                format!("poisson_tail({k}, {lambda}, upper)"),
            );
        }
    }
}

#[test]
fn binomial_quantile_matches_reference() {
    for (p, n, prob, expected) in [
        (0.5, 10, 0.5, 5),
        (0.025, 10, 0.5, 2),
        (0.975, 10, 0.5, 8),
        (0.5, 54, 0.5, 27),
        (0.25, 20, 0.25, 4),
        (0.9, 30, 0.4, 15),
        (0.5, 1, 0.5, 0),
        (0.99, 50, 0.9, 49),
        (0.05, 100, 0.01, 0),
        (0.5, 0, 0.5, 0),
        (0.5, 10, 0.0, 0),
        (0.5, 10, 1.0, 10),
        (0.0, 10, 0.5, 0),
        (1.0, 10, 0.5, 10),
        (1.0, 10, 0.0, 10),
    ] {
        assert_eq!(
            binomial_quantile(p, n, prob),
            expected,
            "binomial_quantile({p}, {n}, {prob})"
        );
    }
}

// ---------------------------------------------------------------------------
// Fix-round regressions (PR #69 review).

/// `gamma_quantile(0.5, 1e-300, 1)` hung: `u_est ≈ −7e299` and the
/// `hi += 4` bracket expansion could never reach its bound (reachable
/// through `chisq_quantile` too). Shape ~1e-300 makes
/// `P(a, x) ≈ x^a = 1` on every positive f64, so the correct f64
/// quantile is 0 — assert the call terminates and any `Ok` is finite
/// and ≥ 0.
#[test]
fn gamma_quantile_tiny_shape_terminates() {
    for v in [
        gamma_quantile(0.5, 1e-300, 1.0),
        gamma_quantile(1e-6, 1e-300, 1.0),
        gamma_quantile(0.999999, 1e-300, 2.5),
        chisq_quantile(0.5, 1e-300),
        chisq_quantile(1e-6, 1e-300),
    ]
    .into_iter()
    .flatten()
    {
        assert!(v.is_finite() && v >= 0.0, "bad value {v:e}");
    }
    // P(a, min subnormal) = 1 ≥ p pins the underflowed-quantile arm.
    assert_eq!(gamma_quantile(0.5, 1e-300, 1.0), Ok(0.0));
}

/// `binomial_quantile` was O(n·log n) (≈45 s at n = 1e8): binary
/// search over `binomial_tail`'s O(n) literal sum. It now searches
/// `pbinom`'s `pbeta` identity `I_{1−prob}(n−k, k+1)`, O(log n)
/// constant-time evaluations. Reference values from ONE command
/// (R 4.6.2):
///
///   Rscript -e 'sprintf("%.17g", qbinom(c(1e-6,0.5,1-1e-6), 1e9, 0.3))'
///
/// output: `299931118 300000000 300068885` — the whole test runs in
/// well under a second (see the `cargo test` report).
#[test]
fn binomial_quantile_large_n_is_fast() {
    let n = 1_000_000_000u64;
    for (p, expected) in [
        (1e-6, 299_931_118u64),
        (0.5, 300_000_000),
        (1.0 - 1e-6, 300_068_885),
    ] {
        assert_eq!(binomial_quantile(p, n, 0.3), expected, "p = {p}");
    }
}

/// `poisson_tail(k, …)` computed `k + 1` in plain u64: a debug-build
/// panic at k = u64::MAX. `P(X ≤ u64::MAX) = 1` and
/// `P(X ≥ u64::MAX) = P(X = u64::MAX) = 0` for lambda 3 (R gives the
/// same: `ppois` saturates at the integer ceiling of double space).
#[test]
fn poisson_tail_u64_max_no_panic() {
    assert_eq!(poisson_tail(u64::MAX, 3.0, false), 1.0);
    assert_eq!(poisson_tail(u64::MAX, 3.0, true), 0.0);
}

#[test]
fn run_special_accepts_new_grid_names() {
    // In-memory fixtures in the `inputs.grid` shape
    // `fixtures/R/special.R` will emit in the fixtures-only follow-up PR
    // (spec §4.1, plan Task 2): every new `special.*` name runs through
    // the corpus runner without a file under `fixtures/`.
    for (name, grid) in [
        ("t_cdf", vec![vec![0.0, 1.0], vec![2.0, 30.0]]),
        ("t_sf", vec![vec![0.0, 1.0], vec![2.0, 30.0]]),
        ("t_quantile", vec![vec![0.975, 30.0], vec![0.025, 0.5]]),
        ("f_sf", vec![vec![3.0, 4.0, 10.0]]),
        ("gamma_quantile", vec![vec![0.5, 5.0, 2.0]]),
        ("chisq_quantile", vec![vec![0.975, 4.0]]),
        (
            "poisson_tail",
            vec![vec![5.0, 3.0, 0.0], vec![5.0, 3.0, 1.0]],
        ),
        ("binomial_quantile", vec![vec![0.5, 10.0, 0.5]]),
    ] {
        let fixture = json!({
            "id": name,
            "procedure": format!("special.{name}"),
            "inputs": { "grid": grid },
        });
        let out = run_special(&fixture).unwrap_or_else(|e| panic!("{name}: {e}"));
        let values = out["values"].as_array().expect("values array");
        assert_eq!(values.len(), grid.len(), "{name}: value count");
        for v in values {
            assert!(v.is_f64() || v.is_u64(), "{name}: {v} is not numeric");
        }
    }
}

proptest! {
    /// `t_quantile(p, df) = -t_quantile(1 - p, df)` within 1e-9
    /// relative (the plan's `t_quantile_symmetry`).
    #[test]
    fn t_quantile_symmetry(p in 1e-12..=0.5f64, df in 0.05..1e4f64) {
        let a = t_quantile(p, df).expect("t_quantile(p)");
        let b = t_quantile(1.0 - p, df).expect("t_quantile(1-p)");
        let diff = (a + b).abs();
        prop_assert!(
            diff <= 1e-9 * a.abs() || diff <= 1e-9,
            "t_quantile({p}, {df}) = {a}, t_quantile({}, {df}) = {b}",
            1.0 - p
        );
    }

    /// `P(X ≤ k) + P(X ≥ k+1) = 1` within 1e-12 (the plan's
    /// `poisson_tail_complements`).
    #[test]
    fn poisson_tail_complements(k in 0..500u64, lambda in 0.0..1e4f64) {
        let lower = poisson_tail(k, lambda, false);
        let upper = poisson_tail(k + 1, lambda, true);
        prop_assert!(
            (lower + upper - 1.0).abs() <= 1e-12,
            "poisson_tail({k}, {lambda}, lower) + poisson_tail({}, {lambda}, upper) = {}",
            k + 1,
            lower + upper
        );
    }
}
