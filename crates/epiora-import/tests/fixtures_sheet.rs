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

/// The ODS mimetype string, stored first and uncompressed per ODF.
const ODS_MIMETYPE: &[u8] = b"application/vnd.oasis.opendocument.spreadsheet";

/// A minimal manifest calamine accepts (no `manifest:encryption-data`).
const ODS_MANIFEST: &[u8] = b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<manifest:manifest xmlns:manifest=\"urn:oasis:names:tc:opendocument:xmlns:manifest:1.0\">\
<manifest:file-entry manifest:full-path=\"/\" manifest:media-type=\"application/vnd.oasis.opendocument.spreadsheet\"/>\
<manifest:file-entry manifest:full-path=\"content.xml\" manifest:media-type=\"text/xml\"/>\
</manifest:manifest>";

/// Wraps `content` (a `content.xml`) in a minimal ODS zip.
fn ods_zip(content: &[u8]) -> Vec<u8> {
    use std::io::{Cursor, Write};
    let mut ods = Vec::new();
    {
        let mut zw = zip::ZipWriter::new(Cursor::new(&mut ods));
        let stored = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Stored);
        zw.start_file("mimetype", stored).unwrap();
        zw.write_all(ODS_MIMETYPE).unwrap();
        let deflated = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zw.start_file("META-INF/manifest.xml", deflated).unwrap();
        zw.write_all(ODS_MANIFEST).unwrap();
        zw.start_file("content.xml", deflated).unwrap();
        zw.write_all(content).unwrap();
        zw.finish().unwrap();
    }
    ods
}

/// A 2×2 sheet (header `A`,`B`; one data row `1`,`2`) whose
/// `content.xml` carries `whitespace` bytes of whitespace between the
/// header-row cells — a direct child of `table:table-row`, so the ODS
/// repack drops it before calamine parses.
fn ods_content(whitespace: usize) -> Vec<u8> {
    let mut content = Vec::with_capacity(whitespace + 2048);
    content.extend_from_slice(
        b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\
<office:document-content xmlns:office=\"urn:oasis:names:tc:opendocument:xmlns:office:1.0\" \
xmlns:table=\"urn:oasis:names:tc:opendocument:xmlns:table:1.0\" \
xmlns:text=\"urn:oasis:names:tc:opendocument:xmlns:text:1.0\" office:version=\"1.2\">\
<office:body><office:spreadsheet><table:table table:name=\"Sheet1\">\
<table:table-row>\
<table:table-cell office:value-type=\"string\"><text:p>A</text:p></table:table-cell>",
    );
    content.resize(content.len() + whitespace, b' ');
    content.extend_from_slice(
        b"<table:table-cell office:value-type=\"string\"><text:p>B</text:p></table:table-cell>\
</table:table-row>\
<table:table-row>\
<table:table-cell office:value-type=\"float\" office:value=\"1\"><text:p>1</text:p></table:table-cell>\
<table:table-cell office:value-type=\"float\" office:value=\"2\"><text:p>2</text:p></table:table-cell>\
</table:table-row>\
</table:table></office:spreadsheet></office:body></office:document-content>",
    );
    content
}

/// The ODS repack streams `content.xml` instead of buffering members: a
/// `content.xml` holding ≥ 64 MiB of whitespace between the cells of a
/// 2×2 sheet still imports to the expected columns and values.
#[test]
fn ods_large_whitespace_is_streamed() {
    let ods = ods_zip(&ods_content(65 * 1024 * 1024));
    let imported = import(&ods, "large.ods", &ImportSettings::default())
        .expect("large-whitespace ODS imports");
    assert_eq!(imported.columns.len(), 2);
    assert_eq!(imported.columns[0].name, "A");
    assert_eq!(imported.columns[0].data, ColumnData::Integer(vec![Some(1)]));
    assert_eq!(imported.columns[1].name, "B");
    assert_eq!(imported.columns[1].data, ColumnData::Integer(vec![Some(2)]));
}
