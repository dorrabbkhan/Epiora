#![cfg(feature = "corpus")]

//! Continuous-summary calculator fixture tests (v1.0 Task 8). The
//! corpus `calc.<id>` fixtures under `fixtures/calc/` are the oracle
//! (Task 2); the t-test/ANOVA calculators share the MEANS summary
//! builders.

use epiora_core::calc::run;
use epiora_core::corpus::compare::compare;
use epiora_core::corpus::load_dataset_dir;
use epiora_core::corpus::run_case::run_case;
use epiora_core::dataset::Dataset;
use epiora_core::error::EngineError;
use epiora_core::procedures::{means, MeansOptions, RunContext};
use epiora_core::stats::Estimate;
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

const CONTINUOUS: &[&str] = &["mean_ci", "median_ci", "t_test_summary", "anova_summary"];

#[test]
fn every_continuous_fixture_matches() {
    let datasets: BTreeMap<String, Dataset> = BTreeMap::new();
    let mut problems = Vec::new();
    for id in CONTINUOUS {
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

fn value(e: &Estimate) -> f64 {
    match e {
        Estimate::Value(v) => *v,
        Estimate::Undefined { reason } => panic!("expected Value, got Undefined({reason:?})"),
    }
}

fn leaf_value(v: &Value) -> f64 {
    v.as_f64()
        .unwrap_or_else(|| panic!("expected a number, got {v}"))
}

/// `t_test_summary` and MEANS share `t_tests_from_summary`: the same
/// group summaries give the same t, df and two-sided p. `means_ties`
/// is the two-group corpus dataset (`means_basic` has three groups
/// and therefore no t-test in its MEANS output).
#[test]
fn t_test_summary_equals_means_on_same_data() {
    let sets = load_dataset_dir(&fixtures_root().join("data")).expect("datasets");
    let ds = sets.get("means_ties").expect("means_ties dataset");
    let out = means(
        ds,
        &MeansOptions {
            variable: "value".into(),
            group_by: Some("grp".into()),
            include_missing: false,
        },
        &ctx(),
    )
    .expect("means on means_ties");
    let groups = &out.result.groups;
    assert_eq!(groups.len(), 2, "means_ties must have two groups");
    let sd = |i: usize| libm::sqrt(value(&groups[i].variance));
    let calc = run(
        "t_test_summary",
        &json!({
            "conf_level": 95,
            "n_1": groups[0].observations,
            "mean_1": value(&groups[0].mean),
            "sd_1": sd(0),
            "n_2": groups[1].observations,
            "mean_2": value(&groups[1].mean),
            "sd_2": sd(1),
        }),
        &ctx(),
    )
    .expect("t_test_summary");
    let means_tests = out.result.t_test.as_ref().expect("two-group t_test");
    for side in ["pooled", "welch"] {
        let m = match side {
            "pooled" => &means_tests.pooled,
            _ => &means_tests.welch,
        };
        for (calc_key, est) in [("t", &m.t), ("df", &m.df), ("p_2t", &m.p_value)] {
            let c = leaf_value(&calc.result[side][calc_key]);
            let diff = (c - value(est)).abs();
            assert!(
                diff < 1e-12,
                "{side}.{calc_key}: calc {c} vs means {} (diff {diff})",
                value(est)
            );
        }
    }
}

/// Inventory `## t_test_summary`: each group takes exactly one of
/// `sd_i`/`se_i`/`var_i`. `se` and `var` inputs converting to the
/// textbook fixture's `sd` must give the same t, df, p and CI —
/// `sd = se·√n`, `sd = √var`.
#[test]
fn t_test_summary_se_and_var_equal_sd() {
    let (n1, sd1, n2, sd2) = (25.0_f64, 2.5_f64, 30.0_f64, 3.0_f64);
    let base = json!({
        "conf_level": 95,
        "n_1": n1 as u64, "mean_1": 10.5, "sd_1": sd1,
        "n_2": n2 as u64, "mean_2": 8.5, "sd_2": sd2,
    });
    let with = |spread: &str, v1: f64, v2: f64| {
        let mut inputs = base.clone();
        let m = inputs.as_object_mut().expect("object");
        m.remove("sd_1");
        m.remove("sd_2");
        m.insert(format!("{spread}_1"), json!(v1));
        m.insert(format!("{spread}_2"), json!(v2));
        run("t_test_summary", &inputs, &ctx()).expect("t_test_summary")
    };
    let want = run("t_test_summary", &base, &ctx()).expect("t_test_summary");
    let variants = [
        with("se", sd1 / libm::sqrt(n1), sd2 / libm::sqrt(n2)),
        with("var", sd1 * sd1, sd2 * sd2),
    ];
    for (label, variant) in ["se", "var"].iter().zip(variants.iter()) {
        for side in ["pooled", "welch"] {
            for leaf_name in ["t", "df", "p_2t", "diff", "lower", "upper"] {
                let a = leaf_value(&want.result[side][leaf_name]);
                let b = leaf_value(&variant.result[side][leaf_name]);
                assert!(
                    (a - b).abs() <= 1e-12,
                    "{label} {side}.{leaf_name}: sd {a} vs {label} {b}"
                );
            }
        }
        for leaf_name in ["f", "df_num", "df_den", "p"] {
            let a = leaf_value(&want.result["f_variance"][leaf_name]);
            let b = leaf_value(&variant.result["f_variance"][leaf_name]);
            assert!(
                (a - b).abs() <= 1e-12,
                "{label} f_variance.{leaf_name}: sd {a} vs {label} {b}"
            );
        }
    }
}

/// The `sd_i`/`se_i`/`var_i` groups are exactly-one: none given →
/// `calc_input_missing`; more than one → `calc_input_out_of_range`.
#[test]
fn t_test_summary_needs_exactly_one_spread() {
    let base = json!({
        "conf_level": 95,
        "n_1": 25, "mean_1": 10.5,
        "n_2": 30, "mean_2": 8.5,
    });
    // Neither group carries a spread.
    let err = run("t_test_summary", &base, &ctx()).expect_err("no spread");
    assert_eq!(err.code(), "calc_input_missing");
    // Group 2 spread only: group 1's group still misses.
    let mut only_g2 = base.clone();
    only_g2["sd_2"] = json!(3.0);
    let err = run("t_test_summary", &only_g2, &ctx()).expect_err("group 1 spread");
    assert_eq!(err.code(), "calc_input_missing");
    // Two spreads in one group is out of range.
    for (a, b) in [("sd_1", "se_1"), ("sd_2", "var_2")] {
        let mut both = base.clone();
        both["sd_1"] = json!(2.5);
        both["var_2"] = json!(9.0);
        both[a] = json!(2.5);
        both[b] = json!(6.25);
        let err =
            run("t_test_summary", &both, &ctx()).expect_err(&format!("{a} + {b} must be rejected"));
        match err {
            EngineError::CalcInputOutOfRange { reason, .. } => {
                assert_eq!(reason, "bad_choice", "{a} + {b}")
            }
            other => panic!("expected CalcInputOutOfRange, got {other:?}"),
        }
    }
}

/// The normal approximation needs an expected observation on each
/// side of the percentile: `n·p < 1` or `n·(1−p) < 1` is below the
/// method's minimum and reports `Undefined { too_few }`.
#[test]
fn median_ci_needs_minimum_n() {
    let out = run(
        "median_ci",
        &json!({ "n": 10, "percentile": 5, "conf_level": 95 }),
        &ctx(),
    )
    .expect("median_ci");
    let want = json!({ "undefined": "too_few" });
    assert_eq!(out.result["normal"]["lower_rank"], want);
    assert_eq!(out.result["normal"]["upper_rank"], want);
}
