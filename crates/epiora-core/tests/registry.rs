#![cfg(feature = "corpus")]

use epiora_core::corpus::load_dataset_dir;
use epiora_core::procedures::{freq, FreqOptions, ProportionCi, RunContext};
use epiora_core::stats::{rxc, stratified, two_by_two, Table2x2};
use epiora_core::validation::{statistic_ids, REGISTRY};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
    }
}

/// The leaf ids a serialized stats value emits, prefixed by family —
/// the same enumeration `receipt` uses for its `validation` list.
fn emitted(value: &Value, family: &str) -> BTreeSet<String> {
    statistic_ids(value)
        .into_iter()
        .map(|path| format!("{family}.{path}"))
        .collect()
}

#[test]
fn every_emitted_statistic_is_registered() {
    let registered: BTreeSet<&str> = REGISTRY.iter().map(|c| c.statistic).collect();
    let mut all = BTreeSet::new();

    let t = Table2x2 {
        a: 12,
        b: 4,
        c: 3,
        d: 9,
    };
    let two = serde_json::to_value(two_by_two(&t)).expect("serialize 2x2");
    all.extend(emitted(&two, "two_by_two"));

    let strat = serde_json::to_value(stratified(&[
        t,
        Table2x2 {
            a: 8,
            b: 6,
            c: 4,
            d: 10,
        },
    ]))
    .expect("serialize stratified");
    all.extend(emitted(&strat, "stratified_two_by_two"));

    let rxc_stats =
        serde_json::to_value(rxc(&[vec![3, 4], vec![4, 4], vec![4, 4]])).expect("serialize rxc");
    all.extend(emitted(&rxc_stats, "rxc"));

    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/data");
    let sets = load_dataset_dir(&dir).expect("corpus datasets");
    let out = freq(
        sets.get("missing").expect("missing dataset"),
        &FreqOptions {
            variables: vec!["GROUP".into()],
            stratify_by: None,
            include_missing: true,
            ci_method: ProportionCi::ClopperPearson,
        },
        &ctx(),
    )
    .expect("freq");
    for table in &out.result.tables {
        let f = serde_json::to_value(table).expect("serialize freq table");
        all.extend(emitted(&f, "freq"));
    }

    let unregistered: Vec<String> = all
        .iter()
        .filter(|id| !registered.contains(id.as_str()))
        .cloned()
        .collect();
    assert!(
        unregistered.is_empty(),
        "emitted statistics missing from REGISTRY:\n{}",
        unregistered.join("\n")
    );
    assert!(!all.is_empty());
}

#[test]
fn every_registry_fixture_exists_and_contains_the_statistic() {
    let mut problems = Vec::new();
    for coverage in REGISTRY {
        let rel = coverage
            .statistic
            .split_once('.')
            .map(|(_, rel)| rel)
            .expect("statistic is family-prefixed");
        for fixture_id in coverage.fixtures {
            let path = fixtures_root().join(format!("{fixture_id}.json"));
            if !path.exists() {
                problems.push(format!(
                    "{}: fixture {fixture_id} does not exist",
                    coverage.statistic
                ));
                continue;
            }
            let fixture: Value =
                serde_json::from_str(&std::fs::read_to_string(&path).expect("read fixture"))
                    .expect("parse fixture");
            if !path_exists(&fixture["expected"], rel) {
                problems.push(format!(
                    "{}: not a leaf path in {fixture_id}",
                    coverage.statistic
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// Does `rel` (a dotted leaf path with `[]` for array positions) reach
/// at least one leaf in `expected`? `[]` steps into every array element.
fn path_exists(expected: &Value, rel: &str) -> bool {
    let mut frontier = vec![expected];
    for seg in rel.split('.') {
        let (key, is_arr) = match seg.strip_suffix("[]") {
            Some(k) => (k, true),
            None => (seg, false),
        };
        let mut next = Vec::new();
        for node in frontier {
            if let Some(child) = node.get(key) {
                if is_arr {
                    if let Some(items) = child.as_array() {
                        next.extend(items);
                    }
                } else {
                    next.push(child);
                }
            }
        }
        if next.is_empty() {
            return false;
        }
        frontier = next;
    }
    !frontier.is_empty()
}
