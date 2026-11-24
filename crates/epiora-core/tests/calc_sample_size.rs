#![cfg(feature = "corpus")]

//! Sample-size calculator fixture tests (v1.0 Task 9). The corpus
//! `calc.<id>` fixtures under `fixtures/calc/` are the oracle
//! (Task 2 + the C1 real-valued-leaves follow-up).

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

const SAMPLE_SIZE: &[&str] = &[
    "ss_proportion",
    "ss_unmatched_cc",
    "ss_cohort",
    "ss_mean_difference",
];

#[test]
fn every_sample_size_fixture_matches() {
    let datasets: BTreeMap<String, Dataset> = BTreeMap::new();
    let mut problems = Vec::new();
    for id in SAMPLE_SIZE {
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

/// Review Focus 3: equal proportions — in every input form
/// (`percent_*` pairs equal, `odds_ratio` 1, `risk_ratio` 1,
/// `risk_difference` 0) and a zero `ss_mean_difference` difference —
/// turn every sample-size leaf, real and rounded, into
/// `{"undefined": "infinite_sample_size"}`, never a giant or negative
/// number and never a typed error.
#[test]
fn equal_proportions_sample_size_is_undefined() {
    fn leaves<'a>(v: &'a Value, path: String, out: &mut Vec<(String, &'a Value)>) {
        match v {
            Value::Object(m) if m.contains_key("undefined") => out.push((path, v)),
            Value::Object(m) => {
                for (k, c) in m {
                    leaves(c, format!("{path}.{k}"), out);
                }
            }
            Value::Array(items) => {
                for (i, c) in items.iter().enumerate() {
                    leaves(c, format!("{path}[{i}]"), out);
                }
            }
            _ => out.push((path, v)),
        }
    }
    let want = json!({ "undefined": "infinite_sample_size" });
    let cases = [
        // Equal percent exposures.
        (
            "ss_unmatched_cc",
            json!({"conf_level": 95, "power": 80, "ratio_controls_per_case": 1,
                   "percent_controls_exposed": 40, "percent_cases_exposed": 40}),
        ),
        // OR = 1 derives p1 = p2.
        (
            "ss_unmatched_cc",
            json!({"conf_level": 95, "power": 80, "ratio_controls_per_case": 1,
                   "percent_controls_exposed": 40, "odds_ratio": 1}),
        ),
        (
            "ss_cohort",
            json!({"conf_level": 95, "power": 80, "ratio_unexposed_per_exposed": 1,
                   "percent_unexposed_outcome": 5, "percent_exposed_outcome": 5}),
        ),
        (
            "ss_cohort",
            json!({"conf_level": 95, "power": 80, "ratio_unexposed_per_exposed": 1,
                   "percent_unexposed_outcome": 5, "odds_ratio": 1}),
        ),
        (
            "ss_cohort",
            json!({"conf_level": 95, "power": 80, "ratio_unexposed_per_exposed": 1,
                   "percent_unexposed_outcome": 5, "risk_ratio": 1}),
        ),
        (
            "ss_cohort",
            json!({"conf_level": 95, "power": 80, "ratio_unexposed_per_exposed": 1,
                   "percent_unexposed_outcome": 5, "risk_difference": 0}),
        ),
        (
            "ss_mean_difference",
            json!({"conf_level": 95, "power": 80, "ratio": 1,
                   "difference": 0, "sd_1": 10, "sd_2": 10}),
        ),
        (
            "ss_mean_difference",
            json!({"conf_level": 95, "power": 80, "ratio": 1,
                   "mean_1": 10, "mean_2": 10, "sd_1": 10, "sd_2": 10}),
        ),
    ];
    for (id, inputs) in cases {
        let out = run(id, &inputs, &ctx()).unwrap_or_else(|e| {
            panic!("{id} on {inputs}: equal proportions are Undefined, never an error: {e}")
        });
        let mut ls = Vec::new();
        leaves(&out.result, String::new(), &mut ls);
        assert!(!ls.is_empty(), "{id} produced no leaves");
        for (path, leaf) in ls {
            assert_eq!(
                leaf, &want,
                "{id} {path}: expected infinite_sample_size, got {leaf}"
            );
        }
    }
}

/// The rounded-up sample-size leaf is the ceiling of its `<leaf>_real`
/// sibling — for every ceiling-produced pair in every fixture
/// (C1/plan Task 9). `total` pairs are excluded: totals are sums or
/// products of the integer leaves, not ceilings (`ceil(a)+ceil(b) ≠
/// ceil(a+b)` in general), which is why the fixture generator's own
/// `check_ss_real` guard skips them.
#[test]
fn rounded_up_is_ceiling() {
    fn pairs<'a>(v: &'a Value, path: String, out: &mut Vec<(String, &'a Value, &'a Value)>) {
        let Value::Object(m) = v else { return };
        for (k, real) in m {
            if let Some(stem) = k.strip_suffix("_real") {
                if stem != "total" {
                    if let Some(rounded) = m.get(stem) {
                        out.push((format!("{path}.{stem}"), rounded, real));
                    }
                }
            }
            pairs(real, format!("{path}.{k}"), out);
        }
    }
    let datasets: BTreeMap<String, Dataset> = BTreeMap::new();
    let mut problems = Vec::new();
    for id in SAMPLE_SIZE {
        let dir = fixtures_root().join("calc").join(id);
        for file in std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("{dir:?}: {e}"))
            .map(|e| e.expect("dir entry").path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
        {
            let fixture: Value =
                serde_json::from_str(&std::fs::read_to_string(&file).expect("read fixture"))
                    .expect("parse fixture");
            let out =
                run_case(&fixture, &datasets).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            let mut engine_pairs = Vec::new();
            pairs(&out.actual, String::new(), &mut engine_pairs);
            for (path, rounded, real) in &engine_pairs {
                match (rounded.as_f64(), real.as_f64()) {
                    (Some(n), Some(r)) => {
                        assert_eq!(
                            n,
                            r.ceil(),
                            "{} {path}: rounded {n} is not ceil({r})",
                            file.display()
                        );
                    }
                    (None, None) => {} // both undefined — consistent
                    _ => problems.push(format!(
                        "{} {path}: rounded/real kind mismatch ({rounded} vs {real})",
                        file.display()
                    )),
                }
            }
            // The fixture's own expected tree obeys the same pair rule.
            let mut fixture_pairs = Vec::new();
            pairs(&fixture["expected"], String::new(), &mut fixture_pairs);
            for (path, rounded, real) in &fixture_pairs {
                let (Some(n), Some(r)) = (rounded["value"].as_f64(), real["value"].as_f64()) else {
                    continue;
                };
                assert_eq!(
                    n,
                    r.ceil(),
                    "{} {path}: fixture rounded {n} is not ceil({r})",
                    file.display()
                );
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}
