//! Corpus test: every spreadsheet case under `fixtures/import/` (v0.3
//! spec §6.5, plan Task 4) — `xlsx_*`, `xls_*` and `ods_*` directories.
//! Same comparison as `fixtures_text.rs`: detection, column
//! names/types/values (through the v0.2 corpus dataset JSON loader,
//! spec v0.2 §3.3) and coercions exactly; `expect_error` cases compare
//! `code()` and `line`.

use epiora_core::corpus::load_dataset;
use epiora_core::dataset::ColumnData;
use epiora_import::{import, Detection, ImportError, ImportSettings};
use serde_json::Value as Json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn case_dirs() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/import");
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(&root)
        .expect("read fixtures/import")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    dirs
}

/// Path-style comparison of one column's values; `None` when equal.
fn column_diff(expected: &ColumnData, actual: &ColumnData, col: &str) -> Option<String> {
    if expected.value_type() != actual.value_type() {
        return Some(format!(
            "columns[{col}].type: expected {:?}, got {:?}",
            expected.value_type(),
            actual.value_type()
        ));
    }
    macro_rules! cmp {
        ($e:expr, $a:expr) => {{
            for (i, (e, a)) in $e.iter().zip($a.iter()).enumerate() {
                if e != a {
                    return Some(format!(
                        "columns[{col}].values[{i}]: expected {e:?}, got {a:?}"
                    ));
                }
            }
            if $e.len() != $a.len() {
                return Some(format!(
                    "columns[{col}].values: expected {} rows, got {}",
                    $e.len(),
                    $a.len()
                ));
            }
            None
        }};
    }
    match (expected, actual) {
        (ColumnData::Integer(e), ColumnData::Integer(a)) => cmp!(e, a),
        (ColumnData::Real(e), ColumnData::Real(a)) => cmp!(e, a),
        (ColumnData::Text(e), ColumnData::Text(a)) => cmp!(e, a),
        (ColumnData::Boolean(e), ColumnData::Boolean(a)) => cmp!(e, a),
        (ColumnData::Date(e), ColumnData::Date(a)) => cmp!(e, a),
        _ => unreachable!("types already compared equal"),
    }
}

/// Builds the expected `Dataset` from `expect.columns` via the v0.2
/// corpus dataset JSON loader (spec v0.2 §3.3).
fn expected_dataset(case: &str, columns: &Json) -> epiora_core::dataset::Dataset {
    let doc = format!(r#"{{"id":"{case}","version":1,"columns":{columns}}}"#);
    load_dataset(&doc).unwrap_or_else(|e| panic!("{case}: corpus loader rejected fixture: {e}"))
}

#[test]
fn every_spreadsheet_fixture_matches() {
    let mut failures: Vec<String> = Vec::new();
    let mut ran = 0usize;
    for dir in case_dirs() {
        let case = dir
            .file_name()
            .expect("dir name")
            .to_string_lossy()
            .to_string();
        if !(case.starts_with("xlsx_") || case.starts_with("xls_") || case.starts_with("ods_")) {
            continue;
        }
        ran += 1;
        let json_text = std::fs::read_to_string(dir.join("expected.json")).expect("expected.json");
        let expected: Json = serde_json::from_str(&json_text).expect("parse expected.json");
        let file = expected["file"].as_str().expect("file").to_string();
        let bytes = std::fs::read(dir.join(&file)).expect("input file");
        let settings: ImportSettings = serde_json::from_value(expected["settings"].clone())
            .unwrap_or_else(|e| panic!("{case}: bad settings JSON: {e}"));

        match import(&bytes, &file, &settings) {
            Err(err) => {
                let want = &expected["expect_error"];
                if want.is_null() {
                    failures.push(format!("{case}: expected Ok, got {} ({err})", err.code()));
                } else if err.code() != want["code"].as_str().unwrap_or_default() {
                    failures.push(format!(
                        "{case}: error.code: expected {:?}, got {:?} ({err})",
                        want["code"],
                        err.code()
                    ));
                } else if let Some(line) = want["line"].as_u64() {
                    match err {
                        ImportError::RaggedRow { line: got, .. } if got == line => {}
                        _ => failures
                            .push(format!("{case}: error.line: expected {line}, got {err:?}")),
                    }
                }
            }
            Ok(imported) => {
                let expect = &expected["expect"];
                if expect.is_null() {
                    failures.push(format!("{case}: expected error, got Ok"));
                    continue;
                }
                let want_det: Detection = serde_json::from_value(expect["detection"].clone())
                    .unwrap_or_else(|e| panic!("{case}: bad detection JSON: {e}"));
                if imported.detection != want_det {
                    failures.push(format!(
                        "{case}: detection: expected {want_det:?}, got {:?}",
                        imported.detection
                    ));
                    continue;
                }
                let want_ds = expected_dataset(&case, &expect["columns"]);
                let mut diff = None;
                for (i, want_col) in want_ds.columns().iter().enumerate() {
                    let got_col = &imported.columns[i];
                    if want_col.name != got_col.name {
                        diff = Some(format!(
                            "columns[{i}].name: expected {:?}, got {:?}",
                            want_col.name, got_col.name
                        ));
                        break;
                    }
                    diff = column_diff(&want_col.data, &got_col.data, &want_col.name);
                    if diff.is_some() {
                        break;
                    }
                }
                if imported.columns.len() != want_ds.columns().len() {
                    diff = Some(format!(
                        "columns: expected {}, got {}",
                        want_ds.columns().len(),
                        imported.columns.len()
                    ));
                }
                if let Some(d) = diff {
                    failures.push(format!("{case}: {d}"));
                    continue;
                }
                let want_coercions: BTreeMap<String, u64> =
                    serde_json::from_value(expect["coercions"].clone())
                        .unwrap_or_else(|e| panic!("{case}: bad coercions JSON: {e}"));
                if imported.coercions != want_coercions {
                    failures.push(format!(
                        "{case}: coercions: expected {want_coercions:?}, got {:?}",
                        imported.coercions
                    ));
                }
            }
        }
    }
    eprintln!("spreadsheet fixtures: {ran} ran");
    assert!(ran > 0, "no spreadsheet fixtures found");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
