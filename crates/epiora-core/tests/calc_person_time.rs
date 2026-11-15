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
/// method — no NaN or infinity anywhere.
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
    for method in ["midp", "fisher", "normal", "byar", "rothman_greenland"] {
        let ci = &out.result["ci"][method];
        assert_eq!(ci["lower"], json!(0.0), "{method}.lower must be exactly 0");
        assert!(
            ci["upper"].as_f64().is_some_and(|x| x.is_finite()),
            "{method}.upper must be a finite value, got {}",
            ci["upper"]
        );
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
    }
}
