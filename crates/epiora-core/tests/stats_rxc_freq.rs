#![cfg(feature = "corpus")]

use epiora_core::corpus::compare::compare;
use epiora_core::stats::{proportions, rxc};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

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

fn fixture_path(family: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(family)
}

fn report(
    id: &str,
    result: Result<Vec<epiora_core::corpus::compare::Mismatch>, epiora_core::EngineError>,
    failures: &mut Vec<String>,
) {
    match result {
        Ok(mismatches) => {
            for m in mismatches {
                failures.push(format!("{id}: {m}"));
            }
        }
        Err(e) => failures.push(format!("{id}: {e}")),
    }
}

#[test]
fn rxc_fixtures() {
    let mut failures = Vec::new();
    for (id, fixture) in load_dir(&fixture_path("rxc")) {
        let table: Vec<Vec<u64>> = fixture["inputs"]["table"]
            .as_array()
            .expect("table rows")
            .iter()
            .map(|row| {
                row.as_array()
                    .expect("table row")
                    .iter()
                    .map(|c| c.as_u64().expect("table cell is u64"))
                    .collect()
            })
            .collect();
        let actual = serde_json::to_value(rxc(&table)).expect("serialize RxcStats");
        report(&id, compare(&fixture, &actual), &mut failures);
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

/// `proportions` produces only the per-category statistics; `total` and
/// `missing` come from tabulation (Task 8), so the comparison is scoped
/// to the fixture's `categories` subtree.
#[test]
fn freq_proportion_fixtures() {
    let mut failures = Vec::new();
    for (id, fixture) in load_dir(&fixture_path("freq")) {
        let counts: Vec<u64> = fixture["expected"]["categories"]
            .as_array()
            .expect("categories array")
            .iter()
            .map(|c| c["count"]["value"].as_u64().expect("count is u64"))
            .collect();
        let scoped = json!({
            "tolerance_classes": fixture["tolerance_classes"],
            "expected": { "categories": fixture["expected"]["categories"] },
        });
        let actual = json!({
            "categories": serde_json::to_value(proportions(&counts))
                .expect("serialize CategoryStats"),
        });
        report(&id, compare(&scoped, &actual), &mut failures);
    }
    assert!(
        failures.is_empty(),
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
