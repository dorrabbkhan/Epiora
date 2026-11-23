//! Task 8 tests: every `ei3_access` golden maps structurally to its
//! `project.json` + dataset files through the native `epiora-mdb`
//! reader, plus `detect` routing and the code-table → label-set pin.
//! `every_ei3_sample_matches_golden` is skipped only by the D12
//! verdict read from `fixtures/legacy/samples/README.md` — never by
//! `#[ignore]`: a non-`redistributable` verdict asserts the
//! `Unvalidated` report entry instead (plan Task 8 Interfaces).

mod common;

use std::fs;
use std::path::PathBuf;

use common::*;
use epiora_core::dataset::Value;
use epiora_legacy::{detect, map_ei3, BatchSource, ReportKind, SourceFormat};

/// The D12 verdict on the `ei3/` bullet of the samples README
/// (`decision D12 — <verdict>`). Panics when no recognised marker is
/// present: silently defaulting would hide a README rewrite.
fn d12_verdict() -> &'static str {
    let readme = fs::read_to_string(root().join("fixtures/legacy/samples/README.md")).unwrap();
    // The `ei3/` bullet wraps over several lines: gather from the
    // `- `ei3/`` line to the next bullet/blank line, then read the
    // verdict marker out of the whole bullet.
    let lines: Vec<&str> = readme.lines().collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with("- `ei3/`"))
        .expect("samples README must carry the ei3 bullet");
    let bullet = lines[start..]
        .iter()
        .take_while(|l| !l.trim().is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    let line = bullet.as_str();
    for (marker, verdict) in [
        ("redistributable", "redistributable"),
        ("public domain", "redistributable"),
        ("download-only", "download-only"),
        ("unavailable", "unavailable"),
    ] {
        if line.contains(marker) {
            return verdict;
        }
    }
    panic!("README ei3 bullet names no D12 verdict: {line:?}");
}

/// Every `ei3_access` golden: `golden/ei3/<P>/` flat plus the nested
/// `golden/ei7/<P>/<file>/` directories (JET4 `.mdb`s without
/// `metaViews` are classified `ei3_access` by the golden too).
fn ei3_projects() -> Vec<(PathBuf, PathBuf)> {
    let mut out = Vec::new();
    let mut collect = |dir: PathBuf| {
        if !dir.join("project.json").exists() {
            return;
        }
        let pj = golden_project(&dir);
        if pj["source_format"].as_str().unwrap() != "ei3_access" {
            return;
        }
        out.push((dir, root().join(pj["source"]["path"].as_str().unwrap())));
    };
    for e in fs::read_dir(golden_root().join("ei3")).unwrap() {
        collect(e.unwrap().path());
    }
    for e in fs::read_dir(golden_root().join("ei7")).unwrap() {
        for sub in fs::read_dir(e.unwrap().path()).unwrap() {
            collect(sub.unwrap().path());
        }
    }
    out.sort();
    out
}

/// A minimal synthetic EI3 source: `viewV` (Type column, a DATA1
/// SOURCE row, one TEXTBOX field) + the `V` data table.
fn ei3_view_source() -> BatchSource {
    let mut src = BatchSource::new();
    src.push(
        "viewV",
        &encode_batch(
            "viewV",
            &[
                ("Type".into(), 10, text_col(&["SOURCE", "TEXTBOX"])),
                ("Name".into(), 10, text_col(&["DATA1", "X"])),
                ("PageNumber".into(), 10, int_col(&[0, 1])),
                ("Datatable".into(), 10, text_col(&["V", "DATA1"])),
            ],
        ),
    )
    .unwrap();
    src.push(
        "V",
        &encode_batch(
            "V",
            &[
                ("UniqueKey".into(), 10, int_col(&[1, 2])),
                ("RECSTATUS".into(), 10, int_col(&[1, 0])),
                ("X".into(), 10, text_col(&["a", "b"])),
            ],
        ),
    )
    .unwrap();
    src
}

