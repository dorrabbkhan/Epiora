//! Golden `project.json` loading and the structural comparisons:
//! dataset equality, whole-project equality against a golden dir, the
//! Access-vs-SQLite project comparison, and the degenerate-date
//! normalisation the corpus loader needs.

use std::fs;
use std::path::{Path, PathBuf};

use epiora_core::corpus::dataset_json::load_dataset;
use epiora_core::dataset::Dataset;
use epiora_legacy::{LegacyProject, ReportEntry, SourceFormat};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};

use super::{golden_root, root};

/// Load `golden/<dir>/project.json`.
pub fn golden_project(dir: &Path) -> Json {
    serde_json::from_str(&fs::read_to_string(dir.join("project.json")).unwrap()).unwrap()
}

/// Load a golden dataset JSON via the corpus loader.
pub fn golden_dataset(dir: &Path, file: &str) -> Dataset {
    load_dataset(&normalise_golden_json(
        &fs::read_to_string(dir.join(file)).unwrap(),
    ))
    .unwrap()
}

/// The golden stores the mdb-export surface text verbatim — for an
/// out-of-range Access serial that is `"1900-01-00"` (the zeroed
/// `struct tm` under strftime), which the corpus loader's strict ISO
/// check rejects. `emit.mjs`'s `dateToDays` normalises it to
/// `1899-12-31` (`days_from_civil(1900, 1, 0)`), so re-render those
/// strings canonically before loading: the dataset — and therefore
/// the content hash — is identical either way.
fn normalise_golden_json(text: &str) -> String {
    let mut doc: Json = serde_json::from_str(text).unwrap();
    for col in doc["columns"].as_array_mut().unwrap() {
        let ty = col["type"].as_str().unwrap_or("");
        if !matches!(ty, "date" | "date_time") {
            continue;
        }
        for v in col["values"].as_array_mut().unwrap() {
            let Some(s) = v.as_str() else { continue };
            let (date_part, rest) = match s.split_once(' ') {
                Some((d, r)) => (d, r),
                None => (s, ""),
            };
            let b = date_part.as_bytes();
            if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
                continue;
            }
            let num = |a: usize, z: usize| -> Option<i64> {
                let sub = date_part.get(a..z)?;
                sub.bytes()
                    .all(|c| c.is_ascii_digit())
                    .then(|| sub.parse().ok())?
            };
            let Some((y, m, d)) = num(0, 4)
                .zip(num(5, 7))
                .zip(num(8, 10))
                .map(|((y, m), d)| (y, m, d))
            else {
                continue;
            };
            let strict_ok = (1..=12).contains(&m) && (1..=31).contains(&d);
            if strict_ok {
                continue;
            }
            // Hinnant normalisation — same arithmetic emit.mjs hashes.
            let yy = if m <= 2 { y - 1 } else { y };
            let era = if yy >= 0 { yy } else { yy - 399 } / 400;
            let yoe = yy - era * 400;
            let mp = (m + 9) % 12;
            let doy = (153 * mp + 2) / 5 + d - 1;
            let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
            let z = era * 146097 + doe; // days since 0000-03-01
            let era2 = if z >= 0 { z } else { z - 146096 } / 146097;
            let doe2 = z - era2 * 146097;
            let yoe2 = (doe2 - doe2 / 1460 + doe2 / 36524 - doe2 / 146096) / 365;
            let y2 = yoe2 + era2 * 400;
            let doy2 = doe2 - (365 * yoe2 + yoe2 / 4 - yoe2 / 100);
            let mp2 = (5 * doy2 + 2) / 153;
            let d2 = doy2 - (153 * mp2 + 2) / 5 + 1;
            let m2 = if mp2 < 10 { mp2 + 3 } else { mp2 - 9 };
            let y2 = if m2 <= 2 { y2 + 1 } else { y2 };
            *v = Json::String(if rest.is_empty() {
                format!("{y2:04}-{m2:02}-{d2:02}")
            } else {
                format!("{y2:04}-{m2:02}-{d2:02} {rest}")
            });
        }
    }
    doc.to_string()
}

