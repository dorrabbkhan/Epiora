#![cfg(feature = "corpus")]

use epiora_core::corpus::compare::compare;
use epiora_core::corpus::run_special::run_special;
use epiora_core::special::{
    beta_quantile, binomial_quantile, binomial_tail, brent_root, chisq_cdf, chisq_quantile,
    chisq_sf, f_sf, gamma_quantile, hypergeom_pmf, ln_gamma, normal_cdf, normal_quantile,
    normal_sf, poisson_tail, reg_inc_beta, t_cdf, t_quantile, t_sf, NcHypergeom, RootError,
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

// ---------------------------------------------------------------------------
// Round-2 review regressions (PR #69): every table below was produced
// by ONE Rscript command (R 4.6.2), shown in the doc comment, and its
// output machine-converted to a const table — nothing hand-typed.

/// `qbinom` on 210 seeded points — `n` up to 1e9, `prob` and `p`
/// including 0, 1, tiny and 1 − 1e-12 edges. The second review found
/// 2/212 random points off by ±1; with `pbinom` evaluated as
/// `pbeta(prob, k+1, n−k, lower.tail = FALSE)` (the upper
/// incomplete-beta tail computed directly, no `1 −` subtraction) and
/// R's `qDiscrete_search` left-continuity fuzz `p *= 1 − 8ε`, all 210
/// match exactly. Command (seed 42):
///
///   Rscript -e 'set.seed(42)
///   n <- c(sample(1:1000, 60.0, TRUE), sample(1:1e6, 60.0, TRUE), sample(1:1e9, 60.0, TRUE),
///          1, 2, 10, 1000, 1e6, 1e9)
///   prob <- c(runif(186.0, 0.0, 1.0), 0, 1, 1e-12, 1 - 1e-12, 0.5, 1e-9, 1 - 1e-9, 0.3, 0.7, 0.99)[1:186]
///   p <- c(runif(186.0, 0.0, 1.0), 0, 1, 1e-12, 1 - 1e-12, 0.5, 0.001, 0.999, 1e-9, 1 - 1e-9)[1:186]
///   extra_n <- c(1e9, 1e9, 100, 50, 1e6, 1e9, 2, 3, 5, 1e4, 1e9, 1e9, 1e6, 1e9, 10, 1e9, 1e9, 1e5, 1e7, 1e8, 1e9, 1)
///   extra_prob <- c(0.5, 1e-9, 0.5, 0.9, 1e-6, 0.7, 0.4, 0.6, 0.2, 0.8, 1e-12, 1 - 1e-15, 1 - 1e-12, 1e-15, 0.5, 0.999999999, 1e-10, 1 - 1e-10, 0.123456789, 0.0001, 1 - 1e-8, 0.5)
///   extra_p <- c(1e-12, 1 - 1e-12, 0.9999999, 1e-9, 1 - 1e-15, 0.5, 0.1, 0.9, 0.99, 1e-12, 0.999999, 1 - 1e-12, 0.75, 1e-10, 0.5, 1 - 1e-9, 0.5, 0.99999999, 0.4321, 1e-10, 1 - 1e-14, 1)
///   n <- c(n, extra_n, 1e9, 1000000); prob <- c(prob, extra_prob, 1 - 1e-12, 1e-12); p <- c(p, extra_p, 0.9999, 1e-8)
///   q <- mapply(function(pp, nn, pr) qbinom(pp, nn, pr), p, n, prob)
///   for (i in seq_along(q)) cat(sprintf("%.0f, %.17g, %.17g, %.0f\n", n[i], prob[i], p[i], q[i]))'
#[rustfmt::skip]
const QBINOM_R_REF: [(u64, f64, f64, u64); 210] = [
    (561_u64, 0.14190889545716345, 0.9028057963587344, 90_u64),
    (997_u64, 0.279806696344167, 0.2530024694278836, 269_u64),
    (321_u64, 0.56482222257182, 0.1505976002663374, 172_u64),
    (153_u64, 0.9351395054254681, 0.76854715985246, 145_u64),
    (74_u64, 0.35840014507994056, 0.23012333107180893, 23_u64),
    (228_u64, 0.8420071809086949, 0.3053993280045688, 189_u64),
    (146_u64, 0.7224092143587768, 0.5185696121770889, 106_u64),
    (634_u64, 0.7507359893061221, 0.33459966676309705, 471_u64),
    (49_u64, 0.92398845218122, 0.15443496825173497, 43_u64),
    (128_u64, 0.002378106815740466, 0.2663695686496794, 0_u64),
    (303_u64, 0.16042991494759917, 0.3507546167820692, 46_u64),
    (24_u64, 0.399272951297462, 0.5784583787899464, 10_u64),
    (839_u64, 0.6753195773344487, 0.8086017952300608, 578_u64),
    (356_u64, 0.4803720163181424, 0.9332703149411827, 185_u64),
    (601_u64, 0.533828780753538, 0.8338633375242352, 333_u64),
    (165_u64, 0.31695015728473663, 0.12700277450494468, 45_u64),
    (622_u64, 0.8147575901821256, 0.6494539515115321, 511_u64),
    (532_u64, 0.29221952124498785, 0.6903516575694084, 161_u64),
    (410_u64, 0.4091320913285017, 0.03204482444562018, 149_u64),
    (882_u64, 0.0909183076582849, 0.9204891498666257, 92_u64),
    (879_u64, 0.7985966380219907, 0.4784688870422542, 701_u64),
    (899_u64, 0.3597852496895939, 0.26652057841420174, 314_u64),
    (297_u64, 0.040487576741725206, 0.8565107162576169, 16_u64),
    (601_u64, 0.041086339857429266, 0.229146477766335, 21_u64),
    (283_u64, 0.9544342383742332, 0.7919468733016402, 273_u64),
    (932_u64, 0.3733412448782474, 0.6467748603317887, 353_u64),
    (997_u64, 0.8064196675550193, 0.4243346906732768, 802_u64),
    (991_u64, 0.9100590089801699, 0.09506826801225543, 890_u64),
    (621_u64, 0.44007621286436915, 0.0034677039366215467, 240_u64),
    (517_u64, 0.5763365030288696, 0.5311336689628661, 299_u64),
    (212_u64, 0.07366779516451061, 0.5243071159347892, 16_u64),
    (930_u64, 0.16462739440612495, 0.21318555460311472, 144_u64),
    (860_u64, 0.7398907758761197, 0.7169320799876004, 644_u64),
    (872_u64, 0.475711012724787, 0.9613435762003064, 441_u64),
    (259_u64, 0.6855209532659501, 0.518266599625349, 178_u64),
    (314_u64, 0.9515149428043514, 0.1745280222967267, 295_u64),
    (481_u64, 0.49746448756195605, 0.5625401348806918, 241_u64),
    (298_u64, 0.4705006266012788, 0.7592581696808338, 146_u64),
    (24_u64, 0.5601919500622898, 0.6669713375158608, 15_u64),
    (158_u64, 0.6525101205334067, 0.22487292322330177, 99_u64),
    (299_u64, 0.2795734987594187, 0.34584976243786514, 80_u64),
    (911_u64, 0.9799075894989073, 0.3198317599017173, 891_u64),
    (406_u64, 0.6438641080167145, 0.9048983976244926, 274_u64),
    (314_u64, 0.5825784357730299, 0.199198380112648, 176_u64),
    (648_u64, 0.6158710259478539, 0.6809630165807903, 405_u64),
    (292_u64, 0.9251402944792062, 0.13751775864511728, 265_u64),
    (836_u64, 0.3900228952988982, 0.10699468688108027, 309_u64),
    (982_u64, 0.2879196850117296, 0.09285939577966928, 264_u64),
    (146_u64, 0.09073596470989287, 0.9164489551912993, 18_u64),
    (621_u64, 0.3220339040271938, 0.2770604358520359, 193_u64),
    (348_u64, 0.7582701125647873, 0.885793877299875, 273_u64),
    (197_u64, 0.10441292962059379, 0.7728646495379508, 24_u64),
    (516_u64, 0.710277852602303, 0.7950512361712754, 375_u64),
    (226_u64, 0.9664773836266249, 0.20567359309643507, 216_u64),
    (504_u64, 0.20149122620932758, 0.04819332296028733, 87_u64),
    (946_u64, 0.10848865448497236, 0.038815931184217334, 86_u64),
    (355_u64, 0.05540217878296971, 0.28457412985153496, 17_u64),
    (984_u64, 0.8297235150821507, 0.3488098324742168, 812_u64),
    (727_u64, 0.5811977565754205, 0.7374533254187554, 431_u64),
    (945_u64, 0.47009237529709935, 0.2516635812353343, 434_u64),
    (326554_u64, 0.36501412000507116, 0.5174370030872524, 119209_u64),
    (102662_u64, 0.2801246272865683, 0.7594447226729244, 28859_u64),
    (340098_u64, 0.5997158549726009, 0.6360845279414207, 204062_u64),
    (250883_u64, 0.8185696129221469, 0.20394068537279963, 205205_u64),
    (343445_u64, 0.09783228044398129, 0.9930452820844948, 34029_u64),
    (74743_u64, 0.9636895260773599, 0.000405043363571167, 71856_u64),
    (359098_u64, 0.16873644338920712, 0.20657002506777644, 60409_u64),
    (636968_u64, 0.08608340588398278, 0.6340280906297266, 54909_u64),
    (286753_u64, 0.86121069570072, 0.017291825264692307, 246563_u64),
    (103_u64, 0.5247906020376831, 0.02673547132872045, 44_u64),
    (206957_u64, 0.6568108752835542, 0.60784059506841, 135991_u64),
    (632137_u64, 0.22951937280595303, 0.570541352732107, 145147_u64),
    (823372_u64, 0.7212260314263403, 0.24585334793664515, 593558_u64),
    (202505_u64, 0.490750394994393, 0.9195250864140689, 99695_u64),
    (675363_u64, 0.9652555924840271, 0.6722162016667426, 651965_u64),
    (786448_u64, 0.9069424804765731, 0.6454504393041134, 713359_u64),
    (323292_u64, 0.5512505278456956, 0.20828545466065407, 177985_u64),
    (506862_u64, 0.07559909950941801, 0.48061773576773703, 38309_u64),
    (295900_u64, 0.022700011264532804, 0.9865185832604766, 6897_u64),
    (368258_u64, 0.5132395278196782, 0.9931198477279395, 189752_u64),
    (79768_u64, 0.6307261516340077, 0.4507740365806967, 50295_u64),
    (25490_u64, 0.41877162433229387, 0.7148487861268222, 10719_u64),
    (325230_u64, 0.8792659451719373, 0.3165616125334054, 285875_u64),
    (506807_u64, 0.10798707301728427, 0.8336875606328249, 54943_u64),
    (458901_u64, 0.9802786963991821, 0.43397558154538274, 449835_u64),
    (251449_u64, 0.264966631308198, 0.9959922581911087, 67213_u64),
    (226279_u64, 0.08427752321586013, 0.8058112133294344, 19184_u64),
    (625042_u64, 0.38590717571787536, 0.48624216741882265, 241195_u64),
    (678669_u64, 0.12489582714624703, 0.5387720009312034, 84789_u64),
    (316470_u64, 0.5815542216878384, 0.34103990835137665, 183931_u64),
    (157984_u64, 0.2401496081147343, 0.052015604451298714, 37664_u64),
    (746781_u64, 0.7218878886196762, 0.4587231131736189, 539052_u64),
    (63084_u64, 0.14592869067564607, 0.49815659038722515, 9205_u64),
    (571769_u64, 0.15283877030014992, 0.28276422526687384, 87232_u64),
    (301694_u64, 0.25922267022542655, 0.7764450823888183, 78389_u64),
    (716793_u64, 0.7778862600680441, 0.3038527618627995, 557403_u64),
    (627824_u64, 0.4264662996865809, 0.5155511819757521, 267761_u64),
    (466205_u64, 0.060048336163163185, 0.47795077320188284, 27986_u64),
    (434086_u64, 0.11483253701590002, 0.768852224573493, 50002_u64),
    (5261_u64, 0.48275689734146, 0.16392885567620397, 2504_u64),
    (903045_u64, 0.9791735839098692, 0.44177414989098907, 884218_u64),
    (577847_u64, 0.8115167869254947, 0.5234076697379351, 468950_u64),
    (665130_u64, 0.542912817094475, 0.47691918862983584, 361084_u64),
    (611064_u64, 0.07236708700656891, 0.5246432877611369, 44233_u64),
    (25851_u64, 0.4664852460846305, 0.6852282471954823, 12098_u64),
    (575133_u64, 0.3399056459311396, 0.35179202212020755, 195354_u64),
    (526195_u64, 0.689918605145067, 0.8396312852855772, 363365_u64),
    (962080_u64, 0.5141573720611632, 0.9219136915635318, 495356_u64),
    (36078_u64, 0.5149230156093836, 0.1063096213620156, 18459_u64),
    (864879_u64, 0.5455143540166318, 0.20652891811914742, 471425_u64),
    (339448_u64, 0.44745733006857336, 0.4866429117973894, 151879_u64),
    (938283_u64, 0.08388484036549926, 0.32382202381268144, 78585_u64),
    (864165_u64, 0.9301336917560548, 0.47171392501331866, 803772_u64),
    (8946_u64, 0.016448186244815588, 0.5860885020811111, 150_u64),
    (956322_u64, 0.4140923994127661, 0.22395001258701086, 395640_u64),
    (902368_u64, 0.22697609337046742, 0.7340297200717032, 205065_u64),
    (384214_u64, 0.09964058734476566, 0.15048409905284643, 38091_u64),
    (344671_u64, 0.4829238750971854, 0.6975533668883145, 166602_u64),
    (376660_u64, 0.6501286714337766, 0.2641079048626125, 244693_u64),
    (772879_u64, 0.9213297783862799, 0.42646492226049304, 712033_u64),
    (572606890_u64, 0.36260180710814893, 0.721333573339507, 207635044_u64),
    (836159676_u64, 0.8551349900662899, 0.474471707129851, 715028745_u64),
    (553459980_u64, 0.30090615153312683, 0.3385869609192014, 166535020_u64),
    (310316442_u64, 0.4656624251510948, 0.24059213884174824, 144496517_u64),
    (136846529_u64, 0.1427306649275124, 0.21625967789441347, 19528984_u64),
    (580355170_u64, 0.8077189936302602, 0.2420581190381199, 468757251_u64),
    (437809500_u64, 0.6658076317980886, 0.564904673025012, 291498519_u64),
    (678064445_u64, 0.06194097991101444, 0.7125680889002979, 42003497_u64),
    (935164942_u64, 0.43092556833289564, 0.10809659794904292, 402967756_u64),
    (10568316_u64, 0.39685508143156767, 0.5190758735407144, 4194166_u64),
    (180463199_u64, 0.6969567514024675, 0.852190344594419, 125781502_u64),
    (371134363_u64, 0.6593196506146342, 0.7146319421008229, 244701355_u64),
    (864671417_u64, 0.4073507096618414, 0.31398168485611677, 352217514_u64),
    (320559734_u64, 0.3069202203769237, 0.1158171424176544, 98376387_u64),
    (203744394_u64, 0.25510734505951405, 0.5750040675047785, 51977868_u64),
    (184874531_u64, 0.6725681533571333, 0.8313719220459461, 124346845_u64),
    (273535502_u64, 0.8943934335839003, 0.5203397579025477, 244648616_u64),
    (245051258_u64, 0.8457361562177539, 0.6940110286232084, 207251577_u64),
    (207634149_u64, 0.3929018569178879, 0.4999039894901216, 81579841_u64),
    (434251958_u64, 0.07905053999274969, 0.3541603987105191, 34325748_u64),
    (232326271_u64, 0.8284230569843203, 0.9648041590116918, 192474837_u64),
    (371908276_u64, 0.07289182487875223, 0.9222612541634589, 27116194_u64),
    (234449888_u64, 0.11476267757825553, 0.20459270663559437, 26902069_u64),
    (290492347_u64, 0.6399842700920999, 0.9549029327463359, 185924394_u64),
    (303707678_u64, 0.32056620018556714, 0.873992832377553, 97367733_u64),
    (213089124_u64, 0.1887495261617005, 0.697344831423834, 40223423_u64),
    (684926539_u64, 0.3938296411652118, 0.5125937848351896, 269744777_u64),
    (361771070_u64, 0.8620260159950703, 0.4808165153954178, 311855759_u64),
    (84707295_u64, 0.3479114065412432, 0.20180092309601605, 29466973_u64),
    (92130428_u64, 0.0014338984619826078, 0.10022935434244573, 131641_u64),
    (473860657_u64, 0.9112844537012279, 0.47258887463249266, 431821424_u64),
    (305962612_u64, 0.9517234489321709, 0.312179648084566, 291189956_u64),
    (922754961_u64, 0.4909189830068499, 0.772399305133149, 453009267_u64),
    (262205980_u64, 0.4636517153121531, 0.901074001332745, 121582650_u64),
    (74399_u64, 0.596472046803683, 0.33522928413003683, 44320_u64),
    (512177279_u64, 0.9060509572736919, 0.8239442463964224, 464064858_u64),
    (751821571_u64, 0.1730011790059507, 0.22003285982646048, 130058011_u64),
    (585419021_u64, 0.7858810767065734, 0.7661237383726984, 460076938_u64),
    (873145025_u64, 0.23293438460677862, 0.5607078510802239, 203387407_u64),
    (580736563_u64, 0.5770482090301812, 0.4234555377624929, 335110695_u64),
    (297494367_u64, 0.8408770333044231, 0.838569788262248, 250162418_u64),
    (983850543_u64, 0.13220377685502172, 0.28732058755122125, 130062795_u64),
    (926354630_u64, 0.8958911895751953, 0.4707207006867975, 829912269_u64),
    (804715408_u64, 0.45013734116218984, 0.7808445333503187, 362243392_u64),
    (336145351_u64, 0.894142537843436, 0.0827168372925371, 300554033_u64),
    (384305049_u64, 0.24854518054053187, 0.3945175502449274, 95514901_u64),
    (503495506_u64, 0.08369529363699257, 0.09566140733659267, 42132085_u64),
    (280257263_u64, 0.04864107468165457, 0.1333332359790802, 13628014_u64),
    (354096892_u64, 0.9798158670309931, 0.22112617804668844, 346947720_u64),
    (30317149_u64, 0.48416774114593863, 0.2741195410490036, 14676933_u64),
    (624429140_u64, 0.8453930339310318, 0.2990592496935278, 527883283_u64),
    (259347416_u64, 0.4162936045322567, 0.37478677253238857, 107962137_u64),
    (551611965_u64, 0.48934254283085465, 0.4914524925407022, 269926950_u64),
    (610609323_u64, 0.18328781751915812, 0.8786026234738529, 111928417_u64),
    (300437142_u64, 0.7591614678967744, 0.7487298287451267, 228085271_u64),
    (463076022_u64, 0.3051433025393635, 0.9551860231440514, 141321366_u64),
    (909569590_u64, 0.16567824711091816, 0.33192021306604147, 150691022_u64),
    (201238330_u64, 0.03280914295464754, 0.8241627963725477, 6604811_u64),
    (648871333_u64, 0.13650520821101964, 0.13266538339667022, 88564575_u64),
    (774530255_u64, 0.17713641142472625, 0.12469685380347073, 137185272_u64),
    (1_u64, 0.5195604504551739, 0.7281097117811441, 1_u64),
    (2_u64, 0.8111207850743085, 0.5484208285342902, 2_u64),
    (10_u64, 0.11536201252602041, 0.6085174127947539, 1_u64),
    (1000_u64, 0.8934217866044492, 0.1229467005468905, 882_u64),
    (1000000_u64, 0.575352881103754, 0.8764346989337355, 575925_u64),
    (1000000000_u64, 0.1465723942965269, 0.8423488156404346, 146583625_u64),
    (1000000000_u64, 0.5, 1e-12, 499888775_u64),
    (1000000000_u64, 1e-09, 0.999999999999, 14_u64),
    (100_u64, 0.5, 0.9999999, 75_u64),
    (50_u64, 0.9, 1e-09, 29_u64),
    (1000000_u64, 1e-06, 0.999999999999999, 16_u64),
    (1000000000_u64, 0.7, 0.5, 700000000_u64),
    (2_u64, 0.4, 0.1, 0_u64),
    (3_u64, 0.6, 0.9, 3_u64),
    (5_u64, 0.2, 0.99, 3_u64),
    (10000_u64, 0.8, 1e-12, 7714_u64),
    (1000000000_u64, 1e-12, 0.999999, 1_u64),
    (1000000000_u64, 0.999999999999999, 0.999999999999, 1000000000_u64),
    (1000000_u64, 0.999999999999, 0.75, 1000000_u64),
    (1000000000_u64, 1e-15, 1e-10, 0_u64),
    (10_u64, 0.5, 0.5, 5_u64),
    (1000000000_u64, 0.999999999, 0.999999999, 1000000000_u64),
    (1000000000_u64, 1e-10, 0.5, 0_u64),
    (100000_u64, 0.9999999999, 0.99999999, 100000_u64),
    (10000000_u64, 0.123456789, 0.4321, 1234390_u64),
    (100000000_u64, 0.0001, 1e-10, 9371_u64),
    (1000000000_u64, 0.99999999, 0.99999999999999, 1000000000_u64),
    (1_u64, 0.5, 1.0, 1_u64),
    (1000000000_u64, 0.999999999999, 0.9999, 1000000000_u64),
    (1000000_u64, 1e-12, 1e-08, 0_u64),
];

#[test]
fn binomial_quantile_matches_r_random() {
    for (n, prob, p, expected) in QBINOM_R_REF {
        assert_eq!(
            binomial_quantile(p, n, prob),
            expected,
            "binomial_quantile({p}, {n}, {prob})"
        );
    }
}

/// `pt`/`pt(lower.tail = FALSE)` at the review's audit df — 1e8
/// through 1e300, ordinary and tail `t`. Command:
///
///   Rscript -e 'dfs <- c(1e8, 1e10, 1e12, 1e15, 1e20, 1e300)
///   ts <- c(-5, -3.0902323061678136, -1.6448536269514722, -0.6744897501960817, 0,
///           0.6744897501960817, 1.6448536269514722, 3.0902323061678136, 5)
///   for (df in dfs) for (t in ts)
///     cat(sprintf("%.17g, %.17g, %.17g, %.17g\n", t, df, pt(t, df), pt(t, df, lower.tail=FALSE)))'
#[rustfmt::skip]
const T_LARGE_DF_REF: [(f64, f64, f64, f64); 54] = [
    (-5.0, 100000000.0, 2.8665205506336334e-07, 0.999999713347945),
    (-3.0902323061678136, 100000000.0, 0.001000000274422205, 0.9989999997255778),
    (-1.6448536269514722, 100000000.0, 0.05000000157154914, 0.9499999984284508),
    (-0.6744897501960817, 100000000.0, 0.25000000077961704, 0.749999999220383),
    (0.0, 100000000.0, 0.5, 0.5),
    (0.6744897501960817, 100000000.0, 0.749999999220383, 0.25000000077961704),
    (1.6448536269514722, 100000000.0, 0.9499999984284508, 0.05000000157154914),
    (3.0902323061678136, 100000000.0, 0.9989999997255778, 0.001000000274422205),
    (5.0, 100000000.0, 0.999999713347945, 2.8665205506336334e-07),
    (-5.0, 10000000000.0, 2.8665157671103195e-07, 0.9999997133484233),
    (-3.0902323061678136, 10000000000.0, 0.001000000002744219, 0.9989999999972559),
    (-1.6448536269514722, 10000000000.0, 0.05000000001571548, 0.9499999999842845),
    (-0.6744897501960817, 10000000000.0, 0.2500000000077963, 0.7499999999922037),
    (0.0, 10000000000.0, 0.5, 0.5),
    (0.6744897501960817, 10000000000.0, 0.7499999999922037, 0.2500000000077963),
    (1.6448536269514722, 10000000000.0, 0.9499999999842845, 0.05000000001571548),
    (3.0902323061678136, 10000000000.0, 0.9989999999972559, 0.001000000002744219),
    (5.0, 10000000000.0, 0.9999997133484233, 2.8665157671103195e-07),
    (-5.0, 1000000000000.0, 2.866515719275118e-07, 0.9999997133484281),
    (-3.0902323061678136, 1000000000000.0, 0.0010000000000274412, 0.9989999999999726),
    (-1.6448536269514722, 1000000000000.0, 0.05000000000015725, 0.9499999999998427),
    (-0.6744897501960817, 1000000000000.0, 0.25000000000007827, 0.7499999999999217),
    (0.0, 1000000000000.0, 0.5, 0.5),
    (0.6744897501960817, 1000000000000.0, 0.7499999999999217, 0.25000000000007827),
    (1.6448536269514722, 1000000000000.0, 0.9499999999998427, 0.05000000000015725),
    (3.0902323061678136, 1000000000000.0, 0.9989999999999726, 0.0010000000000274412),
    (5.0, 1000000000000.0, 0.9999997133484281, 2.866515719275118e-07),
    (-5.0, 1000000000000000.0, 2.866515718792415e-07, 0.9999997133484282),
    (-3.0902323061678136, 1000000000000000.0, 0.0010000000000000258, 0.999),
    (-1.6448536269514722, 1000000000000000.0, 0.050000000000000246, 0.9499999999999997),
    (-0.6744897501960817, 1000000000000000.0, 0.24999999999999972, 0.7500000000000002),
    (0.0, 1000000000000000.0, 0.5, 0.5),
    (0.6744897501960817, 1000000000000000.0, 0.7500000000000002, 0.24999999999999972),
    (1.6448536269514722, 1000000000000000.0, 0.9499999999999997, 0.050000000000000246),
    (3.0902323061678136, 1000000000000000.0, 0.999, 0.0010000000000000258),
    (5.0, 1000000000000000.0, 0.9999997133484282, 2.866515718792415e-07),
    (-5.0, 1e+20, 2.8665157187919306e-07, 0.9999997133484282),
    (-3.0902323061678136, 1e+20, 0.0009999999999999987, 0.999),
    (-1.6448536269514722, 1e+20, 0.05000000000000005, 0.95),
    (-0.6744897501960817, 1e+20, 0.24999999999999978, 0.7500000000000002),
    (0.0, 1e+20, 0.5, 0.5),
    (0.6744897501960817, 1e+20, 0.7500000000000002, 0.24999999999999978),
    (1.6448536269514722, 1e+20, 0.95, 0.05000000000000005),
    (3.0902323061678136, 1e+20, 0.999, 0.0009999999999999987),
    (5.0, 1e+20, 0.9999997133484282, 2.8665157187919306e-07),
    (-5.0, 1e+300, 2.866515718791936e-07, 0.9999997133484282),
    (-3.0902323061678136, 1e+300, 0.0009999999999999987, 0.999),
    (-1.6448536269514722, 1e+300, 0.05000000000000005, 0.95),
    (-0.6744897501960817, 1e+300, 0.25000000000000067, 0.7499999999999993),
    (0.0, 1e+300, 0.5, 0.5),
    (0.6744897501960817, 1e+300, 0.7499999999999993, 0.25000000000000067),
    (1.6448536269514722, 1e+300, 0.95, 0.05000000000000005),
    (3.0902323061678136, 1e+300, 0.999, 0.0009999999999999987),
    (5.0, 1e+300, 0.9999997133484282, 2.866515718791936e-07),
];

/// `qt` at the same audit df — ordinary and tail `p`. Command:
///
///   Rscript -e 'dfs <- c(1e8, 1e10, 1e12, 1e15, 1e20, 1e300)
///   ps <- c(1e-12, 1e-8, 1e-4, 0.001, 0.01, 0.25, 0.5, 0.75, 0.99, 0.999, 0.9999, 1-1e-8, 1-1e-12)
///   for (df in dfs) for (p in ps) cat(sprintf("%.17g, %.17g, %.17g\n", p, df, qt(p, df)))'
#[rustfmt::skip]
const T_QUANTILE_LARGE_DF_REF: [(f64, f64, f64); 78] = [
    (1e-12, 100000000.0, -7.034484713122775),
    (1e-08, 100000000.0, -5.612001700073571),
    (0.0001, 100000000.0, -3.7190166233482964),
    (0.001, 100000000.0, -3.0902323876691056),
    (0.01, 100000000.0, -2.326347911331584),
    (0.25, 100000000.0, -0.674489752649431),
    (0.5, 100000000.0, 0.0),
    (0.75, 100000000.0, 0.674489752649431),
    (0.99, 100000000.0, 2.326347911331584),
    (0.999, 100000000.0, 3.0902323876691056),
    (0.9999, 100000000.0, 3.719016623348325),
    (0.99999999, 100000000.0, 5.612001699204287),
    (0.999999999999, 100000000.0, 7.034487797870631),
    (1e-12, 10000000000.0, -7.034483834179348),
    (1e-08, 10000000000.0, -5.612001248733778),
    (0.0001, 10000000000.0, -3.7190164868346063),
    (0.001, 10000000000.0, -3.090232306982826),
    (0.01, 10000000000.0, -2.3263478744137482),
    (0.25, 10000000000.0, -0.6744897502206152),
    (0.5, 10000000000.0, 0.0),
    (0.75, 10000000000.0, 0.6744897502206152),
    (0.99, 10000000000.0, 2.3263478744137482),
    (0.999, 10000000000.0, 3.090232306982826),
    (0.9999, 10000000000.0, 3.7190164868346347),
    (0.99999999, 10000000000.0, 5.612001247864494),
    (0.999999999999, 10000000000.0, 7.034486918926063),
    (1e-12, 1000000000000.0, -7.0344838253899145),
    (1e-08, 1000000000000.0, -5.612001244220379),
    (0.0001, 1000000000000.0, -3.7190164854694694),
    (0.001, 1000000000000.0, -3.090232306175963),
    (0.01, 1000000000000.0, -2.3263478740445698),
    (0.25, 1000000000000.0, -0.6744897501963271),
    (0.5, 1000000000000.0, 0.0),
    (0.75, 1000000000000.0, 0.6744897501963271),
    (0.99, 1000000000000.0, 2.3263478740445698),
    (0.999, 1000000000000.0, 3.090232306175963),
    (0.9999, 1000000000000.0, 3.719016485469498),
    (0.99999999, 1000000000000.0, 5.612001243351095),
    (0.999999999999, 1000000000000.0, 7.034486910136618),
    (1e-12, 1000000000000000.0, -7.034483825301221),
    (1e-08, 1000000000000000.0, -5.612001244174836),
    (0.0001, 1000000000000000.0, -3.7190164854556937),
    (0.001, 1000000000000000.0, -3.0902323061678216),
    (0.01, 1000000000000000.0, -2.3263478740408443),
    (0.25, 1000000000000000.0, -0.6744897501960819),
    (0.5, 1000000000000000.0, 0.0),
    (0.75, 1000000000000000.0, 0.6744897501960819),
    (0.99, 1000000000000000.0, 2.3263478740408443),
    (0.999, 1000000000000000.0, 3.0902323061678216),
    (0.9999, 1000000000000000.0, 3.719016485455722),
    (0.99999999, 1000000000000000.0, 5.612001243305551),
    (0.999999999999, 1000000000000000.0, 7.034486910047924),
    (1e-12, 1e+20, -7.034483825301132),
    (1e-08, 1e+20, -5.61200124417479),
    (0.0001, 1e+20, -3.71901648545568),
    (0.001, 1e+20, -3.090232306167813),
    (0.01, 1e+20, -2.3263478740408408),
    (0.25, 1e+20, -0.6744897501960817),
    (0.5, 1e+20, 0.0),
    (0.75, 1e+20, 0.6744897501960817),
    (0.99, 1e+20, 2.3263478740408408),
    (0.999, 1e+20, 3.090232306167813),
    (0.9999, 1e+20, 3.7190164854557084),
    (0.99999999, 1e+20, 5.612001243305506),
    (0.999999999999, 1e+20, 7.0344869100478356),
    (1e-12, 1e+300, -7.034483825301132),
    (1e-08, 1e+300, -5.61200124417479),
    (0.0001, 1e+300, -3.71901648545568),
    (0.001, 1e+300, -3.090232306167813),
    (0.01, 1e+300, -2.3263478740408408),
    (0.25, 1e+300, -0.6744897501960817),
    (0.5, 1e+300, 0.0),
    (0.75, 1e+300, 0.6744897501960817),
    (0.99, 1e+300, 2.3263478740408408),
    (0.999, 1e+300, 3.090232306167813),
    (0.9999, 1e+300, 3.7190164854557084),
    (0.99999999, 1e+300, 5.612001243305506),
    (0.999999999999, 1e+300, 7.0344869100478356),
];

#[test]
fn t_large_df_matches_reference() {
    for (t, df, e_cdf, e_sf) in T_LARGE_DF_REF {
        ref_check(e_cdf, t_cdf(t, df), 1e-9, format!("t_cdf({t}, {df})"));
        ref_check(e_sf, t_sf(t, df), 1e-9, format!("t_sf({t}, {df})"));
    }
    for (p, df, e_q) in T_QUANTILE_LARGE_DF_REF {
        let actual = t_quantile(p, df).expect("t_quantile in domain");
        ref_check(e_q, actual, 1e-9, format!("t_quantile({p}, {df})"));
    }
}

/// `pf(x, df1, df2, lower.tail = FALSE)` at the review's audit df
/// pairs — symmetric, asymmetric and both-huge, `x` on both sides of
/// 1 so the beta argument never sits at a rounding boundary. Command:
///
///   Rscript -e 'fp <- list(c(100000000.0, 100000000.0), c(10000000000.0, 1000000000000.0), c(1000000000000.0, 10000000000.0), c(1000000000000000.0, 100000000.0),
///                         c(1e+20, 1e+20), c(1e+300, 1e+300), c(100000000.0, 1e+300), c(1e300,1e8))
///   xs <- c(0.01, 0.1, 0.5, 0.9, 0.99, 0.9999, 1, 1.0001, 1.01, 1.1, 2, 10, 100)
///   for (d in fp) for (x in xs)
///     cat(sprintf("%.17g, %.17g, %.17g, %.17g\n", x, d[1], d[2], pf(x, d[1], d[2], lower.tail=FALSE)))'
#[rustfmt::skip]
const F_SF_LARGE_DF_REF: [(f64, f64, f64, f64); 104] = [
    (0.01, 100000000.0, 100000000.0, 1.0),
    (0.1, 100000000.0, 100000000.0, 1.0),
    (0.5, 100000000.0, 100000000.0, 1.0),
    (0.9, 100000000.0, 100000000.0, 1.0),
    (0.99, 100000000.0, 100000000.0, 1.0),
    (0.9999, 100000000.0, 100000000.0, 0.6914712629622579),
    (1.0, 100000000.0, 100000000.0, 0.5),
    (1.0001, 100000000.0, 100000000.0, 0.30854634030406514),
    (1.01, 100000000.0, 100000000.0, 0.0),
    (1.1, 100000000.0, 100000000.0, 0.0),
    (2.0, 100000000.0, 100000000.0, 0.0),
    (10.0, 100000000.0, 100000000.0, 0.0),
    (100.0, 100000000.0, 100000000.0, 0.0),
    (0.01, 10000000000.0, 1000000000000.0, 1.0),
    (0.1, 10000000000.0, 1000000000000.0, 1.0),
    (0.5, 10000000000.0, 1000000000000.0, 1.0),
    (0.9, 10000000000.0, 1000000000000.0, 1.0),
    (0.99, 10000000000.0, 1000000000000.0, 1.0),
    (0.9999, 10000000000.0, 1000000000000.0, 0.9999999999990123),
    (1.0, 10000000000.0, 1000000000000.0, 0.49999814741426074),
    (1.0001, 10000000000.0, 1000000000000.0, 9.910080502751437e-13),
    (1.01, 10000000000.0, 1000000000000.0, 0.0),
    (1.1, 10000000000.0, 1000000000000.0, 0.0),
    (2.0, 10000000000.0, 1000000000000.0, 0.0),
    (10.0, 10000000000.0, 1000000000000.0, 0.0),
    (100.0, 10000000000.0, 1000000000000.0, 0.0),
    (0.01, 1000000000000.0, 10000000000.0, 1.0),
    (0.1, 1000000000000.0, 10000000000.0, 1.0),
    (0.5, 1000000000000.0, 10000000000.0, 1.0),
    (0.9, 1000000000000.0, 10000000000.0, 1.0),
    (0.99, 1000000000000.0, 10000000000.0, 1.0),
    (0.9999, 1000000000000.0, 10000000000.0, 0.9999999999990139),
    (1.0, 1000000000000.0, 10000000000.0, 0.5000018525857393),
    (1.0001, 1000000000000.0, 10000000000.0, 9.927094135703951e-13),
    (1.01, 1000000000000.0, 10000000000.0, 0.0),
    (1.1, 1000000000000.0, 10000000000.0, 0.0),
    (2.0, 1000000000000.0, 10000000000.0, 0.0),
    (10.0, 1000000000000.0, 10000000000.0, 0.0),
    (100.0, 1000000000000.0, 10000000000.0, 0.0),
    (0.01, 1000000000000000.0, 100000000.0, 1.0),
    (0.1, 1000000000000000.0, 100000000.0, 1.0),
    (0.5, 1000000000000000.0, 100000000.0, 1.0),
    (0.9, 1000000000000000.0, 100000000.0, 1.0),
    (0.99, 1000000000000000.0, 100000000.0, 1.0),
    (0.9999, 1000000000000000.0, 100000000.0, 0.7602792207327131),
    (1.0, 1000000000000000.0, 100000000.0, 0.5000188063166328),
    (1.0001, 1000000000000000.0, 100000000.0, 0.23977936476686695),
    (1.01, 1000000000000000.0, 100000000.0, 0.0),
    (1.1, 1000000000000000.0, 100000000.0, 0.0),
    (2.0, 1000000000000000.0, 100000000.0, 0.0),
    (10.0, 1000000000000000.0, 100000000.0, 0.0),
    (100.0, 1000000000000000.0, 100000000.0, 0.0),
    (0.01, 1e+20, 1e+20, 1.0),
    (0.1, 1e+20, 1e+20, 1.0),
    (0.5, 1e+20, 1e+20, 1.0),
    (0.9, 1e+20, 1e+20, 1.0),
    (0.99, 1e+20, 1e+20, 1.0),
    (0.9999, 1e+20, 1e+20, 1.0),
    (1.0, 1e+20, 1e+20, 0.5),
    (1.0001, 1e+20, 1e+20, 0.0),
    (1.01, 1e+20, 1e+20, 0.0),
    (1.1, 1e+20, 1e+20, 0.0),
    (2.0, 1e+20, 1e+20, 0.0),
    (10.0, 1e+20, 1e+20, 0.0),
    (100.0, 1e+20, 1e+20, 0.0),
    (0.01, 1e+300, 1e+300, 1.0),
    (0.1, 1e+300, 1e+300, 1.0),
    (0.5, 1e+300, 1e+300, 1.0),
    (0.9, 1e+300, 1e+300, 1.0),
    (0.99, 1e+300, 1e+300, 1.0),
    (0.9999, 1e+300, 1e+300, 1.0),
    (1.0, 1e+300, 1e+300, 0.5),
    (1.0001, 1e+300, 1e+300, 0.0),
    (1.01, 1e+300, 1e+300, 0.0),
    (1.1, 1e+300, 1e+300, 0.0),
    (2.0, 1e+300, 1e+300, 0.0),
    (10.0, 1e+300, 1e+300, 0.0),
    (100.0, 1e+300, 1e+300, 0.0),
    (0.01, 100000000.0, 1e+300, 1.0),
    (0.1, 100000000.0, 1e+300, 1.0),
    (0.5, 100000000.0, 1e+300, 1.0),
    (0.9, 100000000.0, 1e+300, 1.0),
    (0.99, 100000000.0, 1e+300, 1.0),
    (0.9999, 100000000.0, 1e+300, 0.7602426159624958),
    (1.0, 100000000.0, 1e+300, 0.4999811936805463),
    (1.0001, 100000000.0, 1e+300, 0.23974273766123683),
    (1.01, 100000000.0, 1e+300, 0.0),
    (1.1, 100000000.0, 1e+300, 0.0),
    (2.0, 100000000.0, 1e+300, 0.0),
    (10.0, 100000000.0, 1e+300, 0.0),
    (100.0, 100000000.0, 1e+300, 0.0),
    (0.01, 1e+300, 100000000.0, 1.0),
    (0.1, 1e+300, 100000000.0, 1.0),
    (0.5, 1e+300, 100000000.0, 1.0),
    (0.9, 1e+300, 100000000.0, 1.0),
    (0.99, 1e+300, 100000000.0, 1.0),
    (0.9999, 1e+300, 100000000.0, 0.7602792317203317),
    (1.0, 1e+300, 100000000.0, 0.5000188063194537),
    (1.0001, 1e+300, 100000000.0, 0.23977935378515805),
    (1.01, 1e+300, 100000000.0, 0.0),
    (1.1, 1e+300, 100000000.0, 0.0),
    (2.0, 1e+300, 100000000.0, 0.0),
    (10.0, 1e+300, 100000000.0, 0.0),
    (100.0, 1e+300, 100000000.0, 0.0),
];

#[test]
fn f_sf_large_df_matches_reference() {
    for (x, df1, df2, e_sf) in F_SF_LARGE_DF_REF {
        ref_check(
            e_sf,
            f_sf(x, df1, df2),
            1e-9,
            format!("f_sf({x}, {df1}, {df2})"),
        );
    }
}

/// `qgamma`/`qchisq` at `p` within 1e-16 of 1 — the `P(a, y) − p`
/// cancellation the upper-tail solve removes. Command:
///
///   Rscript -e 'ps <- c(1-1e-8, 1-1e-12, 1-1e-15, 1-1e-16)
///   for (s in c(0.5, 2.5, 50)) for (p in ps) cat(sprintf("%.17g, %.17g, %.17g\n", p, s, qgamma(p, s, 1)))
///   for (df in c(1, 10)) for (p in ps) cat(sprintf("%.17g, %.17g, %.17g\n", p, df, qchisq(p, df)))'
#[rustfmt::skip]
const GAMMA_QUANTILE_UPPER_REF: [(f64, f64, f64); 20] = [
    (0.99999999, 0.5, 16.42062667573443),
    (0.999999999999, 0.5, 25.42208566622403),
    (0.999999999999999, 0.5, 32.21601948468177),
    (0.9999999999999999, 0.5, 34.38162610583421),
    (0.99999999, 2.5, 22.897293556181317),
    (0.999999999999, 2.5, 32.61934126118208),
    (0.999999999999999, 2.5, 39.81883170126342),
    (0.9999999999999999, 2.5, 42.09751611826066),
    (0.99999999, 50.0, 100.31595435875327),
    (0.999999999999, 50.0, 116.9053216800022),
    (0.999999999999999, 50.0, 128.31877077443156),
    (0.9999999999999999, 50.0, 131.81903785915443),
    (0.99999999, 1.0, 32.84125335146886),
    (0.999999999999, 1.0, 50.84417133244806),
    (0.999999999999999, 1.0, 64.43203896936353),
    (0.9999999999999999, 1.0, 68.76325221166842),
    (0.99999999, 10.0, 57.66396161460047),
    (0.999999999999, 10.0, 78.47169567822768),
    (0.999999999999999, 10.0, 93.67053652165554),
    (0.9999999999999999, 10.0, 98.45486389167608),
];

#[test]
fn gamma_quantile_upper_tail_matches_reference() {
    for (i, (p, a, e)) in GAMMA_QUANTILE_UPPER_REF.iter().enumerate() {
        let actual = if i < 12 {
            gamma_quantile(*p, *a, 1.0)
        } else {
            chisq_quantile(*p, *a)
        };
        let actual = actual.unwrap_or_else(|e| panic!("upper-tail quantile({p}, {a}): {e:?}"));
        ref_check(*e, actual, 1e-6, format!("upper-tail quantile({p}, {a})"));
    }
}

/// `pbeta` reference points across the TOMS 708 dispatch — `min ≤ 1`
/// mixed paths, `b0 < 40` paths, `bfrac`/`basym` and the `x ≈ 1`
/// corners that broke the plain continued fraction. Command:
///
///   Rscript -e 'args <- rbind(c(0.5, 0.5, 0.5), c(0.1, 0.3, 0.7), c(0.99999999, 10.0, 0.1),
///     c(0.99999999, 100.0, 0.1), c(0.99999999, 1000.0, 0.1), c(0.99999999, 100000000.0, 0.1),
///     c(0.99999999, 1000000000000.0, 0.1), c(0.001, 0.1, 100.0), c(0.001, 0.1, 1000000.0), c(0.25, 100.0, 0.5),
///     c(0.75, 0.5, 100.0), c(0.9999, 200000.0, 0.5), c(0.5, 100000000.0, 100000000.0), c(0.0001, 1000000.0, 0.1),
///     c(0.9999, 5.0, 40.0), c(0.4, 39.0, 39.0), c(0.6, 41.0, 41.0), c(0.01, 0.1, 15.0), c(0.99, 15.0, 0.1),
///     c(0.999, 1000.0, 1000.0), c(1e-05, 0.5, 0.5), c(0.99999,0.5,0.5))
///   for (i in 1:nrow(args))
///     cat(sprintf("%.17g, %.17g, %.17g, %.17g\n", args[i,1], args[i,2], args[i,3], pbeta(args[i,1], args[i,2], args[i,3])))'
#[rustfmt::skip]
const BETA_DISPATCH_REF: [(f64, f64, f64, f64); 22] = [
    (0.5, 0.5, 0.5, 0.49999999999999956),
    (0.1, 0.3, 0.7, 0.4333100473342346),
    (0.99999999, 10.0, 0.1, 0.7912246284470038),
    (0.99999999, 100.0, 0.1, 0.7360852918833172),
    (0.99999999, 1000.0, 0.1, 0.6676165540760735),
    (0.99999999, 100000000.0, 0.1, 0.024127343512689893),
    (0.99999999, 1000000000000.0, 0.1, 0.0),
    (0.001, 0.1, 100.0, 0.8272491543033136),
    (0.001, 0.1, 1000000.0, 1.0),
    (0.25, 100.0, 0.5, 4.042394315083124e-62),
    (0.75, 0.5, 100.0, 1.0),
    (0.9999, 200000.0, 0.5, 2.537094410550744e-10),
    (0.5, 100000000.0, 100000000.0, 0.5),
    (0.0001, 1000000.0, 0.1, 0.0),
    (0.9999, 5.0, 40.0, 1.0),
    (0.4, 39.0, 39.0, 0.037642954570293076),
    (0.6, 41.0, 41.0, 0.9659270425834079),
    (0.01, 0.1, 15.0, 0.8562029935242689),
    (0.99, 15.0, 0.1, 0.14379700647573107),
    (0.999, 1000.0, 1000.0, 1.0),
    (1e-05, 0.5, 0.5, 0.002013171839475387),
    (0.99999, 0.5, 0.5, 0.9979868281605292),

];

#[test]
fn reg_inc_beta_matches_reference() {
    for (x, a, b, e) in BETA_DISPATCH_REF {
        ref_check(
            e,
            reg_inc_beta(x, a, b),
            1e-9,
            format!("reg_inc_beta({x}, {a}, {b})"),
        );
    }
}

/// `pgamma`/`qgamma`/`qchisq` in the deep lower tail — the
/// `pgamma_raw` `else` region where the old code formed `1 − (1 − P)`
/// through `inc_gamma_upper` and floored P ≲ 2e-16 at DBL_EPSILON
/// (fix round 3: the region now uses R's `ppois_asymp` like `pgamma`
/// itself). `pgamma(x, a)` is checked as `pchisq(2x, 2a)`. Command:
///
///   Rscript -e 'ps <- c(1e-10, 1e-14, 1e-16, 1e-30, 1e-100, 1e-300)
///   for (a in c(50, 500, 3e3, 1e4, 1e6)) for (pt in ps) { x <- qgamma(pt, a); cat(sprintf("pgamma %.17g, %.17g, %.17g\n", x, a, pgamma(x, a))) }
///   qs <- c(1e-10, 1e-14, 1e-15, 1e-16, 1e-100)
///   for (a in c(1e3, 3e3, 1e4, 1e6)) for (p in qs) cat(sprintf("qgamma %.17g, %.17g, %.17g\n", p, a, qgamma(p, a)))
///   for (d in c(1e3, 3e3, 1e4, 1e6)) for (p in qs) cat(sprintf("qchisq %.17g, %.17g, %.17g\n", p, d, qchisq(p, d)))'
///
/// Verbatim output (each row's args are the R grid above, in order):
///
//   pgamma 17.19991195456241, 50, 1.0000000000000006e-10
//   pgamma 13.24755513025787, 50, 9.9999999999999732e-15
//   pgamma 11.729765631787519, 50, 1.0000000000000008e-16
//   pgamma 5.4447188498667103, 50, 1.0000000000000036e-30
//   pgamma 0.19558111701205563, 50, 1.0000000000000024e-100
//   pgamma 1.9483261670067792e-05, 50, 1.0000000000000237e-300
//   pgamma 370.63403585967654, 500, 1.0000000000000229e-10
//   pgamma 347.58724909461716, 500, 9.999999999999858e-15
//   pgamma 337.6945215796091, 500, 9.9999999999999751e-17
//   pgamma 285.22471044820162, 500, 9.9999999999996803e-31
//   pgamma 161.46698514371747, 500, 1.0000000000000226e-100
//   pgamma 51.632849087921606, 500, 1.0000000000000237e-300
//   pgamma 2664.6204371718313, 3000, 1.0000000000000201e-10
//   pgamma 2599.9308207896765, 3000, 9.999999999999533e-15
//   pgamma 2571.6002824180364, 3000, 9.9999999999996473e-17
//   pgamma 2414.818709529537, 3000, 1.0000000000000376e-30
//   pgamma 1980.2578448046629, 3000, 1.0000000000001059e-100
//   pgamma 1399.9464072151261, 3000, 1.0000000000001375e-300
//   pgamma 9376.961682044388, 10000, 1.0000000000000352e-10
//   pgamma 9254.003719701559, 10000, 1.0000000000000421e-14
//   pgamma 9199.852484754334, 10000, 1.0000000000000527e-16
//   pgamma 8896.6693669514571, 10000, 9.9999999999990287e-31
//   pgamma 8020.4643833717373, 10000, 1.0000000000004546e-100
//   pgamma 6737.6871915903293, 10000, 1.0000000000008195e-300
//   pgamma 993651.8087301997, 1000000, 1.0000000000001295e-10
//   pgamma 992368.53831248044, 1000000, 9.9999999999993815e-15
//   pgamma 991800.10480391572, 1000000, 1.0000000000004801e-16
//   pgamma 988579.41024492646, 1000000, 1.0000000000004278e-30
//   pgamma 978876.80232331576, 1000000, 9.999999999996055e-101
//   pgamma 963408.65393986576, 1000000, 1.0000000000021837e-300
//   qgamma 1e-10, 1000, 811.79876271756098
//   qgamma 1e-14, 1000, 776.88350240983686
//   qgamma 1.0000000000000001e-15, 1000, 769.15426007941937
//   qgamma 9.9999999999999998e-17, 1000, 761.74025118236432
//   qgamma 1e-100, 1000, 468.67885153942672
//   qgamma 1e-10, 3000, 2664.6204371718313
//   qgamma 1e-14, 3000, 2599.9308207896765
//   qgamma 1.0000000000000001e-15, 3000, 2585.4919773637994
//   qgamma 9.9999999999999998e-17, 3000, 2571.6002824180364
//   qgamma 1e-100, 3000, 1980.2578448046629
//   qgamma 1e-10, 10000, 9376.961682044388
//   qgamma 1e-14, 10000, 9254.003719701559
//   qgamma 1.0000000000000001e-15, 10000, 9226.4285779217917
//   qgamma 9.9999999999999998e-17, 10000, 9199.852484754334
//   qgamma 1e-100, 10000, 8020.4643833717373
//   qgamma 1e-10, 1000000, 993651.8087301997
//   qgamma 1e-14, 1000000, 992368.53831248044
//   qgamma 1.0000000000000001e-15, 1000000, 992079.33061289124
//   qgamma 9.9999999999999998e-17, 1000000, 991800.10480391572
//   qgamma 1e-100, 1000000, 978876.80232331576
//   qchisq 1e-10, 1000, 741.26807171935309
//   qchisq 1e-14, 1000, 695.17449818923433
//   qchisq 1.0000000000000001e-15, 1000, 685.05989152713244
//   qchisq 9.9999999999999998e-17, 1000, 675.3890431592182
//   qchisq 1e-100, 1000, 322.93397028743493
//   qchisq 1e-10, 3000, 2533.2497125685859
//   qchisq 1e-14, 3000, 2445.1562381450203
//   qchisq 1.0000000000000001e-15, 3000, 2425.5833299989908
//   qchisq 9.9999999999999998e-17, 3000, 2406.7835249531968
//   qchisq 1e-100, 3000, 1638.561214398655
//   qchisq 1e-10, 10000, 9126.5118024232561
//   qchisq 1e-14, 10000, 8956.0774648778843
//   qchisq 1.0000000000000001e-15, 10000, 8917.9448497332869
//   qchisq 9.9999999999999998e-17, 10000, 8881.225188935874
//   qchisq 1e-100, 10000, 7284.7615761801671
//   qchisq 1e-10, 1000000, 991029.9997742835
//   qchisq 1e-14, 1000000, 989218.70170225028
//   qchisq 1.0000000000000001e-15, 1000000, 988810.58333821443
//   qchisq 9.9999999999999998e-17, 1000000, 988416.58235771896
//   qchisq 1e-100, 1000000, 970215.08557517291
#[rustfmt::skip]
const PGAMMA_LOWER_REF: [(f64, f64, f64); 30] = [
    (17.19991195456241, 50.0, 1.0000000000000006e-10),
    (13.24755513025787, 50.0, 9.999999999999973e-15),
    (11.729765631787519, 50.0, 1.0000000000000008e-16),
    (5.44471884986671, 50.0, 1.0000000000000036e-30),
    (0.19558111701205563, 50.0, 1.0000000000000024e-100),
    (1.9483261670067792e-05, 50.0, 1.0000000000000237e-300),
    (370.63403585967654, 500.0, 1.0000000000000229e-10),
    (347.58724909461716, 500.0, 9.999999999999858e-15),
    (337.6945215796091, 500.0, 9.999999999999975e-17),
    (285.2247104482016, 500.0, 9.99999999999968e-31),
    (161.46698514371747, 500.0, 1.0000000000000226e-100),
    (51.632849087921606, 500.0, 1.0000000000000237e-300),
    (2664.6204371718313, 3000.0, 1.0000000000000201e-10),
    (2599.9308207896765, 3000.0, 9.999999999999533e-15),
    (2571.6002824180364, 3000.0, 9.999999999999647e-17),
    (2414.818709529537, 3000.0, 1.0000000000000376e-30),
    (1980.257844804663, 3000.0, 1.0000000000001059e-100),
    (1399.946407215126, 3000.0, 1.0000000000001375e-300),
    (9376.961682044388, 10000.0, 1.0000000000000352e-10),
    (9254.003719701559, 10000.0, 1.0000000000000421e-14),
    (9199.852484754334, 10000.0, 1.0000000000000527e-16),
    (8896.669366951457, 10000.0, 9.999999999999029e-31),
    (8020.464383371737, 10000.0, 1.0000000000004546e-100),
    (6737.687191590329, 10000.0, 1.0000000000008195e-300),
    (993651.8087301997, 1000000.0, 1.0000000000001295e-10),
    (992368.5383124804, 1000000.0, 9.999999999999382e-15),
    (991800.1048039157, 1000000.0, 1.0000000000004801e-16),
    (988579.4102449265, 1000000.0, 1.0000000000004278e-30),
    (978876.8023233158, 1000000.0, 9.999999999996055e-101),
    (963408.6539398658, 1000000.0, 1.0000000000021837e-300),
];

#[rustfmt::skip]
const GAMMA_QUANTILE_LOWER_REF: [(f64, f64, f64); 20] = [
    (1e-10, 1000.0, 811.798762717561),
    (1e-14, 1000.0, 776.8835024098369),
    (1e-15, 1000.0, 769.1542600794194),
    (1e-16, 1000.0, 761.7402511823643),
    (1e-100, 1000.0, 468.6788515394267),
    (1e-10, 3000.0, 2664.6204371718313),
    (1e-14, 3000.0, 2599.9308207896765),
    (1e-15, 3000.0, 2585.4919773637994),
    (1e-16, 3000.0, 2571.6002824180364),
    (1e-100, 3000.0, 1980.257844804663),
    (1e-10, 10000.0, 9376.961682044388),
    (1e-14, 10000.0, 9254.003719701559),
    (1e-15, 10000.0, 9226.428577921792),
    (1e-16, 10000.0, 9199.852484754334),
    (1e-100, 10000.0, 8020.464383371737),
    (1e-10, 1000000.0, 993651.8087301997),
    (1e-14, 1000000.0, 992368.5383124804),
    (1e-15, 1000000.0, 992079.3306128912),
    (1e-16, 1000000.0, 991800.1048039157),
    (1e-100, 1000000.0, 978876.8023233158),
];

#[rustfmt::skip]
const CHISQ_QUANTILE_LOWER_REF: [(f64, f64, f64); 20] = [
    (1e-10, 1000.0, 741.2680717193531),
    (1e-14, 1000.0, 695.1744981892343),
    (1e-15, 1000.0, 685.0598915271324),
    (1e-16, 1000.0, 675.3890431592182),
    (1e-100, 1000.0, 322.93397028743493),
    (1e-10, 3000.0, 2533.249712568586),
    (1e-14, 3000.0, 2445.1562381450203),
    (1e-15, 3000.0, 2425.583329998991),
    (1e-16, 3000.0, 2406.783524953197),
    (1e-100, 3000.0, 1638.561214398655),
    (1e-10, 10000.0, 9126.511802423256),
    (1e-14, 10000.0, 8956.077464877884),
    (1e-15, 10000.0, 8917.944849733287),
    (1e-16, 10000.0, 8881.225188935874),
    (1e-100, 10000.0, 7284.761576180167),
    (1e-10, 1000000.0, 991029.9997742835),
    (1e-14, 1000000.0, 989218.7017022503),
    (1e-15, 1000000.0, 988810.5833382144),
    (1e-16, 1000000.0, 988416.582357719),
    (1e-100, 1000000.0, 970215.0855751729),
];

#[test]
fn gamma_lower_tail_matches_reference() {
    for (x, a, e) in PGAMMA_LOWER_REF {
        ref_check(
            e,
            chisq_cdf(2.0 * x, 2.0 * a),
            1e-9,
            format!("pgamma({x}, {a})"),
        );
    }
    for (p, a, e) in GAMMA_QUANTILE_LOWER_REF {
        let actual = gamma_quantile(p, a, 1.0)
            .unwrap_or_else(|e| panic!("gamma_quantile({p}, {a}, 1): {e:?}"));
        ref_check(e, actual, 1e-6, format!("qgamma({p}, {a})"));
    }
    for (p, df, e) in CHISQ_QUANTILE_LOWER_REF {
        let actual =
            chisq_quantile(p, df).unwrap_or_else(|e| panic!("chisq_quantile({p}, {df}): {e:?}"));
        ref_check(e, actual, 1e-6, format!("qchisq({p}, {df})"));
    }
}
// ---------------------------------------------------------------------------
// Round-4 review fix (PR #69): `inc_gamma_lower`/`inc_gamma_upper` are now
// R 4.6.2 `pgamma_raw` end to end, including the `res < DBL_MIN/DBL_EPSILON`
// log-space redo and the log_p forms of `dpois_wrap` (via `ebd0`), `pd_*`,
// `ppois_asymp` and `pgamma_smallx`. Both tables come from ONE `Rscript -e`
// each (R 4.6.2), quoted verbatim and machine-converted — nothing hand-typed.
// pgamma(x, a) is checked as `pchisq(2x, 2a)`; the upper tail as
// `pchisq(2x, 2a, lower.tail = FALSE)`.

// Denormal-band `pgamma` — the review's five misses plus a broad grid
// (`a` in 1e-15..1e6, `x` at `qgamma(q, a)` for q down to 1e-320, both
// tails), covering outputs from 1e-321 denormals up through the redo
// band's top (~1e-280). Command:
//
//   Rscript -e 'p <- function(tag, g) cat(tag, "|", paste(sprintf("%.17g", g), collapse = " "), "\n", sep = "")
//   args <- rbind(
//     c(1475.25, 3162.28), c(25298.2, 31622.8), c(25335.05, 31622.8),
//     c(25338.22, 31622.8), c(24765.16, 31000)
//   )
//   for (i in 1:nrow(args)) p(sprintf("den %.17g %.17g", args[i,1], args[i,2]), c(pgamma(args[i,1], args[i,2]), pgamma(args[i,1], args[i,2], lower.tail=FALSE)))
//   for (a in c(1e-15, 1e-8, 0.5, 1, 2, 50, 500, 1e3, 3e3, 1e4, 3e4, 1e5, 1e6)) {
//     for (q in c(1e-280, 1e-290, 1e-295, 1e-300, 1e-305, 1e-308, 1e-312, 1e-316, 1e-320)) {
//       x <- qgamma(q, a);          if (is.finite(x) && x > 0) p(sprintf("lo %.17g %.17g", x, a), c(pgamma(x, a), pgamma(x, a, lower.tail=FALSE)))
//       x <- qgamma(q, a, lower.tail=FALSE); if (is.finite(x) && x > 0) p(sprintf("up %.17g %.17g", x, a), c(pgamma(x, a), pgamma(x, a, lower.tail=FALSE)))
//     }
//   }'
//
// Verbatim output (tag x a | lower upper — each row machine-converted
// into PGAMMA_DENORMAL_REF in order):
//
//   den 1475.25 3162.2800000000002|4.474434870173894e-317 1
//   den 25298.200000000001 31622.799999999999|1.5879269857337664e-320 1
//   den 25335.049999999999 31622.799999999999|1.5479471936722927e-316 1
//   den 25338.220000000001 31622.799999999999|3.4005203931943582e-316 1
//   den 24765.16 31000|4.6867314197323568e-318 1
//   up 603.78018730324368 1.0000000000000001e-15|1 1.000000000000006e-280
//   up 626.76873126647274 1.0000000000000001e-15|1 9.9999999999998583e-291
//   up 638.26351172697139 1.0000000000000001e-15|1 9.9999999999997594e-296
//   up 649.75861507496847 1.0000000000000001e-15|1 1.0000000000000237e-300
//   up 661.25403002802386 1.0000000000000001e-15|1 9.9999999999984405e-306
//   up 668.15142409548787 1.0000000000000001e-15|1 9.9999999999998657e-309
//   up 677.34811418711331 1.0000000000000001e-15|1 9.9999999999846534e-313
//   up 686.54498785258158 1.0000000000000001e-15|1 9.9999998365971443e-317
//   up 695.7420512920994 1.0000000000000001e-15|1 9.9998886718268301e-321
//   up 619.87202297179397 1e-08|1 9.9999999999986797e-281
//   up 642.86151503531869 1e-08|1 9.9999999999984812e-291
//   up 654.35674434596001 1e-08|1 9.9999999999997594e-296
//   up 665.85228091361273 1e-08|1 1.0000000000001375e-300
//   up 677.34811425797614 1e-08|1 9.9999999999995775e-306
//   up 684.24575255597404 1e-08|1 9.9999999999998657e-309
//   up 693.44276066831526 1e-08|1 9.9999999999846534e-313
//   up 702.63994396180101 1e-08|1 9.9999998365971443e-317
//   up 711.837308964763 1e-08|1 9.9998886718268301e-321
//   up 640.91923075962529 0.5|1 1.0000000000000274e-280
//   up 663.92747388496377 0.5|1 1.0000000000000422e-290
//   up 675.431822476501 0.5|1 1.0000000000000896e-295
//   up 686.93631561119696 0.5|1 1.0000000000001375e-300
//   up 698.44094849949477 0.5|1 9.9999999999995775e-306
//   up 705.34379340986311 0.5|1 9.9999999999998657e-309
//   up 714.5476606976747 0.5|1 9.9999999999846534e-313
//   up 723.75161067673196 0.5|1 9.9999998365971443e-317
//   up 732.95565233879267 0.5|1 9.9998886718268301e-321
//   lo 9.9999999999989133e-281 1|9.9999999999989133e-281 1
//   up 644.72382603833285 1|1 9.9999999999994638e-281
//   lo 9.9999999999998683e-291 1|9.9999999999998683e-291 1
//   up 667.74967696827321 1|1 1.0000000000000418e-290
//   lo 9.9999999999992097e-296 1|9.9999999999997594e-296 1
//   up 679.2626024332435 1|1 9.9999999999997594e-296
//   lo 9.9999999999996869e-301 1|1.0000000000000237e-300 1
//   up 690.77552789821368 1|1 1.0000000000000237e-300
//   lo 9.9999999999990273e-306 1|9.9999999999995775e-306 1
//   up 702.28845336318398 1|1 9.9999999999995775e-306
//   lo 9.9999999999993173e-309 1|9.9999999999998657e-309 1
//   up 709.19620864216608 1|1 9.9999999999998657e-309
//   lo 9.9999999999846534e-313 1|9.9999999999846534e-313 1
//   up 718.40654901414382 1|1 9.9999999999846534e-313
//   lo 9.9999998365971443e-317 1|9.9999998365971443e-317 1
//   up 727.6168894024587 1|1 9.9999998365971443e-317
//   lo 9.9998886718268301e-321 1|9.9998886718268301e-321 1
//   up 736.82724089097394 1|1 9.9998886718268301e-321
//   lo 1.4142135623730209e-140 2|9.9999999999989537e-281 1
//   up 651.2041837164428 2|1 9.9999999999992473e-281
//   lo 1.4142135623730884e-145 2|9.9999999999999081e-291 1
//   up 674.26478185163808 2|1 1.0000000000000051e-290
//   lo 4.4721359549994112e-148 2|9.999999999998623e-296 1
//   up 685.79463775465717 2|1 9.9999999999997594e-296
//   lo 1.4142135623730757e-150 2|9.9999999999991001e-301 1
//   up 697.3242113793525 2|1 1.0000000000000237e-300
//   lo 4.4721359549993705e-153 2|9.9999999999984405e-306 1
//   up 708.85351199079241 2|1 9.9999999999995775e-306
//   lo 1.4142135623730493e-154 2|9.9999999999987293e-309 1
//   up 715.77096499658421 2|1 9.9999999999987293e-309
//   lo 1.4142135623719509e-156 2|9.9999999999846534e-313 1
//   up 724.99409088966502 2|1 9.9999999999846534e-313
//   lo 1.4142135508187502e-158 2|9.9999998365971443e-317 1
//   up 734.21705517123337 2|1 9.9999998365971443e-317
//   lo 1.4142056902605067e-160 2|9.9998886718268301e-321 1
//   up 743.43987297822241 2|1 9.9998886718268301e-321
//   lo 4.8939768897147307e-05 50|1.0000000000000095e-280 1
//   up 829.54168161633186 50|1 1.0000000000000209e-280
//   lo 3.0878895687526133e-05 50|1.0000000000001262e-290 1
//   up 853.98894128660856 50|1 1.0000000000000132e-290
//   lo 2.4527975647291754e-05 50|9.9999999999997594e-296 1
//   up 866.19649484021409 50|1 9.9999999999997594e-296
//   lo 1.9483261670067792e-05 50|1.0000000000000237e-300 0.99999999999999989
//   up 878.3937665919384 50|1 1.0000000000000237e-300
//   lo 1.5476103633071999e-05 50|9.9999999999995775e-306 0.99999999999999989
//   up 890.58106463128286 50|1 9.9999999999995775e-306
//   lo 1.3479122250997504e-05 50|9.9999999999998657e-309 1
//   up 897.88877902048921 50|1 9.9999999999987293e-309
//   lo 1.1211445056134629e-05 50|9.9999999999846534e-313 1
//   up 907.62708374558167 50|1 9.9999999999846534e-313
//   lo 9.3252734714700409e-06 50|9.9999998365971443e-317 1
//   up 917.35944355670074 50|1 9.9999998365971443e-317
//   lo 7.7564226661004054e-06 50|9.9998886718268301e-321 1
//   up 927.08600600880015 50|1 9.9998886718268301e-321
//   lo 57.252779278208671 500|9.9999999999986014e-281 1
//   up 1772.5435141046225 500|1 9.9999999999984253e-281
//   lo 54.361422322744687 500|9.9999999999998967e-291 1
//   up 1804.4704359438006 500|1 1.0000000000001183e-290
//   lo 52.977496877954486 500|9.9999999999997594e-296 1
//   up 1820.3528816511905 500|1 9.9999999999997594e-296
//   lo 51.632849087921606 500|1.0000000000000237e-300 1
//   up 1836.1832722783238 500|1 9.9999999999991001e-301
//   lo 50.326139522110317 500|9.9999999999984405e-306 1
//   up 1851.9629959049744 500|1 1.0000000000000715e-305
//   lo 49.559789000048028 500|9.9999999999998657e-309 1
//   up 1861.4070654649552 500|1 1.0000000000001002e-308
//   lo 48.558054410953723 500|9.9999999999846534e-313 1
//   up 1873.9720099603028 500|1 9.9999999999846534e-313
//   lo 47.578691119636979 500|9.9999998365971443e-317 1
//   up 1886.5065126672594 500|1 9.9999998365971443e-317
//   lo 46.621105107069475 500|9.9998886718268301e-321 1
//   up 1899.0112071435108 500|1 9.9998886718268301e-321
//   lo 248.55826834342676 1000|1.0000000000000053e-280 1
//   up 2592.5118104624571 1000|1 9.9999999999995066e-281
//   lo 241.09722092697129 1000|1.0000000000000794e-290 1
//   up 2629.7921490420322 1000|1 9.999999999997886e-291
//   lo 237.47733753611672 1000|1.0000000000000896e-295 1
//   up 2648.3110917187742 1000|1 9.999999999998623e-296
//   lo 233.92836429052846 1000|1.0000000000000237e-300 1
//   up 2666.7520733019951 1000|1 1.0000000000001375e-300
//   lo 230.44827724025353 1000|9.9999999999995775e-306 1
//   up 2685.1171231638641 1000|1 1.0000000000000715e-305
//   lo 228.39248023515816 1000|9.9999999999998657e-309 1
//   up 2696.1005183948905 1000|1 9.999999999997593e-309
//   lo 225.68820691964848 1000|9.9999999999846534e-313 1
//   up 2710.70431601367 1000|1 9.9999999999846534e-313
//   lo 223.02512562217515 1000|9.9999998365971443e-317 1
//   up 2725.2624217392449 1000|1 9.9999998365971443e-317
//   lo 220.40233091415243 1000|9.9998886718268301e-321 1
//   up 2739.7757586545349 1000|1 9.9998886718268301e-321
//   lo 1441.3411893729242 3000|9.9999999999988558e-281 1
//   up 5407.5999444197096 3000|1 9.9999999999982026e-281
//   lo 1420.3576725244573 3000|9.9999999999979914e-291 1
//   up 5458.9678069295096 3000|1 1.0000000000003354e-290
//   lo 1410.082411583875 3000|1.0000000000002032e-295 1
//   up 5484.4317072641243 3000|1 1.0000000000002032e-295
//   lo 1399.9464072151261 3000|1.0000000000001375e-300 1
//   up 5509.7539877055096 3000|1 9.9999999999968255e-301
//   lo 1389.9460525503559 3000|9.9999999999961676e-306 1
//   up 5534.9382497780871 3000|1 9.9999999999984405e-306
//   lo 1384.0095070227467 3000|9.999999999997593e-309 1
//   up 5549.9840006875938 3000|1 9.9999999999964517e-309
//   lo 1376.1668897365882 3000|9.9999999999846534e-313 1
//   up 5569.9708940919281 3000|1 9.9999999999846534e-313
//   lo 1368.4059233290616 3000|9.9999998365971443e-317 1
//   up 5589.8746132836186 3000|1 9.9999998365971443e-317
//   lo 1360.7249872113543 3000|9.9998886718268301e-321 1
//   up 5609.6967912233131 3000|1 9.9998886718268301e-321
//   lo 6834.8635443373869 10000|1.000000000000107e-280 1
//   up 14016.891044168409 10000|1 1.0000000000005775e-280
//   lo 6785.7347743580913 10000|9.9999999999926256e-291 1
//   up 14096.608783920521 10000|1 1.0000000000005537e-290
//   lo 6761.5793384930184 10000|1.0000000000005444e-295 1
//   up 14136.057838220062 10000|1 1.0000000000007717e-295
//   lo 6737.6871915903293 10000|1.0000000000008195e-300 1
//   up 14175.243016921313 10000|1 9.9999999999922782e-301
//   lo 6714.0516916896649 10000|1.0000000000008673e-305 1
//   up 14214.170955659256 10000|1 9.9999999999927564e-306
//   lo 6699.9909217590275 10000|9.9999999999919063e-309 1
//   up 14237.406901757608 10000|1 9.9999999999953154e-309
//   lo 6681.3810806910633 10000|9.9999999999846534e-313 1
//   up 14268.249973465472 10000|1 9.9999999999846534e-313
//   lo 6662.9259800816444 10000|9.9999998365971443e-317 1
//   up 14298.937916315483 10000|1 9.9999998365971443e-317
//   lo 6644.6226291000876 10000|9.9998886718268301e-321 1
//   up 14329.47373311302 10000|1 9.9998886718268301e-321
//   lo 24221.100662372759 30000|9.9999999999992118e-281 1
//   up 36631.462873519966 30000|1 1.000000000000385e-280
//   lo 24125.640110605083 30000|1.0000000000004165e-290 1
//   up 36757.571441771848 30000|1 9.9999999999948072e-291
//   lo 24078.624176615715 30000|1.0000000000000896e-295 1
//   up 36819.911295730657 30000|1 1.0000000000000896e-295
//   lo 24032.068249697299 30000|9.9999999999922782e-301 1
//   up 36881.791074670989 30000|1 9.9999999999979628e-301
//   lo 23985.960798732907 30000|1.0000000000014357e-305 1
//   up 36943.222303310686 30000|1 1.0000000000001852e-305
//   lo 23958.506977441019 30000|1.0000000000010098e-308 1
//   up 36979.870352977356 30000|1 9.9999999999987293e-309
//   lo 23922.142837158932 30000|9.9999999999846534e-313 1
//   up 37028.493417197176 30000|1 9.9999999999846534e-313
//   lo 23886.049203644438 30000|9.9999998365971443e-317 1
//   up 37076.845918049585 30000|1 9.9999998365971443e-317
//   lo 23850.220878884469 30000|9.9998886718268301e-321 1
//   up 37124.933065516067 30000|1 9.9998886718268301e-321
//   lo 89106.708960396645 100000|1.0000000000001014e-280 1
//   up 111746.13822521623 100000|1 1.0000000000003294e-280
//   lo 88920.289577458549 100000|1.0000000000004125e-290 1
//   up 111963.22634951196 100000|1 1.0000000000003426e-290
//   lo 88828.387643754642 100000|1.0000000000002032e-295 1
//   up 112070.46283957762 100000|1 1.0000000000005444e-295
//   lo 88737.3279114217 100000|1.0000000000000237e-300 1
//   up 112176.85724295325 100000|1 1.0000000000005922e-300
//   lo 88647.089311527074 100000|1.0000000000006399e-305 1
//   up 112282.43062216583 100000|1 1.0000000000007536e-305
//   lo 88593.331855179931 100000|9.999999999997593e-309 1
//   up 112345.38899552909 100000|1 1.0000000000004411e-308
//   lo 88522.096458751344 100000|9.9999999999846534e-313 1
//   up 112428.8923363459 100000|1 9.9999999999846534e-313
//   lo 88451.356405906146 100000|9.9999998365971443e-317 1
//   up 112511.90039384722 100000|1 9.9999998365971443e-317
//   lo 88381.102192901817 100000|9.9998886718268301e-321 1
//   up 112594.42268375172 100000|1 9.9998886718268301e-321
//   lo 964641.7911315955 1000000|9.999999999981942e-281 1
//   up 1036211.1655328015 1000000|1 1.0000000000015793e-280
//   lo 964019.75012513099 1000000|9.9999999999848417e-291 1
//   up 1036863.8832824981 1000000|1 9.9999999999927865e-291
//   lo 963712.86908329639 1000000|9.9999999999963479e-296 1
//   up 1037186.1029875163 1000000|1 1.0000000000003171e-295
//   lo 963408.65393986576 1000000|1.0000000000021837e-300 1
//   up 1037505.6569794014 1000000|1 1.0000000000016152e-300
//   lo 963107.03805306577 1000000|9.9999999999984405e-306 1
//   up 1037822.6118935209 1000000|1 9.9999999999813883e-306
//   lo 962927.28946805408 1000000|9.999999999997593e-309 1
//   up 1038011.5639782328 1000000|1 9.9999999999907699e-309
//   lo 962689.02136879007 1000000|9.9999999999846534e-313 1
//   up 1038262.1035049941 1000000|1 9.9999999999846534e-313
//   lo 962452.3213285706 1000000|9.9999998365971443e-317 1
//   up 1038511.0750781563 1000000|1 9.9999998365971443e-317
//   lo 962217.15927049809 1000000|9.9998886718268301e-321 1
//   up 1038758.5087865973 1000000|1 9.9998886718268301e-321

#[rustfmt::skip]
#[allow(clippy::unreadable_literal, clippy::excessive_precision)]
// verbatim machine-converted R `%.17g` output
const PGAMMA_DENORMAL_REF: [(f64, f64, f64, f64); 212] = [
    (1475.25, 3162.2800000000002, 4.474434870173894e-317, 1.0), // den
    (25298.200000000001, 31622.799999999999, 1.5879269857337664e-320, 1.0), // den
    (25335.049999999999, 31622.799999999999, 1.5479471936722927e-316, 1.0), // den
    (25338.220000000001, 31622.799999999999, 3.4005203931943582e-316, 1.0), // den
    (24765.16, 31000.0, 4.6867314197323568e-318, 1.0), // den
    (603.78018730324368, 1.0000000000000001e-15, 1.0, 1.000000000000006e-280), // up
    (626.76873126647274, 1.0000000000000001e-15, 1.0, 9.9999999999998583e-291), // up
    (638.26351172697139, 1.0000000000000001e-15, 1.0, 9.9999999999997594e-296), // up
    (649.75861507496847, 1.0000000000000001e-15, 1.0, 1.0000000000000237e-300), // up
    (661.25403002802386, 1.0000000000000001e-15, 1.0, 9.9999999999984405e-306), // up
    (668.15142409548787, 1.0000000000000001e-15, 1.0, 9.9999999999998657e-309), // up
    (677.34811418711331, 1.0000000000000001e-15, 1.0, 9.9999999999846534e-313), // up
    (686.54498785258158, 1.0000000000000001e-15, 1.0, 9.9999998365971443e-317), // up
    (695.7420512920994, 1.0000000000000001e-15, 1.0, 9.9998886718268301e-321), // up
    (619.87202297179397, 1e-08, 1.0, 9.9999999999986797e-281), // up
    (642.86151503531869, 1e-08, 1.0, 9.9999999999984812e-291), // up
    (654.35674434596001, 1e-08, 1.0, 9.9999999999997594e-296), // up
    (665.85228091361273, 1e-08, 1.0, 1.0000000000001375e-300), // up
    (677.34811425797614, 1e-08, 1.0, 9.9999999999995775e-306), // up
    (684.24575255597404, 1e-08, 1.0, 9.9999999999998657e-309), // up
    (693.44276066831526, 1e-08, 1.0, 9.9999999999846534e-313), // up
    (702.63994396180101, 1e-08, 1.0, 9.9999998365971443e-317), // up
    (711.837308964763, 1e-08, 1.0, 9.9998886718268301e-321), // up
    (640.91923075962529, 0.5, 1.0, 1.0000000000000274e-280), // up
    (663.92747388496377, 0.5, 1.0, 1.0000000000000422e-290), // up
    (675.431822476501, 0.5, 1.0, 1.0000000000000896e-295), // up
    (686.93631561119696, 0.5, 1.0, 1.0000000000001375e-300), // up
    (698.44094849949477, 0.5, 1.0, 9.9999999999995775e-306), // up
    (705.34379340986311, 0.5, 1.0, 9.9999999999998657e-309), // up
    (714.5476606976747, 0.5, 1.0, 9.9999999999846534e-313), // up
    (723.75161067673196, 0.5, 1.0, 9.9999998365971443e-317), // up
    (732.95565233879267, 0.5, 1.0, 9.9998886718268301e-321), // up
    (9.9999999999989133e-281, 1.0, 9.9999999999989133e-281, 1.0), // lo
    (644.72382603833285, 1.0, 1.0, 9.9999999999994638e-281), // up
    (9.9999999999998683e-291, 1.0, 9.9999999999998683e-291, 1.0), // lo
    (667.74967696827321, 1.0, 1.0, 1.0000000000000418e-290), // up
    (9.9999999999992097e-296, 1.0, 9.9999999999997594e-296, 1.0), // lo
    (679.2626024332435, 1.0, 1.0, 9.9999999999997594e-296), // up
    (9.9999999999996869e-301, 1.0, 1.0000000000000237e-300, 1.0), // lo
    (690.77552789821368, 1.0, 1.0, 1.0000000000000237e-300), // up
    (9.9999999999990273e-306, 1.0, 9.9999999999995775e-306, 1.0), // lo
    (702.28845336318398, 1.0, 1.0, 9.9999999999995775e-306), // up
    (9.9999999999993173e-309, 1.0, 9.9999999999998657e-309, 1.0), // lo
    (709.19620864216608, 1.0, 1.0, 9.9999999999998657e-309), // up
    (9.9999999999846534e-313, 1.0, 9.9999999999846534e-313, 1.0), // lo
    (718.40654901414382, 1.0, 1.0, 9.9999999999846534e-313), // up
    (9.9999998365971443e-317, 1.0, 9.9999998365971443e-317, 1.0), // lo
    (727.6168894024587, 1.0, 1.0, 9.9999998365971443e-317), // up
    (9.9998886718268301e-321, 1.0, 9.9998886718268301e-321, 1.0), // lo
    (736.82724089097394, 1.0, 1.0, 9.9998886718268301e-321), // up
    (1.4142135623730209e-140, 2.0, 9.9999999999989537e-281, 1.0), // lo
    (651.2041837164428, 2.0, 1.0, 9.9999999999992473e-281), // up
    (1.4142135623730884e-145, 2.0, 9.9999999999999081e-291, 1.0), // lo
    (674.26478185163808, 2.0, 1.0, 1.0000000000000051e-290), // up
    (4.4721359549994112e-148, 2.0, 9.999999999998623e-296, 1.0), // lo
    (685.79463775465717, 2.0, 1.0, 9.9999999999997594e-296), // up
    (1.4142135623730757e-150, 2.0, 9.9999999999991001e-301, 1.0), // lo
    (697.3242113793525, 2.0, 1.0, 1.0000000000000237e-300), // up
    (4.4721359549993705e-153, 2.0, 9.9999999999984405e-306, 1.0), // lo
    (708.85351199079241, 2.0, 1.0, 9.9999999999995775e-306), // up
    (1.4142135623730493e-154, 2.0, 9.9999999999987293e-309, 1.0), // lo
    (715.77096499658421, 2.0, 1.0, 9.9999999999987293e-309), // up
    (1.4142135623719509e-156, 2.0, 9.9999999999846534e-313, 1.0), // lo
    (724.99409088966502, 2.0, 1.0, 9.9999999999846534e-313), // up
    (1.4142135508187502e-158, 2.0, 9.9999998365971443e-317, 1.0), // lo
    (734.21705517123337, 2.0, 1.0, 9.9999998365971443e-317), // up
    (1.4142056902605067e-160, 2.0, 9.9998886718268301e-321, 1.0), // lo
    (743.43987297822241, 2.0, 1.0, 9.9998886718268301e-321), // up
    (4.8939768897147307e-05, 50.0, 1.0000000000000095e-280, 1.0), // lo
    (829.54168161633186, 50.0, 1.0, 1.0000000000000209e-280), // up
    (3.0878895687526133e-05, 50.0, 1.0000000000001262e-290, 1.0), // lo
    (853.98894128660856, 50.0, 1.0, 1.0000000000000132e-290), // up
    (2.4527975647291754e-05, 50.0, 9.9999999999997594e-296, 1.0), // lo
    (866.19649484021409, 50.0, 1.0, 9.9999999999997594e-296), // up
    (1.9483261670067792e-05, 50.0, 1.0000000000000237e-300, 0.99999999999999989), // lo
    (878.3937665919384, 50.0, 1.0, 1.0000000000000237e-300), // up
    (1.5476103633071999e-05, 50.0, 9.9999999999995775e-306, 0.99999999999999989), // lo
    (890.58106463128286, 50.0, 1.0, 9.9999999999995775e-306), // up
    (1.3479122250997504e-05, 50.0, 9.9999999999998657e-309, 1.0), // lo
    (897.88877902048921, 50.0, 1.0, 9.9999999999987293e-309), // up
    (1.1211445056134629e-05, 50.0, 9.9999999999846534e-313, 1.0), // lo
    (907.62708374558167, 50.0, 1.0, 9.9999999999846534e-313), // up
    (9.3252734714700409e-06, 50.0, 9.9999998365971443e-317, 1.0), // lo
    (917.35944355670074, 50.0, 1.0, 9.9999998365971443e-317), // up
    (7.7564226661004054e-06, 50.0, 9.9998886718268301e-321, 1.0), // lo
    (927.08600600880015, 50.0, 1.0, 9.9998886718268301e-321), // up
    (57.252779278208671, 500.0, 9.9999999999986014e-281, 1.0), // lo
    (1772.5435141046225, 500.0, 1.0, 9.9999999999984253e-281), // up
    (54.361422322744687, 500.0, 9.9999999999998967e-291, 1.0), // lo
    (1804.4704359438006, 500.0, 1.0, 1.0000000000001183e-290), // up
    (52.977496877954486, 500.0, 9.9999999999997594e-296, 1.0), // lo
    (1820.3528816511905, 500.0, 1.0, 9.9999999999997594e-296), // up
    (51.632849087921606, 500.0, 1.0000000000000237e-300, 1.0), // lo
    (1836.1832722783238, 500.0, 1.0, 9.9999999999991001e-301), // up
    (50.326139522110317, 500.0, 9.9999999999984405e-306, 1.0), // lo
    (1851.9629959049744, 500.0, 1.0, 1.0000000000000715e-305), // up
    (49.559789000048028, 500.0, 9.9999999999998657e-309, 1.0), // lo
    (1861.4070654649552, 500.0, 1.0, 1.0000000000001002e-308), // up
    (48.558054410953723, 500.0, 9.9999999999846534e-313, 1.0), // lo
    (1873.9720099603028, 500.0, 1.0, 9.9999999999846534e-313), // up
    (47.578691119636979, 500.0, 9.9999998365971443e-317, 1.0), // lo
    (1886.5065126672594, 500.0, 1.0, 9.9999998365971443e-317), // up
    (46.621105107069475, 500.0, 9.9998886718268301e-321, 1.0), // lo
    (1899.0112071435108, 500.0, 1.0, 9.9998886718268301e-321), // up
    (248.55826834342676, 1000.0, 1.0000000000000053e-280, 1.0), // lo
    (2592.5118104624571, 1000.0, 1.0, 9.9999999999995066e-281), // up
    (241.09722092697129, 1000.0, 1.0000000000000794e-290, 1.0), // lo
    (2629.7921490420322, 1000.0, 1.0, 9.999999999997886e-291), // up
    (237.47733753611672, 1000.0, 1.0000000000000896e-295, 1.0), // lo
    (2648.3110917187742, 1000.0, 1.0, 9.999999999998623e-296), // up
    (233.92836429052846, 1000.0, 1.0000000000000237e-300, 1.0), // lo
    (2666.7520733019951, 1000.0, 1.0, 1.0000000000001375e-300), // up
    (230.44827724025353, 1000.0, 9.9999999999995775e-306, 1.0), // lo
    (2685.1171231638641, 1000.0, 1.0, 1.0000000000000715e-305), // up
    (228.39248023515816, 1000.0, 9.9999999999998657e-309, 1.0), // lo
    (2696.1005183948905, 1000.0, 1.0, 9.999999999997593e-309), // up
    (225.68820691964848, 1000.0, 9.9999999999846534e-313, 1.0), // lo
    (2710.70431601367, 1000.0, 1.0, 9.9999999999846534e-313), // up
    (223.02512562217515, 1000.0, 9.9999998365971443e-317, 1.0), // lo
    (2725.2624217392449, 1000.0, 1.0, 9.9999998365971443e-317), // up
    (220.40233091415243, 1000.0, 9.9998886718268301e-321, 1.0), // lo
    (2739.7757586545349, 1000.0, 1.0, 9.9998886718268301e-321), // up
    (1441.3411893729242, 3000.0, 9.9999999999988558e-281, 1.0), // lo
    (5407.5999444197096, 3000.0, 1.0, 9.9999999999982026e-281), // up
    (1420.3576725244573, 3000.0, 9.9999999999979914e-291, 1.0), // lo
    (5458.9678069295096, 3000.0, 1.0, 1.0000000000003354e-290), // up
    (1410.082411583875, 3000.0, 1.0000000000002032e-295, 1.0), // lo
    (5484.4317072641243, 3000.0, 1.0, 1.0000000000002032e-295), // up
    (1399.9464072151261, 3000.0, 1.0000000000001375e-300, 1.0), // lo
    (5509.7539877055096, 3000.0, 1.0, 9.9999999999968255e-301), // up
    (1389.9460525503559, 3000.0, 9.9999999999961676e-306, 1.0), // lo
    (5534.9382497780871, 3000.0, 1.0, 9.9999999999984405e-306), // up
    (1384.0095070227467, 3000.0, 9.999999999997593e-309, 1.0), // lo
    (5549.9840006875938, 3000.0, 1.0, 9.9999999999964517e-309), // up
    (1376.1668897365882, 3000.0, 9.9999999999846534e-313, 1.0), // lo
    (5569.9708940919281, 3000.0, 1.0, 9.9999999999846534e-313), // up
    (1368.4059233290616, 3000.0, 9.9999998365971443e-317, 1.0), // lo
    (5589.8746132836186, 3000.0, 1.0, 9.9999998365971443e-317), // up
    (1360.7249872113543, 3000.0, 9.9998886718268301e-321, 1.0), // lo
    (5609.6967912233131, 3000.0, 1.0, 9.9998886718268301e-321), // up
    (6834.8635443373869, 10000.0, 1.000000000000107e-280, 1.0), // lo
    (14016.891044168409, 10000.0, 1.0, 1.0000000000005775e-280), // up
    (6785.7347743580913, 10000.0, 9.9999999999926256e-291, 1.0), // lo
    (14096.608783920521, 10000.0, 1.0, 1.0000000000005537e-290), // up
    (6761.5793384930184, 10000.0, 1.0000000000005444e-295, 1.0), // lo
    (14136.057838220062, 10000.0, 1.0, 1.0000000000007717e-295), // up
    (6737.6871915903293, 10000.0, 1.0000000000008195e-300, 1.0), // lo
    (14175.243016921313, 10000.0, 1.0, 9.9999999999922782e-301), // up
    (6714.0516916896649, 10000.0, 1.0000000000008673e-305, 1.0), // lo
    (14214.170955659256, 10000.0, 1.0, 9.9999999999927564e-306), // up
    (6699.9909217590275, 10000.0, 9.9999999999919063e-309, 1.0), // lo
    (14237.406901757608, 10000.0, 1.0, 9.9999999999953154e-309), // up
    (6681.3810806910633, 10000.0, 9.9999999999846534e-313, 1.0), // lo
    (14268.249973465472, 10000.0, 1.0, 9.9999999999846534e-313), // up
    (6662.9259800816444, 10000.0, 9.9999998365971443e-317, 1.0), // lo
    (14298.937916315483, 10000.0, 1.0, 9.9999998365971443e-317), // up
    (6644.6226291000876, 10000.0, 9.9998886718268301e-321, 1.0), // lo
    (14329.47373311302, 10000.0, 1.0, 9.9998886718268301e-321), // up
    (24221.100662372759, 30000.0, 9.9999999999992118e-281, 1.0), // lo
    (36631.462873519966, 30000.0, 1.0, 1.000000000000385e-280), // up
    (24125.640110605083, 30000.0, 1.0000000000004165e-290, 1.0), // lo
    (36757.571441771848, 30000.0, 1.0, 9.9999999999948072e-291), // up
    (24078.624176615715, 30000.0, 1.0000000000000896e-295, 1.0), // lo
    (36819.911295730657, 30000.0, 1.0, 1.0000000000000896e-295), // up
    (24032.068249697299, 30000.0, 9.9999999999922782e-301, 1.0), // lo
    (36881.791074670989, 30000.0, 1.0, 9.9999999999979628e-301), // up
    (23985.960798732907, 30000.0, 1.0000000000014357e-305, 1.0), // lo
    (36943.222303310686, 30000.0, 1.0, 1.0000000000001852e-305), // up
    (23958.506977441019, 30000.0, 1.0000000000010098e-308, 1.0), // lo
    (36979.870352977356, 30000.0, 1.0, 9.9999999999987293e-309), // up
    (23922.142837158932, 30000.0, 9.9999999999846534e-313, 1.0), // lo
    (37028.493417197176, 30000.0, 1.0, 9.9999999999846534e-313), // up
    (23886.049203644438, 30000.0, 9.9999998365971443e-317, 1.0), // lo
    (37076.845918049585, 30000.0, 1.0, 9.9999998365971443e-317), // up
    (23850.220878884469, 30000.0, 9.9998886718268301e-321, 1.0), // lo
    (37124.933065516067, 30000.0, 1.0, 9.9998886718268301e-321), // up
    (89106.708960396645, 100000.0, 1.0000000000001014e-280, 1.0), // lo
    (111746.13822521623, 100000.0, 1.0, 1.0000000000003294e-280), // up
    (88920.289577458549, 100000.0, 1.0000000000004125e-290, 1.0), // lo
    (111963.22634951196, 100000.0, 1.0, 1.0000000000003426e-290), // up
    (88828.387643754642, 100000.0, 1.0000000000002032e-295, 1.0), // lo
    (112070.46283957762, 100000.0, 1.0, 1.0000000000005444e-295), // up
    (88737.3279114217, 100000.0, 1.0000000000000237e-300, 1.0), // lo
    (112176.85724295325, 100000.0, 1.0, 1.0000000000005922e-300), // up
    (88647.089311527074, 100000.0, 1.0000000000006399e-305, 1.0), // lo
    (112282.43062216583, 100000.0, 1.0, 1.0000000000007536e-305), // up
    (88593.331855179931, 100000.0, 9.999999999997593e-309, 1.0), // lo
    (112345.38899552909, 100000.0, 1.0, 1.0000000000004411e-308), // up
    (88522.096458751344, 100000.0, 9.9999999999846534e-313, 1.0), // lo
    (112428.8923363459, 100000.0, 1.0, 9.9999999999846534e-313), // up
    (88451.356405906146, 100000.0, 9.9999998365971443e-317, 1.0), // lo
    (112511.90039384722, 100000.0, 1.0, 9.9999998365971443e-317), // up
    (88381.102192901817, 100000.0, 9.9998886718268301e-321, 1.0), // lo
    (112594.42268375172, 100000.0, 1.0, 9.9998886718268301e-321), // up
    (964641.7911315955, 1000000.0, 9.999999999981942e-281, 1.0), // lo
    (1036211.1655328015, 1000000.0, 1.0, 1.0000000000015793e-280), // up
    (964019.75012513099, 1000000.0, 9.9999999999848417e-291, 1.0), // lo
    (1036863.8832824981, 1000000.0, 1.0, 9.9999999999927865e-291), // up
    (963712.86908329639, 1000000.0, 9.9999999999963479e-296, 1.0), // lo
    (1037186.1029875163, 1000000.0, 1.0, 1.0000000000003171e-295), // up
    (963408.65393986576, 1000000.0, 1.0000000000021837e-300, 1.0), // lo
    (1037505.6569794014, 1000000.0, 1.0, 1.0000000000016152e-300), // up
    (963107.03805306577, 1000000.0, 9.9999999999984405e-306, 1.0), // lo
    (1037822.6118935209, 1000000.0, 1.0, 9.9999999999813883e-306), // up
    (962927.28946805408, 1000000.0, 9.999999999997593e-309, 1.0), // lo
    (1038011.5639782328, 1000000.0, 1.0, 9.9999999999907699e-309), // up
    (962689.02136879007, 1000000.0, 9.9999999999846534e-313, 1.0), // lo
    (1038262.1035049941, 1000000.0, 1.0, 9.9999999999846534e-313), // up
    (962452.3213285706, 1000000.0, 9.9999998365971443e-317, 1.0), // lo
    (1038511.0750781563, 1000000.0, 1.0, 9.9999998365971443e-317), // up
    (962217.15927049809, 1000000.0, 9.9998886718268301e-321, 1.0), // lo
    (1038758.5087865973, 1000000.0, 1.0, 9.9998886718268301e-321), // up
];

// `pchisq(·, df, lower.tail = FALSE)` at df ∈ {2, 30, 490, 1e3, 1e4,
// 1e5, 1e6} — the `a ≥ 1` upper tail through R's own `pgamma_raw`
// branches (`d·sum` and `ppois_asymp`), including the review's two
// misses at df = 1e6, x = 1.002e6/1.005e6 and deep tails down to
// underflow. Command:
//
//   Rscript -e 'for (df in c(2, 30, 490, 1e3, 1e4, 1e5, 1e6))
//     for (r in c(0.5, 0.8, 0.9, 0.95, 0.99, 1.0, 1.001, 1.002, 1.005, 1.01, 1.05, 1.1, 1.5, 2, 5, 10)) {
//       x <- df * r
//       cat(sprintf("%.17g %.17g %.17g\n", x, df, pchisq(x, df, lower.tail = FALSE)))
//     }'
//
// Verbatim output (x df expected — each row machine-converted into
// CHISQ_SF_LARGE_DF_REF in order):
//
//   1 2 0.60653065971263342
//   1.6000000000000001 2 0.44932896411722162
//   1.8 2 0.40656965974059911
//   1.8999999999999999 2 0.38674102345450123
//   1.98 2 0.37157669102204577
//   2 2 0.36787944117144233
//   2.0019999999999998 2 0.3675117456086936
//   2.004 2 0.36714441755772104
//   2.0099999999999998 2 0.36604463480401539
//   2.02 2 0.36421897957152333
//   2.1000000000000001 2 0.34993774911115533
//   2.2000000000000002 2 0.33287108369807955
//   3 2 0.22313016014842982
//   4 2 0.1353352832366127
//   10 2 0.006737946999085467
//   20 2 4.5399929762484854e-05
//   15 30 0.98973957208765739
//   24 30 0.7720245323035444
//   27 30 0.62327112540873042
//   28.5 30 0.54395895398522964
//   29.699999999999999 30 0.48109254291037656
//   30 30 0.46565370894400965
//   30.029999999999998 30 0.46411794253778554
//   30.059999999999999 30 0.4625837325825925
//   30.149999999999999 30 0.45799063904173615
//   30.300000000000001 30 0.45036844045096575
//   31.5 30 0.39113496740655801
//   33 30 0.32254209693456487
//   45 30 0.038601758266317329
//   60 30 0.00092068239614866631
//   150 30 6.7069525239379602e-18
//   300 30 2.6480484856308771e-46
//   245 490 1
//   392 490 0.99958986855840681
//   441 490 0.9451134677202867
//   465.5 490 0.78073373296619208
//   485.10000000000002 490 0.55398413854555317
//   490 490 0.49150397998258166
//   490.48999999999995 490 0.48526504281389055
//   490.98000000000002 490 0.47903386012875621
//   492.44999999999993 490 0.46040183040451388
//   494.89999999999998 490 0.42964174699291158
//   514.5 490 0.21442578413449942
//   539 490 0.062141101575264618
//   735 490 4.3488045139502043e-12
//   980 490 5.6609715625348496e-35
//   2450 490 2.770928753367236e-257
//   4900 490 0
//   500 1000 1
//   800 1000 0.99999918906189211
//   900 1000 0.98928276190871023
//   950 1000 0.86912406574568846
//   990 1000 0.58294614635514874
//   1000 1000 0.49405285382923969
//   1000.9999999999999 1000 0.48513891876805254
//   1002 1000 0.47623832995729865
//   1004.9999999999999 1000 0.44966046079982591
//   1010 1000 0.40603315094773595
//   1050 1000 0.13247405682147709
//   1100 1000 0.014614408126295201
//   1500 1000 1.0454640385979652e-22
//   2000 1000 4.1436785914549906e-69
//   5000 1000 0
//   10000 1000 0
//   5000 10000 1
//   8000 10000 1
//   9000 10000 0.99999999999987299
//   9500 10000 0.99983561383564123
//   9900 10000 0.75952008568397933
//   10000 10000 0.49811936596618267
//   10009.999999999998 10000 0.46994791261915303
//   10020 10000 0.44194472105365762
//   10049.999999999998 10000 0.36029216645090567
//   10100 10000 0.23901532524017516
//   10500 10000 0.00024794736798936065
//   11000 10000 3.6183295580964844e-12
//   15000 10000 5.91586882905196e-208
//   20000 10000 0
//   50000 10000 0
//   100000 10000 0
//   50000 100000 1
//   80000 100000 1
//   90000 100000 1
//   95000 100000 1
//   99000 100000 0.98752168436191723
//   100000 100000 0.49940529189520672
//   100099.99999999999 100000 0.41098073201216101
//   100200 100000 0.32693001062380461
//   100499.99999999999 100000 0.13185481160339071
//   101000 100000 0.012868840377233664
//   105000 100000 1.8950787187407648e-28
//   110000.00000000001 100000 2.5841772471780533e-104
//   150000 100000 0
//   200000 100000 0
//   500000 100000 0
//   1000000 100000 0
//   500000 1000000 1
//   800000 1000000 1
//   900000 1000000 1
//   950000 1000000 1
//   990000 1000000 0.99999999999934996
//   1000000 1000000 0.49981193680339453
//   1000999.9999999999 1000000 0.23967680482554687
//   1002000 1000000 0.078718661386129637
//   1004999.9999999999 1000000 0.00020767800946709449
//   1010000 1000000 9.0685288232620253e-13
//   1050000 1000000 2.1856384174895054e-265
//   1100000 1000000 0
//   1500000 1000000 0
//   2000000 1000000 0
//   5000000 1000000 0
//   10000000 1000000 0

#[rustfmt::skip]
#[allow(clippy::unreadable_literal, clippy::excessive_precision)]
// verbatim machine-converted R `%.17g` output
const CHISQ_SF_LARGE_DF_REF: [(f64, f64, f64); 112] = [
    (1.0, 2.0, 0.60653065971263342),
    (1.6000000000000001, 2.0, 0.44932896411722162),
    (1.8, 2.0, 0.40656965974059911),
    (1.8999999999999999, 2.0, 0.38674102345450123),
    (1.98, 2.0, 0.37157669102204577),
    (2.0, 2.0, 0.36787944117144233),
    (2.0019999999999998, 2.0, 0.3675117456086936),
    (2.004, 2.0, 0.36714441755772104),
    (2.0099999999999998, 2.0, 0.36604463480401539),
    (2.02, 2.0, 0.36421897957152333),
    (2.1000000000000001, 2.0, 0.34993774911115533),
    (2.2000000000000002, 2.0, 0.33287108369807955),
    (3.0, 2.0, 0.22313016014842982),
    (4.0, 2.0, 0.1353352832366127),
    (10.0, 2.0, 0.006737946999085467),
    (20.0, 2.0, 4.5399929762484854e-05),
    (15.0, 30.0, 0.98973957208765739),
    (24.0, 30.0, 0.7720245323035444),
    (27.0, 30.0, 0.62327112540873042),
    (28.5, 30.0, 0.54395895398522964),
    (29.699999999999999, 30.0, 0.48109254291037656),
    (30.0, 30.0, 0.46565370894400965),
    (30.029999999999998, 30.0, 0.46411794253778554),
    (30.059999999999999, 30.0, 0.4625837325825925),
    (30.149999999999999, 30.0, 0.45799063904173615),
    (30.300000000000001, 30.0, 0.45036844045096575),
    (31.5, 30.0, 0.39113496740655801),
    (33.0, 30.0, 0.32254209693456487),
    (45.0, 30.0, 0.038601758266317329),
    (60.0, 30.0, 0.00092068239614866631),
    (150.0, 30.0, 6.7069525239379602e-18),
    (300.0, 30.0, 2.6480484856308771e-46),
    (245.0, 490.0, 1.0),
    (392.0, 490.0, 0.99958986855840681),
    (441.0, 490.0, 0.9451134677202867),
    (465.5, 490.0, 0.78073373296619208),
    (485.10000000000002, 490.0, 0.55398413854555317),
    (490.0, 490.0, 0.49150397998258166),
    (490.48999999999995, 490.0, 0.48526504281389055),
    (490.98000000000002, 490.0, 0.47903386012875621),
    (492.44999999999993, 490.0, 0.46040183040451388),
    (494.89999999999998, 490.0, 0.42964174699291158),
    (514.5, 490.0, 0.21442578413449942),
    (539.0, 490.0, 0.062141101575264618),
    (735.0, 490.0, 4.3488045139502043e-12),
    (980.0, 490.0, 5.6609715625348496e-35),
    (2450.0, 490.0, 2.770928753367236e-257),
    (4900.0, 490.0, 0.0),
    (500.0, 1000.0, 1.0),
    (800.0, 1000.0, 0.99999918906189211),
    (900.0, 1000.0, 0.98928276190871023),
    (950.0, 1000.0, 0.86912406574568846),
    (990.0, 1000.0, 0.58294614635514874),
    (1000.0, 1000.0, 0.49405285382923969),
    (1000.9999999999999, 1000.0, 0.48513891876805254),
    (1002.0, 1000.0, 0.47623832995729865),
    (1004.9999999999999, 1000.0, 0.44966046079982591),
    (1010.0, 1000.0, 0.40603315094773595),
    (1050.0, 1000.0, 0.13247405682147709),
    (1100.0, 1000.0, 0.014614408126295201),
    (1500.0, 1000.0, 1.0454640385979652e-22),
    (2000.0, 1000.0, 4.1436785914549906e-69),
    (5000.0, 1000.0, 0.0),
    (10000.0, 1000.0, 0.0),
    (5000.0, 10000.0, 1.0),
    (8000.0, 10000.0, 1.0),
    (9000.0, 10000.0, 0.99999999999987299),
    (9500.0, 10000.0, 0.99983561383564123),
    (9900.0, 10000.0, 0.75952008568397933),
    (10000.0, 10000.0, 0.49811936596618267),
    (10009.999999999998, 10000.0, 0.46994791261915303),
    (10020.0, 10000.0, 0.44194472105365762),
    (10049.999999999998, 10000.0, 0.36029216645090567),
    (10100.0, 10000.0, 0.23901532524017516),
    (10500.0, 10000.0, 0.00024794736798936065),
    (11000.0, 10000.0, 3.6183295580964844e-12),
    (15000.0, 10000.0, 5.91586882905196e-208),
    (20000.0, 10000.0, 0.0),
    (50000.0, 10000.0, 0.0),
    (100000.0, 10000.0, 0.0),
    (50000.0, 100000.0, 1.0),
    (80000.0, 100000.0, 1.0),
    (90000.0, 100000.0, 1.0),
    (95000.0, 100000.0, 1.0),
    (99000.0, 100000.0, 0.98752168436191723),
    (100000.0, 100000.0, 0.49940529189520672),
    (100099.99999999999, 100000.0, 0.41098073201216101),
    (100200.0, 100000.0, 0.32693001062380461),
    (100499.99999999999, 100000.0, 0.13185481160339071),
    (101000.0, 100000.0, 0.012868840377233664),
    (105000.0, 100000.0, 1.8950787187407648e-28),
    (110000.00000000001, 100000.0, 2.5841772471780533e-104),
    (150000.0, 100000.0, 0.0),
    (200000.0, 100000.0, 0.0),
    (500000.0, 100000.0, 0.0),
    (1000000.0, 100000.0, 0.0),
    (500000.0, 1000000.0, 1.0),
    (800000.0, 1000000.0, 1.0),
    (900000.0, 1000000.0, 1.0),
    (950000.0, 1000000.0, 1.0),
    (990000.0, 1000000.0, 0.99999999999934996),
    (1000000.0, 1000000.0, 0.49981193680339453),
    (1000999.9999999999, 1000000.0, 0.23967680482554687),
    (1002000.0, 1000000.0, 0.078718661386129637),
    (1004999.9999999999, 1000000.0, 0.00020767800946709449),
    (1010000.0, 1000000.0, 9.0685288232620253e-13),
    (1050000.0, 1000000.0, 2.1856384174895054e-265),
    (1100000.0, 1000000.0, 0.0),
    (1500000.0, 1000000.0, 0.0),
    (2000000.0, 1000000.0, 0.0),
    (5000000.0, 1000000.0, 0.0),
    (10000000.0, 1000000.0, 0.0),
];

#[test]
fn gamma_denormal_matches_reference() {
    for (x, a, e_lower, e_upper) in PGAMMA_DENORMAL_REF {
        ref_check(
            e_lower,
            chisq_cdf(2.0 * x, 2.0 * a),
            1e-9,
            format!("pgamma({x}, {a})"),
        );
        ref_check(
            e_upper,
            chisq_sf(2.0 * x, 2.0 * a),
            1e-9,
            format!("pgamma({x}, {a}, upper)"),
        );
    }
}

#[test]
fn gamma_upper_large_df_matches_reference() {
    for (x, df, e_sf) in CHISQ_SF_LARGE_DF_REF {
        ref_check(e_sf, chisq_sf(x, df), 1e-9, format!("chisq_sf({x}, {df})"));
    }
}

proptest! {
    /// P and Q stay in [0, 1] and P + Q ≈ 1 where both are ≥ 1e-3 —
    /// `pgamma_smallx`'s `f1·f2` lower form keeps tiny-a P below 1
    /// where the convergent series exceeded it (review OPTIONAL item).
    #[test]
    fn inc_gamma_stays_in_unit_interval(
        a in 1e-300..1e7f64,
        r in 0.0..1e4f64,
        x0 in 0.0..1e8f64,
    ) {
        for x in [a * r, x0] {
            let p = chisq_cdf(2.0 * x, 2.0 * a);
            let q = chisq_sf(2.0 * x, 2.0 * a);
            prop_assert!((0.0..=1.0).contains(&p), "P({a}, {x}) = {p}");
            prop_assert!((0.0..=1.0).contains(&q), "Q({a}, {x}) = {q}");
            if p >= 1e-3 && q >= 1e-3 {
                prop_assert!(
                    (p + q - 1.0).abs() <= 1e-12,
                    "P + Q at ({a}, {x}) = {}",
                    p + q
                );
            }
        }
    }
}
