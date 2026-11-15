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