/// Every ei7 golden project directory with its source file.
pub fn ei7_projects() -> Vec<(PathBuf, PathBuf)> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(golden_root().join("ei7")).unwrap() {
        for sub in std::fs::read_dir(entry.unwrap().path()).unwrap() {
            let dir = sub.unwrap().path();
            if !dir.join("project.json").exists() {
                continue;
            }
            let pj = golden_project(&dir);
            // The ei7 tree also holds `ei3_access` fixtures (Task 8
            // territory): this suite only covers the EI7 layouts.
            if !pj["source_format"].as_str().unwrap().starts_with("ei7_") {
                continue;
            }
            let src = root().join(pj["source"]["path"].as_str().unwrap());
            out.push((dir, src));
        }
    }
    out.sort();
    out
}

/// Whole-dataset equality: columns, data, metas, label sets, hash.
pub fn assert_dataset_eq(got: &Dataset, want: &Dataset, ctx: &str) {
    assert_eq!(
        got.columns().len(),
        want.columns().len(),
        "{ctx}: column count"
    );
    for (a, b) in got.columns().iter().zip(want.columns()) {
        assert_eq!(&a.name, &b.name, "{ctx}: column name");
        assert_eq!(&a.data, &b.data, "{ctx}/{}: column data", a.name);
    }
    assert_eq!(got.meta(), want.meta(), "{ctx}: column metas");
    assert_eq!(got.label_sets(), want.label_sets(), "{ctx}: label sets");
    assert_eq!(
        got.content_hash_hex(),
        want.content_hash_hex(),
        "{ctx}: content hash"
    );
}

/// Two mapped projects of the same file compare equal — the
/// Access/SQLite cross-backend check (spec §8.2).
pub fn assert_projects_eq(a: &LegacyProject, b: &LegacyProject) {
    assert_eq!(a.name, b.name);
    assert_eq!(a.legacy_id, b.legacy_id);
    assert_eq!(a.views.len(), b.views.len());
    for (va, vb) in a.views.iter().zip(&b.views) {
        assert_eq!(va.name, vb.name);
        assert_eq!(va.deleted_records, vb.deleted_records, "{}", va.name);
        assert_dataset_eq(&va.dataset, &vb.dataset, &va.name);
    }
    assert_eq!(a.datasets.len(), b.datasets.len());
    for (da, db) in a.datasets.iter().zip(&b.datasets) {
        assert_dataset_eq(da, db, da.id());
    }
    assert_eq!(a.links, b.links);
    assert_eq!(
        a.programs
            .iter()
            .map(|p| (&p.name, &p.text))
            .collect::<Vec<_>>(),
        b.programs
            .iter()
            .map(|p| (&p.name, &p.text))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        a.check_code
            .iter()
            .map(|c| (&c.view, c.blocks))
            .collect::<Vec<_>>(),
        b.check_code
            .iter()
            .map(|c| (&c.view, c.blocks))
            .collect::<Vec<_>>()
    );
    assert_eq!(a.report, b.report, "report differs across backends");
}

