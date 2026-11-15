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
/// stratum shape. Sizes 1e6, 1e9 and 2^53−1 for every calculator
/// whose paths closed-form after the rewrite. `two_by_two` and
/// `two_by_n` keep the noncentral-hypergeometric support sums and
/// root-finding that have no closed form ("Still unbounded (owner
/// decision)" on the PR), so they are checked at 1e6 only — the
/// largest size that finishes.
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
    }
    // Hypergeometric-support paths — 1e6 is the largest size that
    // still finishes single-stratum (1e9 exceeds 60 s; 2^53−1 aborts
    // on the support allocation), so only it is exercised there.
    let s = 1_000_000u64;
    check(
        "two_by_two",
        json!({"strata": [[[s / 2, s / 2], [s / 2, s / 2]]], "conf_level": 95}),
    );
    check(
        "two_by_n",
        json!({"strata": [[[s / 2, s / 2], [s / 2, s / 2]]], "conf_level": 95}),
    );
    // Multi-stratum `two_by_two` additionally runs `martin`'s
    // O(support²) log-space convolution for the `adjusted` block, so
    // its bound is far lower than the single-stratum 1e6: measured
    // (release probe, `target/release/probe`) 40,000 counts per cell
    // finishes in 67 s and 50,000 per cell exceeds a 90 s cap; the
    // review's 60 s frame cuts between 30,000 (37 s) and 40,000.
    // The assertions below run 2 strata of 2,000 per cell so the
    // path's leaves are range-checked in debug-build time.
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