#[test]
fn every_ei3_sample_matches_golden() {
    if d12_verdict() != "redistributable" {
        // The D12-unavailable world (spec §8.2): EI3 support ships
        // "unvalidated" — every project report carries the entry the
        // plan names, instead of a golden comparison. (`EI3_VALIDATED`
        // in `ei3/mod.rs` is the product-side D12 record the verdict
        // would flip; the report assert below checks the behaviour.)
        let mut src = ei3_view_source();
        let p = map_ei3(&mut src, "Probe").unwrap();
        assert!(
            p.report.iter().any(|e| e.kind == ReportKind::Unvalidated
                && e.item == "Probe"
                && e.reason
                    == "Epi Info 3.5.x support is not validated against sample projects (D12)"),
            "expected the D12 Unvalidated entry, got {:?}",
            p.report
        );
        return;
    }
    let projects = ei3_projects();
    assert!(
        projects.len() >= 12,
        "expected the full ei3 golden set, found {}",
        projects.len()
    );
    for (dir, src) in &projects {
        let mut source = mdb_source(src);
        let stem = src.file_stem().unwrap().to_string_lossy().to_string();
        // `detect` routes by layout: viewful files are Ei3Access,
        // viewless ones are the layout_unknown error the caller turns
        // into plain-table import.
        if golden_project(dir)["views"].as_array().unwrap().is_empty() {
            assert_eq!(
                detect(&source).unwrap_err().code(),
                "legacy_layout_unknown",
                "{dir:?}: detect"
            );
        } else {
            assert_eq!(
                detect(&source).unwrap(),
                SourceFormat::Ei3Access,
                "{dir:?}: detect"
            );
        }
        let project =
            map_ei3(&mut source, &stem).unwrap_or_else(|e| panic!("{dir:?}: map_ei3 failed: {e}"));
        assert_project_eq(&project, dir);
    }
}

#[test]
fn ei3_detected_not_ei7() {
    // A `view*` definition table and no `metaViews` → Ei3Access.
    let mut src = ei3_view_source();
    assert_eq!(detect(&src).unwrap(), SourceFormat::Ei3Access);
    // `metaViews` wins: an EI7 project never reads as EI3 even when a
    // `view*` table is present.
    let mut ei7 = ei7_meta_source(&[(1, "X", 1, 1)]);
    ei7.push(
        "viewV",
        &encode_batch("viewV", &[("Type".into(), 10, text_col(&["X"]))]),
    )
    .unwrap();
    assert_eq!(detect(&ei7).unwrap(), SourceFormat::Ei7Access);
    // And the synthetic source maps: one view `V` keyed on UniqueKey,
    // RECSTATUS 0 counted deleted.
    let p = map_ei3(&mut src, "Probe").unwrap();
    assert_eq!(p.views.len(), 1);
    assert_eq!(p.views[0].name, "V");
    assert_eq!(p.views[0].key_column, "UniqueKey");
    assert_eq!(p.views[0].deleted_records, 1);
}

#[test]
fn ei3_code_tables_become_label_sets() {
    let mdb = root().join("fixtures/legacy/samples/ei3/Asthma Survey 2005.mdb");
    let mut src = mdb_source(&mdb);
    let p = map_ei3(&mut src, "Asthma Survey 2005").unwrap();
    let ds = &p.views[0].dataset;
    let sets = ds.label_sets();
    // `W(CODE6,Gender)` — one stored*display pair → the label-set id is
    // the resolved table name.
    let gender = sets.get("codeGender").expect("codeGender label set");
    assert_eq!(
        gender.entries,
        vec![
            (Value::Text("Female".to_string()), "Female".to_string()),
            (Value::Text("Male".to_string()), "Male".to_string()),
        ]
    );
    // `C(CODE10,SchoolID*SchoolID , PSNumber*PSNumber)` — two pairs →
    // `<table>#<stored>-><display>` resolved through the SOURCE map.
    assert!(sets.contains_key("codeSchoolID1234#SchoolID->SchoolID"));
    let metas = ds.meta();
    let idx = |n: &str| ds.columns().iter().position(|c| c.name == n).unwrap();
    assert_eq!(
        metas[idx("Gender")].label_set.as_deref(),
        Some("codeGender")
    );
    assert_eq!(
        metas[idx("SchoolNum")].label_set.as_deref(),
        Some("codeSchoolID1234#SchoolID->SchoolID")
    );
    // Cough/ShortB/WakeUp all point at codeWhistle (one shared set).
    assert_eq!(
        metas[idx("Cough")].label_set.as_deref(),
        Some("codeWhistle")
    );
}
