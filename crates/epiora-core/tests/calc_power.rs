#![cfg(feature = "corpus")]

//! Power calculator fixture tests (v1.0 Task 9). The corpus
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

const POWER: &[&str] = &[
    "power_unmatched_cc",
    "power_cohort",
    "power_mean_difference",
];

#[test]
fn every_power_fixture_matches() {
    let datasets: BTreeMap<String, Dataset> = BTreeMap::new();
    let mut problems = Vec::new();
    for id in POWER {
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

/// Review Focus 3, amended by owner decision C3 (2026-11-16, recorded
/// in docs/decisions.md): the `normal` variants keep OpenEpi's
/// one-sided `Φ(powerz)` formula, so at zero effect (equal
/// proportions, or `difference` = 0) power equals α/2 as a percent —
/// `(100 − conf_level)/2` — within 1e-9, and the continuity-corrected
/// variants are `Undefined` there (their `(r+1)/(r·d)` correction
/// divides by the zero difference), never NaN.
#[test]
fn zero_effect_power_equals_alpha() {
    for conf in [90u64, 95, 99] {
        let alpha_half = (100.0 - conf as f64) / 2.0;

        let cc = run(
            "power_unmatched_cc",
            &json!({"conf_level": conf, "n_cases": 100, "n_controls": 150,
                    "percent_cases_exposed": 40, "percent_controls_exposed": 40}),
            &ctx(),
        )
        .expect("power_unmatched_cc at zero effect");
        let normal = cc.result["normal"]
            .as_f64()
            .expect("normal must be a number at zero effect");
        assert!(
            (normal - alpha_half).abs() <= 1e-9,
            "conf {conf}: power_unmatched_cc.normal {normal} != {alpha_half}"
        );
        assert!(
            cc.result["normal_cc"].get("undefined").is_some(),
            "conf {conf}: normal_cc must be Undefined, got {}",
            cc.result["normal_cc"]
        );

        let co = run(
            "power_cohort",
            &json!({"conf_level": conf, "design": "cohort",
                    "n_exposed": 200, "n_unexposed": 400,
                    "percent_exposed_outcome": 40,
                    "percent_unexposed_outcome": 40}),
            &ctx(),
        )
        .expect("power_cohort at zero effect");
        let normal = co.result["normal"]
            .as_f64()
            .expect("normal must be a number at zero effect");
        assert!(
            (normal - alpha_half).abs() <= 1e-9,
            "conf {conf}: power_cohort.normal {normal} != {alpha_half}"
        );
        assert!(
            co.result["normal_cc"].get("undefined").is_some(),
            "conf {conf}: normal_cc must be Undefined, got {}",
            co.result["normal_cc"]
        );

        let md = run(
            "power_mean_difference",
            &json!({"conf_level": conf, "n_1": 50, "n_2": 50,
                    "difference": 0, "sd_1": 10, "sd_2": 10}),
            &ctx(),
        )
        .expect("power_mean_difference at zero effect");
        let power = md.result["power"]
            .as_f64()
            .expect("power must be a number at zero effect");
        assert!(
            (power - alpha_half).abs() <= 1e-9,
            "conf {conf}: power_mean_difference.power {power} != {alpha_half}"
        );
    }
}
