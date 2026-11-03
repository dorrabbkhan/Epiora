#![cfg(feature = "corpus")]

use epiora_core::corpus::compare::compare;
use epiora_core::stats::{stratified, two_by_two, Estimate, Table2x2};
use proptest::prelude::*;
use serde_json::Value;
use std::path::{Path, PathBuf};

fn fixture_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/stratified")
}

fn load_dir(dir: &Path) -> Vec<(String, Value)> {
    let mut out: Vec<(String, Value)> = std::fs::read_dir(dir)
        .expect("read fixture dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .map(|p| {
            let id = p
                .file_stem()
                .expect("file stem")
                .to_string_lossy()
                .to_string();
            let fixture = serde_json::from_str(&std::fs::read_to_string(&p).expect("read fixture"))
                .expect("parse fixture");
            (id, fixture)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn strata_of(fixture: &Value) -> Vec<Table2x2> {
    fixture["inputs"]["strata"]
        .as_array()
        .expect("inputs.strata is an array")
        .iter()
        .map(|t| {
            let cell = |r: usize, c: usize| t[r][c].as_u64().expect("stratum cell is u64");
            Table2x2 {
                a: cell(0, 0),
                b: cell(0, 1),
                c: cell(1, 0),
                d: cell(1, 1),
            }
        })
        .collect()
}

fn val(e: &Estimate) -> Option<f64> {
    match e {
        Estimate::Value(v) => Some(*v),
        Estimate::Undefined { .. } => None,
    }
}

fn table() -> impl Strategy<Value = Table2x2> {
    (0u64..=500, 0u64..=500, 0u64..=500, 0u64..=500).prop_map(|(a, b, c, d)| Table2x2 {
        a,
        b,
        c,
        d,
    })
}

/// Every leaf of `v` is `{"undefined": "empty_table"}` — the whole
/// statistic block is undefined with the empty-table reason.
fn all_empty_table(v: &Value) -> bool {
    match v {
        Value::Object(m) => match m.get("undefined") {
            Some(u) => m.len() == 1 && u == "empty_table",
            None => m.values().all(all_empty_table),
        },
        Value::Array(items) => items.iter().all(all_empty_table),
        _ => false,
    }
}

#[test]
fn stratified_fixtures() {
    let mut failures = Vec::new();
    for (id, fixture) in load_dir(&fixture_dir()) {
        let strata = strata_of(&fixture);
        let actual = serde_json::to_value(stratified(&strata)).expect("serialize StratifiedStats");
        match compare(&fixture, &actual) {
            Ok(mismatches) => {
                for m in mismatches {
                    failures.push(format!("{id}: {m}"));
                }
            }
            Err(e) => failures.push(format!("{id}: {e}")),
        }
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn empty_stratum_is_excluded_and_listed() {
    let (_, fixture) = load_dir(&fixture_dir())
        .into_iter()
        .find(|(id, _)| id == "empty_stratum")
        .expect("empty_stratum fixture exists");
    let strata = strata_of(&fixture);
    let three = stratified(&strata);
    assert_eq!(three.excluded_strata, vec![1]);

    let two = stratified(&[strata[0], strata[2]]);
    assert_eq!(two.excluded_strata, Vec::<usize>::new());
    assert_eq!(
        serde_json::to_value(&three.mantel_haenszel).expect("serialize"),
        serde_json::to_value(&two.mantel_haenszel).expect("serialize"),
        "empty stratum changed the Mantel-Haenszel values"
    );
    assert_eq!(
        serde_json::to_value(&three.breslow_day_tarone).expect("serialize"),
        serde_json::to_value(&two.breslow_day_tarone).expect("serialize"),
        "empty stratum changed Breslow-Day-Tarone"
    );
    assert_eq!(
        serde_json::to_value(&three.crude).expect("serialize"),
        serde_json::to_value(&two.crude).expect("serialize"),
        "empty stratum changed the crude table"
    );
}

proptest! {
    #[test]
    fn one_stratum_mh_equals_crude(t in table()) {
        let s = stratified(&[t]);
        let c = two_by_two(&t);
        for (mh, crude, name) in [
            (
                &s.mantel_haenszel.odds_ratio.estimate,
                &c.odds_ratio.estimate,
                "OR",
            ),
            (
                &s.mantel_haenszel.risk_ratio.estimate,
                &c.risk_ratio.estimate,
                "RR",
            ),
        ] {
            if let (Some(m), Some(e)) = (val(mh), val(crude)) {
                prop_assert!(
                    (m - e).abs() <= 1e-9 * e.abs(),
                    "one-stratum MH {name} {m} != crude {e} within 1e-9"
                );
            }
        }
    }
}

#[test]
fn all_strata_empty() {
    let zero = Table2x2 {
        a: 0,
        b: 0,
        c: 0,
        d: 0,
    };
    let s = stratified(&[zero, zero, zero]);
    assert_eq!(s.excluded_strata, vec![0, 1, 2]);
    let v = serde_json::to_value(&s).expect("serialize StratifiedStats");
    for key in ["crude", "mantel_haenszel", "breslow_day_tarone"] {
        assert!(
            all_empty_table(&v[key]),
            "{key} is not all Undefined {{ EmptyTable }}: {}",
            v[key]
        );
    }
}
