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