fn sha256_hex(text: &str) -> String {
    let digest = Sha256::digest(text.as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

fn report_json(report: &[ReportEntry]) -> Vec<Json> {
    report
        .iter()
        .map(|e| {
            serde_json::json!({
                "kind": e.kind.as_str(),
                "item": e.item,
                "reason": e.reason,
            })
        })
        .collect()
}

/// The `source_format` spellings of `project.json`.
pub fn source_format(s: &str) -> SourceFormat {
    match s {
        "ei7_access" => SourceFormat::Ei7Access,
        "ei7_sqlite" => SourceFormat::Ei7Sqlite,
        "ei7_sqlserver" => SourceFormat::Ei7SqlServer,
        "ei3_access" => SourceFormat::Ei3Access,
        other => panic!("unknown source_format {other}"),
    }
}

/// `BTreeMap` of view name → dataset for `refresh_plan`.
pub fn view_datasets(p: &LegacyProject) -> Vec<(String, Dataset)> {
    p.views
        .iter()
        .map(|v| (v.name.clone(), v.dataset.clone()))
        .collect()
}

/// Full structural comparison of a mapped project against a golden
/// `project.json` + dataset files. `source`/`source_format`-as-path
/// provenance is excluded by construction (the mapper never sees it).
pub fn assert_project_eq(got: &LegacyProject, golden_dir: &Path) {
    let pj = golden_project(golden_dir);
    assert_eq!(got.name, pj["name"].as_str().unwrap(), "project name");
    assert_eq!(
        got.legacy_id,
        pj["legacy_id"].as_str().unwrap_or(""),
        "legacy_id"
    );
    assert_eq!(
        got.format,
        source_format(pj["source_format"].as_str().unwrap()),
        "source_format"
    );

    let gviews = pj["views"].as_array().unwrap();
    assert_eq!(got.views.len(), gviews.len(), "view count");
    for (v, gv) in got.views.iter().zip(gviews) {
        assert_eq!(&v.name, gv["name"].as_str().unwrap(), "view name");
        assert_eq!(
            v.dataset.row_count() as u64,
            gv["records"].as_u64().unwrap(),
            "view {}: records",
            v.name
        );
        assert_eq!(
            v.deleted_records,
            gv["deleted_records"].as_u64().unwrap(),
            "view {}: deleted_records",
            v.name
        );
        let file = gv["dataset"].as_str().unwrap();
        let gd = golden_dataset(golden_dir, file);
        assert_dataset_eq(&v.dataset, &gd, &v.name);
        assert_eq!(
            v.dataset.content_hash_hex(),
            gv["hash"].as_str().unwrap(),
            "view {}: hash",
            v.name
        );
    }

    let glinks = pj["links"].as_array().unwrap();
    assert_eq!(got.links.len(), glinks.len(), "link count");
    for (l, gl) in got.links.iter().zip(glinks) {
        assert_eq!(l.parent, gl["parent"].as_str().unwrap(), "link parent");
        assert_eq!(l.child, gl["child"].as_str().unwrap(), "link child");
        assert_eq!(l.child_key, gl["child_key"].as_str().unwrap(), "child_key");
        assert_eq!(
            l.parent_key,
            gl["parent_key"].as_str().unwrap(),
            "parent_key"
        );
        let gkind = gl["kind"].as_str().unwrap();
        assert_eq!(
            gkind,
            match l.kind {
                epiora_core::dataset::LinkKind::Relate => "relate",
                epiora_core::dataset::LinkKind::Grid => "grid",
            },
            "link kind"
        );
        if gkind == "grid" {
            let ds = got
                .datasets
                .iter()
                .find(|d| d.id() == l.child)
                .unwrap_or_else(|| panic!("grid dataset {} missing", l.child));
            assert_eq!(
                gl["dataset"].as_str().unwrap(),
                format!("{}.dataset.json", ds.id()),
                "grid file"
            );
            assert_eq!(
                ds.content_hash_hex(),
                gl["hash"].as_str().unwrap(),
                "grid {}: hash",
                l.child
            );
            let gd = golden_dataset(golden_dir, gl["dataset"].as_str().unwrap());
            assert_dataset_eq(ds, &gd, &l.child);
        }
    }

    let gprogs = pj["programs"].as_array().unwrap();
    assert_eq!(got.programs.len(), gprogs.len(), "program count");
    for (p, gp) in got.programs.iter().zip(gprogs) {
        assert_eq!(p.name, gp["name"].as_str().unwrap(), "program name");
        assert_eq!(
            sha256_hex(&p.text),
            gp["text_sha256"].as_str().unwrap(),
            "program {}: text sha",
            p.name
        );
    }

    let gcheck = pj["check_code"].as_array().unwrap();
    assert_eq!(got.check_code.len(), gcheck.len(), "check_code count");
    for (c, gc) in got.check_code.iter().zip(gcheck) {
        assert_eq!(c.view, gc["view"].as_str().unwrap(), "check view");
        assert_eq!(
            c.blocks as u64,
            gc["blocks"].as_u64().unwrap(),
            "check {}: blocks",
            c.view
        );
    }

    let want: Vec<Json> = pj["report"].as_array().unwrap().clone();
    let mine = report_json(&got.report);
    if mine != want {
        let n = mine.len().max(want.len());
        for i in 0..n {
            let m = mine.get(i).map(|j| j.to_string());
            let w = want.get(i).map(|j| j.to_string());
            if m != w {
                eprintln!(
                    "{}: first report diff at index {i}\n  mine:  {m:?}\n  want:  {w:?}\n  mine len {} want len {}",
                    golden_dir.display(),
                    mine.len(),
                    want.len()
                );
                break;
            }
        }
    }
    assert_eq!(
        mine,
        want,
        "{}: report entries (kind/item/reason, ordered)",
        golden_dir.display()
    );
}
