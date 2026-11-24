#![cfg(feature = "corpus")]

//! Person-time calculator fixture tests (v1.0 Task 8). The corpus
//! `calc.<id>` fixtures under `fixtures/calc/` are the oracle
//! (Task 2).

use epiora_core::calc::run;
use epiora_core::corpus::compare::compare;
use epiora_core::corpus::run_case::run_case;
use epiora_core::dataset::Dataset;
use epiora_core::procedures::RunContext;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project: None,
        working_view: None,
    }
}

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

const PERSON_TIME: &[&str] = &["rate_one", "rate_compare"];

#[test]
fn every_person_time_fixture_matches() {
    let datasets: BTreeMap<String, Dataset> = BTreeMap::new();
    let mut problems = Vec::new();
    for id in PERSON_TIME {
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

/// Review Focus 3: zero observed events gives a lower confidence
/// limit of exactly 0 and a finite upper limit for every rate CI
/// method — except Rothman–Greenland, where R's `rg_ci` degenerates
/// to `exp(-Inf) = 0` / `NaN`, so the upper is `undefined` rather
/// than an invented point bound (`smr` behaves the same way).
#[test]
fn zero_events_rate_ci_defined_lower_zero() {
    let out = run(
        "rate_one",
        &json!({
            "cases": 0,
            "person_time": 1000.0,
            "per_unit": 1000.0,
            "conf_level": 95,
        }),
        &ctx(),
    )
    .expect("rate_one on zero events");
    assert_eq!(out.result["estimate"], json!(0.0));
    for method in ["midp", "fisher", "normal", "byar"] {
        let ci = &out.result["ci"][method];
        assert_eq!(ci["lower"], json!(0.0), "{method}.lower must be exactly 0");
        assert!(
            ci["upper"].as_f64().is_some_and(|x| x.is_finite()),
            "{method}.upper must be a finite value, got {}",
            ci["upper"]
        );
    }
    let rg = &out.result["ci"]["rothman_greenland"];
    assert_eq!(rg["lower"], json!(0.0), "rg.lower must be exactly 0");
    assert_eq!(
        rg["upper"],
        json!({ "undefined": "out_of_range" }),
        "rg.upper must be undefined at zero events, got {}",
        rg["upper"]
    );
}

/// Spec §5.3 / inventory `## rate_compare` inputs: strata cells are
/// typed — case counts are integers `0 ≤ n < 2^53`, person-time is
/// finite `> 0`, and the strata count stays within 1..9 — enforced as
/// `calc_input_out_of_range` errors, not computed on.
#[test]
fn rate_compare_rejects_bad_strata() {
    let stratum = json!([[10, 20], [1000, 2000]]);
    let cases: Vec<serde_json::Value> = vec![
        json!([[[-1, 20], [1000, 2000]]]),   // negative case count
        json!([[[10.5, 20], [1000, 2000]]]), // non-integer case count
        json!([[[10, 20], [0, 2000]]]),      // zero person-time
        json!([[[10, 20], [-100, 2000]]]),   // negative person-time
        json!(vec![stratum; 10]),            // 10 strata exceeds the cap
    ];
    for strata in cases {
        let err = run(
            "rate_compare",
            &json!({
                "strata": strata,
                "per_unit": 1000.0,
                "conf_level": 95,
            }),
            &ctx(),
        )
        .expect_err(&format!("{strata} must be rejected"));
        assert_eq!(err.code(), "calc_input_out_of_range");
    }
}

/// D1 bounded work: a stratum whose binomial support (`m1 + 1`)
/// dwarfs `MAX_EXACT_CONVOLUTION_WORK` answers `too_large` on all
/// five `cmle_rr` leaves instead of allocating unboundedly.
#[test]
fn rate_compare_too_large_is_bounded() {
    let out = run(
        "rate_compare",
        &json!({
            "strata": [[[4e15, 4e15], [1.0, 1.0]]],
            "per_unit": 1000.0,
            "conf_level": 95,
        }),
        &ctx(),
    )
    .expect("rate_compare on huge counts");
    let want = json!({ "undefined": "too_large" });
    for leaf_name in [
        "est",
        "midp_lower",
        "midp_upper",
        "fisher_lower",
        "fisher_upper",
    ] {
        assert_eq!(out.result["strata"][0]["cmle_rr"][leaf_name], want);
        assert_eq!(out.result["crude"]["cmle_rr"][leaf_name], want);
    }
    // Two huge strata: the T4 whole-call cap marks every stratum's
    // and the crude `cmle_rr` leaves, from arithmetic alone.
    let two = run(
        "rate_compare",
        &json!({
            "strata": [
                [[4e15, 4e15], [1.0, 1.0]],
                [[4e15, 4e15], [1.0, 1.0]],
            ],
            "per_unit": 1000.0,
            "conf_level": 95,
        }),
        &ctx(),
    )
    .expect("rate_compare on two huge strata");
    for i in 0..2 {
        for leaf_name in [
            "est",
            "midp_lower",
            "midp_upper",
            "fisher_lower",
            "fisher_upper",
        ] {
            assert_eq!(two.result["strata"][i]["cmle_rr"][leaf_name], want);
        }
    }
    for leaf_name in [
        "est",
        "midp_lower",
        "midp_upper",
        "fisher_lower",
        "fisher_upper",
    ] {
        assert_eq!(two.result["crude"]["cmle_rr"][leaf_name], want);
    }
}

/// Owner decision T1 (recorded on PR #77): a person-time stratum's
/// binomial support `m1 = rint(a + b)` is capped at 2,000,000 — at
/// the limit the stratum still computes; one case past it turns all
/// five `cmle_rr` leaves `too_large`. The pooled (`crude`) table's
/// `m1` is the sum, so it crosses with the stratum.
#[test]
fn pt_support_limit_is_2e6() {
    let run_at = |a: f64, b: f64| {
        run(
            "rate_compare",
            &json!({
                "strata": [[[a, b], [2000000.0, 1000000.0]]],
                "per_unit": 1000.0,
                "conf_level": 95,
            }),
            &ctx(),
        )
        .expect("rate_compare")
    };
    const LEAVES: [&str; 5] = [
        "est",
        "midp_lower",
        "midp_upper",
        "fisher_lower",
        "fisher_upper",
    ];
    // m1 = 1,600,000 + 400,000 = 2,000,000 — at the cap: computes.
    let at = run_at(1_600_000.0, 400_000.0);
    for leaf_name in LEAVES {
        let leaf = &at.result["strata"][0]["cmle_rr"][leaf_name];
        assert!(
            leaf.as_f64().is_some_and(|x| x.is_finite()),
            "m1 = 2e6 must compute, got {leaf} for {leaf_name}"
        );
    }
    // m1 = 1,600,001 + 400,000 = 2,000,001 — one over: too_large,
    // stratum and crude alike.
    let over = run_at(1_600_001.0, 400_000.0);
    let want = json!({ "undefined": "too_large" });
    for leaf_name in LEAVES {
        assert_eq!(over.result["strata"][0]["cmle_rr"][leaf_name], want);
        assert_eq!(over.result["crude"]["cmle_rr"][leaf_name], want);
    }
}

/// Owner decision T4 (recorded on PR #78): `rate_compare` computes
/// each stratum's `cmle_rr` separately, so the call's total support —
/// Σm1 over the informative strata — is capped at 2,000,000 on top of
/// the per-stratum cap. At the cap the strata still compute; one case
/// past it, or nine maximal strata, turns every stratum's and the
/// crude `cmle_rr` leaves `too_large` without any allocation.
#[test]
fn pt_total_support_limit_is_2e6() {
    let run_strata = |strata: Value| {
        run(
            "rate_compare",
            &json!({
                "strata": strata,
                "per_unit": 1000.0,
                "conf_level": 95,
            }),
            &ctx(),
        )
        .expect("rate_compare")
    };
    const LEAVES: [&str; 5] = [
        "est",
        "midp_lower",
        "midp_upper",
        "fisher_lower",
        "fisher_upper",
    ];
    // Σm1 = 1,000,000 + 1,000,000 = 2,000,000 — at the cap: both
    // strata and the pooled crude compute.
    let at = run_strata(json!([
        [[500_000, 500_000], [1_000_000.0, 1_000_000.0]],
        [[500_000, 500_000], [1_000_000.0, 1_000_000.0]],
    ]));
    for i in 0..2 {
        for leaf_name in LEAVES {
            let leaf = &at.result["strata"][i]["cmle_rr"][leaf_name];
            assert!(
                leaf.as_f64().is_some_and(|x| x.is_finite()),
                "Σm1 = 2e6 must compute, got {leaf} for strata[{i}].{leaf_name}"
            );
        }
    }
    for leaf_name in LEAVES {
        let leaf = &at.result["crude"]["cmle_rr"][leaf_name];
        assert!(
            leaf.as_f64().is_some_and(|x| x.is_finite()),
            "Σm1 = 2e6 must compute, got {leaf} for crude.{leaf_name}"
        );
    }
    // Σm1 = 1,000,000 + 1,000,001 = 2,000,001 — one over: every
    // stratum's and the crude's leaves are too_large, including the
    // stratum that is itself under the per-stratum cap.
    let over = run_strata(json!([
        [[500_000, 500_000], [1_000_000.0, 1_000_000.0]],
        [[500_001, 500_000], [1_000_000.0, 1_000_000.0]],
    ]));
    let want = json!({ "undefined": "too_large" });
    for i in 0..2 {
        for leaf_name in LEAVES {
            assert_eq!(over.result["strata"][i]["cmle_rr"][leaf_name], want);
        }
    }
    for leaf_name in LEAVES {
        assert_eq!(over.result["crude"]["cmle_rr"][leaf_name], want);
    }
    // 9 strata × m1 = 2,000,000 — the pre-T4 worst case: the cap is
    // decided arithmetically, so every `cmle_rr` leaf answers
    // too_large without a single polynomial being allocated.
    let nine = run_strata(json!(vec![[[1e6, 1e6], [1.0, 1.0]]; 9]));
    for i in 0..9 {
        for leaf_name in LEAVES {
            assert_eq!(nine.result["strata"][i]["cmle_rr"][leaf_name], want);
        }
    }
    for leaf_name in LEAVES {
        assert_eq!(nine.result["crude"]["cmle_rr"][leaf_name], want);
    }
}
