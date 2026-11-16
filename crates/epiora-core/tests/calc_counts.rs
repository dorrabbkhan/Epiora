#![cfg(feature = "corpus")]

//! Counts-family calculator fixture tests (v1.0 Task 7). The corpus
//! `calc.<id>` fixtures under `fixtures/calc/` are the oracle (Task 2).

use epiora_core::calc::run;
use epiora_core::corpus::compare::compare;
use epiora_core::corpus::run_case::run_case;
use epiora_core::dataset::Dataset;
use epiora_core::procedures::RunContext;
use epiora_core::stats::Estimate;
use proptest::prelude::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project: None,
    }
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

const COUNTS: &[&str] = &[
    "proportion",
    "two_by_two",
    "two_by_n",
    "r_by_c",
    "dose_response",
    "matched_case_control",
    "screening",
    "smr",
    "binomial",
    "poisson",
];

#[test]
fn every_counts_fixture_matches() {
    let datasets: BTreeMap<String, Dataset> = BTreeMap::new();
    let mut problems = Vec::new();
    for id in COUNTS {
        let dir = fixtures_root().join("calc").join(id);
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{dir:?}: {e}"))
            .map(|e| e.expect("dir entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        files.sort();
        assert!(!files.is_empty(), "{dir:?} has no fixtures");
        for file in files {
            let fixture: Value =
                serde_json::from_str(&std::fs::read_to_string(&file).expect("read fixture"))
                    .expect("parse fixture");
            assert_eq!(
                fixture["procedure"].as_str(),
                Some(format!("calc.{id}").as_str())
            );
            match run_case(&fixture, &datasets) {
                Err(e) => problems.push(format!("{}: run_case error {e}", file.display())),
                Ok(out) => match compare(&fixture, &out.actual) {
                    Err(e) => problems.push(format!("{}: compare error {e}", file.display())),
                    Ok(mismatches) => {
                        for m in mismatches {
                            problems.push(format!("{}: {m:?}", file.display()));
                        }
                    }
                },
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The single-table calculator path shares the v0.2 `stats` engine:
/// the same counts produce the same numbers under both entry points.
#[test]
fn two_by_two_calculator_equals_tables_procedure() {
    let out = run(
        "two_by_two",
        &json!({"strata": [[12, 4], [3, 9]], "conf_level": 95}),
        &ctx(),
    )
    .expect("calc two_by_two");
    let stats = epiora_core::stats::two_by_two(&epiora_core::stats::Table2x2 {
        a: 12,
        b: 4,
        c: 3,
        d: 9,
    });
    let stats_json = serde_json::to_value(&stats).expect("serialize stats");
    let block = &out.result["crude"];
    let pairs: &[(&[&str], &[&str])] = &[
        (
            &["chisq", "uncorrected", "value"],
            &["chi_square", "uncorrected", "statistic"],
        ),
        (
            &["chisq", "yates", "value"],
            &["chi_square", "yates", "statistic"],
        ),
        (&["odds_ratio", "est"], &["odds_ratio", "estimate"]),
    ];
    for (calc_path, stats_path) in pairs {
        let mut cv = block;
        for k in *calc_path {
            cv = &cv[k];
        }
        let mut sv = &stats_json;
        for k in *stats_path {
            sv = &sv[k];
        }
        let (c, s) = (cv.as_f64(), sv.as_f64());
        match (c, s) {
            (Some(c), Some(s)) => assert!(
                (c - s).abs() <= 1e-9 * s.abs().max(1.0),
                "{calc_path:?} vs {stats_path:?}: {c} vs {s}"
            ),
            _ => assert_eq!(c, s, "{calc_path:?} vs {stats_path:?}"),
        }
    }
    let wald = &stats.odds_ratio.ci_95_wald;
    let expected = [&wald.lower, &wald.upper].map(|e| match e {
        Estimate::Value(v) => Some(*v),
        _ => None,
    });
    let actual = [
        out.result["crude"]["odds_ratio"]["lower"].as_f64(),
        out.result["crude"]["odds_ratio"]["upper"].as_f64(),
    ];
    for (a, e) in actual.iter().zip(expected.iter()) {
        match (a, e) {
            (Some(a), Some(e)) => assert!((a - e).abs() <= 1e-9 * e.abs().max(1.0)),
            _ => assert_eq!(a, e),
        }
    }
}

fn leaf_number(v: &Value) -> Option<f64> {
    v.as_f64()
}

/// The leaf keys that carry a probability or a p-value: `p`,
/// `*_p`, `*p_1t`/`*p_2t` (covers `p_1t`, `midp_2t`,
/// `fisher_p_1t`, …) and the binomial/Poisson `tail` masses.
fn is_probability_key(key: &str) -> bool {
    key == "p"
        || key.ends_with("_p")
        || key.ends_with("p_1t")
        || key.ends_with("p_2t")
        || matches!(key, "lt" | "le" | "ge" | "gt" | "eq")
}

/// Every leaf is a finite number or an `{"undefined"}` object (a
/// bare `null` means a NaN escaped into `Value::from`), every
/// probability/p-value leaf lands in `[0, 1]`, and every
/// `lower`/`upper` pair is ordered — Review Focus 3 (fix round 2):
/// the `ln_gamma`-composed masses once emitted 3e21 and −6.6e7.
fn assert_value_tree(v: &Value, id: &str) {
    match v {
        Value::Null => panic!("{id}: null leaf — a NaN reached the result tree"),
        Value::Number(n) => assert!(n.as_f64().is_some_and(f64::is_finite)),
        Value::Array(items) => items.iter().for_each(|i| assert_value_tree(i, id)),
        Value::Object(map) => {
            for (k, i) in map {
                if is_probability_key(k) {
                    if let Some(x) = i.as_f64() {
                        assert!(
                            (0.0..=1.0).contains(&x),
                            "{id}: probability leaf {k} = {x} outside [0, 1]"
                        );
                    }
                }
                assert_value_tree(i, id);
            }
            if let (Some(l), Some(u)) = (
                map.get("lower").and_then(Value::as_f64),
                map.get("upper").and_then(Value::as_f64),
            ) {
                assert!(l <= u, "{id}: CI lower {l} > upper {u}");
            }
        }
        Value::String(_) | Value::Bool(_) => {}
    }
}

/// Review Focus 1/3 (PR #74 fix): valid counts below 2^53 must
/// return `Ok` or a typed `Err`, never hang — the O(n) literal
/// binomial sums now go through `pbinom` (R's `pbeta` identity on
/// the regularised incomplete beta) and `matched_n` aggregates per
/// stratum shape. Sizes 1e6, 1e9 and 2^53−1 for every calculator.
/// `two_by_two` and `two_by_n` now finish at every size too: the
/// owner-decision `too_large` guards (PR #74 comment) keep the
/// noncentral-hypergeometric support sums and convolutions under
/// their caps, so the exact-path leaves go Undefined while the
/// closed-form leaves still compute.
#[test]
fn large_counts_terminate() {
    let sizes = [1_000_000u64, 1_000_000_000, (1u64 << 53) - 1];
    let check = |id: &str, inputs: Value| {
        let t = std::time::Instant::now();
        let res = run(id, &inputs, &ctx());
        eprintln!("{id} {:?} in {:?}", inputs, t.elapsed());
        if let Ok(out) = res {
            assert_value_tree(&out.result, id);
        }
    };
    for &s in &sizes {
        check(
            "proportion",
            json!({"numerator": s - 1, "denominator": s, "conf_level": 95,
                   "population_size": 0}),
        );
        check(
            "binomial",
            json!({"numerator": s / 2, "denominator": s, "expected_percent": 50}),
        );
        check("poisson", json!({"observed": s, "expected": 42.5}));
        check(
            "smr",
            json!({"observed": s, "expected": 42.5, "conf_level": 95}),
        );
        check("r_by_c", json!({"table": [[s, s], [s, s]]}));
        check(
            "dose_response",
            json!({"table": [[[1.0, s, s], [2.0, s, s]],
                            [[1.0, s, s], [3.0, s, s]]], "strata": 2}),
        );
        check(
            "screening",
            json!({"table": [[s, s], [s, s]], "conf_level": 95}),
        );
        check(
            "matched_case_control",
            json!({"controls_per_case": 1,
                   "matched_sets": [[s, s / 2], [s / 3, s]],
                   "conf_level": 95}),
        );
        check(
            "matched_case_control",
            json!({"controls_per_case": 4,
                   "matched_sets": [[s, s, s, s, s], [s, s, s, s, s]],
                   "conf_level": 95}),
        );
        // Hypergeometric-support paths: support 1e6 computes, above
        // the owner caps the exact-path leaves turn `too_large` and
        // everything else still computes — so 1e9 and 2^53−1 finish.
        check(
            "two_by_two",
            json!({"strata": [[[s / 2, s / 2], [s / 2, s / 2]]], "conf_level": 95}),
        );
        check(
            "two_by_n",
            json!({"strata": [[[s / 2, s / 2], [s / 2, s / 2]]], "conf_level": 95}),
        );
    }
    // Multi-stratum `two_by_two` additionally runs `martin`'s
    // O(support₁·support₂) log-space convolution for the `adjusted`
    // block; the work cap is 1e8, so two strata of 2,000 per cell
    // (support 4,001 each, work 1.6e7) run the path end to end.
    let s2 = 2_000u64;
    check(
        "two_by_two",
        json!({"strata": [[[s2, s2], [s2, s2]], [[s2, s2], [s2, s2]]],
               "conf_level": 95}),
    );
    // `matched_n` at ~1e12 sets — the shapes carry multiplicities, so
    // this returns in O(M) memory where per-set strata needed O(N).
    let e11 = 100_000_000_000u64;
    check(
        "matched_case_control",
        json!({"controls_per_case": 4,
               "matched_sets": [[4 * e11, 3 * e11, 2 * e11, e11, 0],
                                [0, e11, 2 * e11, 3 * e11, 3 * e11]],
               "conf_level": 95}),
    );
    // The aggregated sums are the same quantities: the MH OR is
    // scale-invariant, so the 1e12-set table's estimate matches the
    // identical shape at unit multiplicity.
    let small = run(
        "matched_case_control",
        &json!({"controls_per_case": 4,
                "matched_sets": [[4, 3, 2, 1, 0], [0, 1, 2, 3, 3]],
                "conf_level": 95}),
        &ctx(),
    )
    .expect("matched_n unit");
    let big = run(
        "matched_case_control",
        &json!({"controls_per_case": 4,
                "matched_sets": [[4 * e11, 3 * e11, 2 * e11, e11, 0],
                                 [0, e11, 2 * e11, 3 * e11, 3 * e11]],
                "conf_level": 95}),
        &ctx(),
    )
    .expect("matched_n 1e12");
    let (a, b) = (
        small.result["matched_n"]["mh_or"]["est"].as_f64(),
        big.result["matched_n"]["mh_or"]["est"].as_f64(),
    );
    match (a, b) {
        (Some(a), Some(b)) => {
            assert!((a - b).abs() <= 1e-12 * b.abs().max(1.0), "{a} vs {b}")
        }
        _ => assert_eq!(a, b),
    }
}

/// A `{"undefined": code}` leaf's reason code, or `None` for a number.
fn undefined_code(v: &Value) -> Option<String> {
    v.get("undefined")
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// v1.0 Task 7 follow-up (owner decision, PR #74 comment): the
/// exact-path leaves of `two_by_two`/`two_by_n` are
/// `{"undefined": "too_large"}` when the per-stratum hypergeometric
/// support exceeds 1_000_000 or the martin convolution work exceeds
/// 1e8; every closed-form leaf still computes.
#[test]
fn exact_paths_too_large_above_owner_limits() {
    // Support of cell a = min(n1, m1) − max(0, n1−m0) + 1.
    // a=0, b=c=999_999, d=1 → n1=m1=999_999, m0=1e6 → support
    // exactly 1_000_000 → computes.
    let at_cap = json!({"strata": [[[0, 999_999], [999_999, 1]]], "conf_level": 95});
    // a=0, b=c=1_000_000, d=1 → support 1_000_001 → too_large.
    let over_cap = json!({"strata": [[[0, 1_000_000], [1_000_000, 1]]], "conf_level": 95});

    let out = run("two_by_two", &at_cap, &ctx()).expect("two_by_two at cap");
    let block = &out.result["strata"][0];
    assert!(block["fisher_exact"]["p_1t"].is_number());
    assert!(block["fisher_exact"]["p_2t"].is_number());
    assert!(block["midp_exact"]["p_1t"].is_number());
    assert!(block["midp_exact"]["p_2t"].is_number());
    assert!(block["cmle_or"]["midp"]["est"].is_number());
    assert!(block["cmle_or"]["midp"]["lower"].is_number());
    assert!(block["cmle_or"]["midp"]["upper"].is_number());
    assert!(block["cmle_or"]["fisher"]["lower"].is_number());
    assert!(block["cmle_or"]["fisher"]["upper"].is_number());

    // two_by_n at the cap.
    let out = run("two_by_n", &at_cap, &ctx()).expect("two_by_n at cap");
    let block = &out.result["strata"][0];
    assert!(block["exact"]["midp_2t"].is_number());
    assert!(block["exact"]["fisher_1t"].is_number());
    assert!(block["exact"]["fisher_2t"].is_number());
    assert!(block["odds_ratio_mle"]["midp"]["est"].is_number());
    assert!(block["odds_ratio_mle"]["midp"]["lower"].is_number());
    assert!(block["odds_ratio_mle"]["midp"]["upper"].is_number());
    assert!(block["odds_ratio_mle"]["fisher"]["lower"].is_number());
    assert!(block["odds_ratio_mle"]["fisher"]["upper"].is_number());

    // Over the cap: exactly the exact-path leaves are too_large; the
    // chi-square, Wald, risk and etiologic leaves stay finite.
    let out = run("two_by_two", &over_cap, &ctx()).expect("two_by_two over cap");
    for path in [&out.result["strata"][0], &out.result["crude"]] {
        let exact_leaves = [
            &path["fisher_exact"]["p_1t"],
            &path["fisher_exact"]["p_2t"],
            &path["midp_exact"]["p_1t"],
            &path["midp_exact"]["p_2t"],
            &path["cmle_or"]["midp"]["est"],
            &path["cmle_or"]["midp"]["lower"],
            &path["cmle_or"]["midp"]["upper"],
            &path["cmle_or"]["fisher"]["lower"],
            &path["cmle_or"]["fisher"]["upper"],
        ];
        for leaf_v in exact_leaves {
            assert_eq!(
                undefined_code(leaf_v).as_deref(),
                Some("too_large"),
                "expected too_large, got {leaf_v}"
            );
        }
        for leaf_v in [
            &path["chisq"]["uncorrected"]["value"],
            &path["chisq"]["yates"]["p_2t"],
            &path["chisq"]["mantel_haenszel"]["p_1t"],
            &path["risk"]["exposed"]["est"],
            &path["risk_ratio"]["est"],
            &path["risk_difference"]["est"],
            &path["odds_ratio"]["est"],
        ] {
            assert!(leaf_v.is_number(), "expected finite, got {leaf_v}");
        }
    }

    // two_by_n over the cap: `exact` and `odds_ratio_mle` too_large.
    let out = run("two_by_n", &over_cap, &ctx()).expect("two_by_n over cap");
    for block in [&out.result["strata"][0], &out.result["crude"]] {
        for leaf_v in [
            &block["exact"]["midp_2t"],
            &block["exact"]["fisher_1t"],
            &block["exact"]["fisher_2t"],
            &block["odds_ratio_mle"]["midp"]["est"],
            &block["odds_ratio_mle"]["midp"]["lower"],
            &block["odds_ratio_mle"]["midp"]["upper"],
            &block["odds_ratio_mle"]["fisher"]["lower"],
            &block["odds_ratio_mle"]["fisher"]["upper"],
        ] {
            assert_eq!(
                undefined_code(leaf_v).as_deref(),
                Some("too_large"),
                "expected too_large, got {leaf_v}"
            );
        }
        assert!(block["odds_ratio"]["est"].is_number());
        assert!(block["risk_ratio"]["est"].is_number());
        assert!(block["chisq"]["uncorrected"]["value"].is_number());
    }

    // Stratified martin path: support₁·support₂ work cap 1e8.
    // a=1, b=c=n−2, d=2 → support n; 10_000·10_000 = 1e8 computes,
    // 10_000·10_001 = 100_010_000 is too_large on the adjusted
    // cmle_or leaves only.
    let strat = |n: u64| [[1, n - 2], [n - 2, 2]];
    let at_conv = json!({"strata": [strat(10_000), strat(10_000)], "conf_level": 95});
    let over_conv = json!({"strata": [strat(10_000), strat(10_001)], "conf_level": 95});
    let out = run("two_by_two", &at_conv, &ctx()).expect("two_by_two at conv cap");
    assert!(
        out.result["adjusted"]["cmle_or"]["est"].is_number(),
        "adjusted.cmle_or.est should compute at 1e8 work"
    );
    assert!(out.result["adjusted"]["cmle_or"]["midp"]["lower"].is_number());
    assert!(out.result["adjusted"]["cmle_or"]["midp"]["upper"].is_number());
    assert!(out.result["adjusted"]["cmle_or"]["fisher"]["lower"].is_number());
    assert!(out.result["adjusted"]["cmle_or"]["fisher"]["upper"].is_number());

    let out = run("two_by_two", &over_conv, &ctx()).expect("two_by_two over conv cap");
    for leaf_v in [
        &out.result["adjusted"]["cmle_or"]["est"],
        &out.result["adjusted"]["cmle_or"]["midp"]["lower"],
        &out.result["adjusted"]["cmle_or"]["midp"]["upper"],
        &out.result["adjusted"]["cmle_or"]["fisher"]["lower"],
        &out.result["adjusted"]["cmle_or"]["fisher"]["upper"],
    ] {
        assert_eq!(
            undefined_code(leaf_v).as_deref(),
            Some("too_large"),
            "expected too_large, got {leaf_v}"
        );
    }
    // The rest of the adjusted block and each stratum's own exact
    // leaves still compute — neither support exceeds 1e6.
    assert!(out.result["adjusted"]["mh_or"]["est"].is_number());
    assert!(out.result["adjusted"]["mh_chisq"]["value"].is_number());
    assert!(out.result["adjusted"]["breslow_day"]["or_chisq"].is_number());
    assert!(out.result["strata"][1]["fisher_exact"]["p_1t"].is_number());

    // Counts at 2^53−1 return quickly: the support check precedes
    // the NcHypergeom allocation that used to abort.
    let s = (1u64 << 53) - 1;
    let huge = json!({"strata": [[[s, s], [s, s]]], "conf_level": 95});
    let t = std::time::Instant::now();
    let out = run("two_by_two", &huge, &ctx()).expect("two_by_two 2^53-1");
    assert_eq!(
        undefined_code(&out.result["crude"]["fisher_exact"]["p_1t"]).as_deref(),
        Some("too_large")
    );
    assert_eq!(
        undefined_code(&out.result["crude"]["cmle_or"]["midp"]["est"]).as_deref(),
        Some("too_large")
    );
    assert!(out.result["crude"]["odds_ratio"]["est"].is_number());
    eprintln!("two_by_two 2^53-1 in {:?}", t.elapsed());
    assert_value_tree(&out.result, "two_by_two");
}

/// v1.0 Task 7 follow-up (D3): `matched_n.cmle_or` is the conditional
/// MLE of `survival::clogit` — one stratum per matched set, Wald CI
/// on the coefficient with the observed-information SE. The expected
/// numbers are the PR #75 regenerated corpus fixtures.
#[test]
fn matched_n_cmle_matches_clogit() {
    let cases: [(u64, [[u64; 5]; 2], [f64; 3]); 2] = [
        (
            2,
            [[20, 10, 4, 0, 0], [10, 8, 12, 0, 0]],
            [1.7213892162463522, 0.9480759477623332, 3.125467786418363],
        ),
        (
            4,
            [[12, 8, 5, 3, 2], [15, 10, 7, 4, 2]],
            [
                2.1750734364802318,
                1.195_404_057_780_931,
                3.9576111719615006,
            ],
        ),
    ];
    for (m, sets, want) in cases {
        let trimmed: Vec<Vec<u64>> = sets.iter().map(|r| r[..=(m as usize)].to_vec()).collect();
        let out = run(
            "matched_case_control",
            &json!({"controls_per_case": m, "matched_sets": trimmed,
                    "conf_level": 95}),
            &ctx(),
        )
        .expect("matched_case_control run");
        for (key, w) in ["est", "lower", "upper"].iter().zip(want) {
            let got = out.result["matched_n"]["cmle_or"][key].as_f64();
            assert!(
                got.is_some_and(|g| (g - w).abs() <= 1e-6 * w.abs()),
                "m={m} cmle_or.{key}: {got:?} vs {w}"
            );
        }
    }

    // Boundary cases — the reason the engine uses for an unbounded or
    // undefined MLE is `unbounded` (stats::exact's reason at the
    // support edge). clogit on these inputs returns NA coef for a
    // flat likelihood and a diverging coefficient ("beta may be
    // infinite") for one-way discordance; both are Undefined here.
    for (name, sets) in [
        ("flat", json!([[0, 0, 3], [4, 0, 0]])), // no informative sets
        ("up only", json!([[0, 5, 0], [0, 0, 0]])), // all discordance up
        ("down only", json!([[0, 0, 0], [0, 0, 7]])), // all discordance down
    ] {
        let out = run(
            "matched_case_control",
            &json!({"controls_per_case": 2, "matched_sets": sets,
                    "conf_level": 95}),
            &ctx(),
        )
        .unwrap_or_else(|e| panic!("{name}: {e}"));
        for key in ["est", "lower", "upper"] {
            assert_eq!(
                undefined_code(&out.result["matched_n"]["cmle_or"][key]).as_deref(),
                Some("unbounded"),
                "{name} cmle_or.{key}"
            );
        }
    }
}

/// R's `mcnemar.test(correct=TRUE)` subtracts the continuity 1 only
/// when the table differs from its transpose (`any(x - t(x) != 0)`):
/// `matrix(c(10, 5, 5, 10), 2)` — equal discordant counts — gives
/// statistic 0 and p = 1, not (0−1)²/n = 1/10 (verified against
/// R 4.6.2; fix round 2).
#[test]
fn mcnemar_cc_balanced_discordance_is_zero() {
    let out = run(
        "matched_case_control",
        &json!({"controls_per_case": 1,
                "matched_sets": [[10, 5], [5, 10]],
                "conf_level": 95}),
        &ctx(),
    )
    .expect("matched_case_control run");
    assert_eq!(out.result["mcnemar_cc"]["value"].as_f64(), Some(0.0));
    assert_eq!(out.result["mcnemar_cc"]["p"].as_f64(), Some(1.0));
}

/// v1.0 Task 8 follow-up (PR #78 fix round 1): `martin_root` must
/// converge where R's `uniroot` does. `brent_root`'s absolute-only
/// tolerance can never satisfy roots ≳ ~500; `special::zeroin`
/// (R_zeroin2, tol = 1e-14 — the tol `stratified_cmle_pt_one`/
/// `stratified_cmle_one` pass in `fixtures/R/methods`) carries the
/// relative term. Expected values: R 4.6.2, `fixtures/R/methods`
/// (renv), the exact command and output in the PR #78 body —
/// `stratified_cmle_pt_one(list(matrix(c(a, 1, 2e6, 1e6), 2, 2,
/// byrow = TRUE)), 0.95, 1e-14)` and `stratified_cmle_one(list(
/// matrix(c(1000, 1, 1, 1000), 2, 2, byrow = TRUE),
/// matrix(c(10, 5, 5, 10), 2, 2, byrow = TRUE)), 0.95, 1e-14)`.
/// Compared at the `iterative` tolerance class (E7).
#[test]
fn martin_cmle_large_roots_match_r() {
    let iterative = |got: Option<f64>, want: f64, path: &str| {
        assert!(
            got.is_some_and(|g| (g - want).abs() <= 1e-6 * want.abs()),
            "{path}: {got:?} vs {want}"
        );
    };
    // Person-time strata `[[m1−1, 1], [2e6, 1e6]]` — the review's
    // `out_of_range` cases at m1 = 100, 1e3 and 1e5.
    // Leaf order: est, midp_lower, midp_upper, fisher_lower,
    // fisher_upper.
    let pt_cases: [(f64, [f64; 5]); 3] = [
        (
            99.0,
            [
                49.499_999_999_999_12,
                9.847_714_539_196_751,
                999.105_105_316_891,
                8.681_153_925_998_533,
                1974.644520809587,
            ],
        ),
        (
            999.0,
            [
                499.500000191906,
                101.0896419972828,
                9_995.444_590_009_036,
                89.445_459_738_344_84,
                19_748.695_103_681_26,
            ],
        ),
        (
            99_999.0,
            [
                49_999.499_999_992_32,
                10137.869656678491,
                999_592.780_801_722_1,
                8_973.718_095_118_962,
                1974894.2601724067,
            ],
        ),
    ];
    for (a, want) in pt_cases {
        let out = run(
            "rate_compare",
            &json!({"strata": [[[a, 1.0], [2000000.0, 1000000.0]]],
                    "per_unit": 1000.0, "conf_level": 95}),
            &ctx(),
        )
        .unwrap_or_else(|e| panic!("rate_compare a={a}: {e}"));
        for (key, w) in [
            "est",
            "midp_lower",
            "midp_upper",
            "fisher_lower",
            "fisher_upper",
        ]
        .iter()
        .zip(want)
        {
            for block in [&out.result["strata"][0], &out.result["crude"]] {
                iterative(
                    block["cmle_rr"][key].as_f64(),
                    w,
                    &format!("rate_compare a={a} cmle_rr.{key}"),
                );
            }
        }
    }
    // m1 = 1.2e6 and 2e6 with a 1-count unexposed arm: the four
    // limits still match R, but R's `est` is an iteration-landing
    // artifact — `martin_func` has no zero there, only a quantized
    // residual plateau (R evidence: f(600131) < 0, f(600132) > 0,
    // f(600133) < 0) — so it is not an oracle value. The regression
    // is that `est` used to come back `out_of_range`; assert only
    // that it is now a finite `Value`. Leaf order: midp_lower,
    // midp_upper, fisher_lower, fisher_upper.
    let pt_extreme: [(f64, [f64; 4]); 2] = [
        (
            1_199_999.0,
            [
                121657.64973310455,
                11995118.717246097,
                107687.86024179005,
                23698733.868378676,
            ],
        ),
        (
            1_999_999.0,
            [
                202762.94508843697,
                19991864.874644913,
                179479.96557191815,
                39497890.062663056,
            ],
        ),
    ];
    for (a, want) in pt_extreme {
        let out = run(
            "rate_compare",
            &json!({"strata": [[[a, 1.0], [2000000.0, 1000000.0]]],
                    "per_unit": 1000.0, "conf_level": 95}),
            &ctx(),
        )
        .unwrap_or_else(|e| panic!("rate_compare a={a}: {e}"));
        for (key, w) in ["midp_lower", "midp_upper", "fisher_lower", "fisher_upper"]
            .iter()
            .zip(want)
        {
            for block in [&out.result["strata"][0], &out.result["crude"]] {
                iterative(
                    block["cmle_rr"][key].as_f64(),
                    w,
                    &format!("rate_compare a={a} cmle_rr.{key}"),
                );
            }
        }
        for block in [&out.result["strata"][0], &out.result["crude"]] {
            assert!(
                block["cmle_rr"]["est"].as_f64().is_some(),
                "rate_compare a={a} cmle_rr.est: {:?}",
                block["cmle_rr"]["est"],
            );
        }
    }
    // The same `martin_root` on the counts hypergeometric path:
    // `two_by_two` strata `[[1000,1],[1,1000]] + [[10,5],[5,10]]`,
    // the review's `adjusted.cmle_or` case (`two_by_n` has no
    // `adjusted` block).
    let out = run(
        "two_by_two",
        &json!({"strata": [[[1000, 1], [1, 1000]], [[10, 5], [5, 10]]],
                "conf_level": 95}),
        &ctx(),
    )
    .expect("two_by_two run");
    let cmle = &out.result["adjusted"]["cmle_or"];
    iterative(
        cmle["est"].as_f64(),
        25385.105906210898,
        "adjusted.cmle_or.est",
    );
    iterative(
        cmle["midp"]["lower"].as_f64(),
        8_814.709_199_503_686,
        "adjusted.cmle_or.midp.lower",
    );
    iterative(
        cmle["midp"]["upper"].as_f64(),
        86_976.376_286_180_47,
        "adjusted.cmle_or.midp.upper",
    );
    iterative(
        cmle["fisher"]["lower"].as_f64(),
        8_055.365_755_196_057,
        "adjusted.cmle_or.fisher.lower",
    );
    iterative(
        cmle["fisher"]["upper"].as_f64(),
        101317.88377482712,
        "adjusted.cmle_or.fisher.upper",
    );
}

proptest! {
    /// PPV and NPV estimates always land in [0, 1] — or are typed
    /// Undefined when the margins make them incalculable (spec §5.3).
    #[test]
    fn screening_ppv_npv_in_unit_interval(
        tp in 0u64..500,
        fp in 0u64..500,
        fneg in 0u64..500,
        tn in 0u64..500,
    ) {
        prop_assume!(tp + fp + fneg + tn > 0);
        let out = run(
            "screening",
            &json!({"table": [[tp, fneg], [fp, tn]], "conf_level": 95}),
            &ctx(),
        )
        .expect("screening run");
        for key in ["ppv", "npv"] {
            if let Some(v) = leaf_number(&out.result[key]["est"]) {
                prop_assert!(
                    (0.0..=1.0).contains(&v),
                    "{key} estimate {v} outside [0, 1]"
                );
            }
        }
    }
}
