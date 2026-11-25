//! `native_and_wasm_bit_identical` — every golden project maps
//! through the wasm surface (`legacyCase`, the exact runner the
//! module exports) to bytes identical to the native
//! `epiora-legacy` pipeline and the golden files: same project
//! JSON fields, same snapshot bytes, same view hashes. Each case's
//! `{input, output}` line lands in
//! `target/parity/epiora-legacy-wasm-cases.jsonl`, which
//! `parity/run.mjs` replays through the real wasm32 `legacyCase`;
//! the workflow diffs the two files byte-for-byte.

use super::cases::{case_tables, golden_root, native_case, root};
use super::{hex_decode, run_case};
use epiora_legacy::LegacyError;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

/// The JSONL the nodejs parity run replays.
fn cases_path() -> PathBuf {
    root().join("target/parity/epiora-legacy-wasm-cases.jsonl")
}

/// One `{"input":…,"ok":…}` or `{"input":…,"err":{…}}` parity line —
/// `run.mjs` must reproduce it byte-for-byte.
fn parity_line(input: &str, result: &Result<String, LegacyError>) -> String {
    let mut l = String::from("{\"input\":");
    l.push_str(&serde_json::to_string(input).unwrap());
    match result {
        Ok(out) => {
            l.push_str(",\"ok\":");
            l.push_str(&serde_json::to_string(out).unwrap());
        }
        Err(e) => {
            l.push_str(",\"err\":{\"code\":");
            l.push_str(&serde_json::to_string(e.code()).unwrap());
            l.push_str(",\"message\":");
            l.push_str(&serde_json::to_string(&e.to_string()).unwrap());
            l.push('}');
        }
    }
    l.push('}');
    l
}

/// The native case file `parity/run.mjs` reads.
fn write_cases(lines: &[String]) {
    let path = cases_path();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, format!("{}\n", lines.join("\n"))).unwrap();
    eprintln!(
        "parity: wrote {} case lines to {}",
        lines.len(),
        path.display()
    );
}

/// One `legacyCase` run asserted against a golden `project.json`;
/// returns the case's parity line.
fn assert_case(dir: &Path) -> String {
    let pj_text = std::fs::read_to_string(dir.join("project.json")).unwrap();
    let pj: Value = serde_json::from_str(&pj_text).unwrap();
    let rel = pj["source"]["path"].as_str().unwrap();
    let source_path = if root().join(rel).is_file() {
        root().join(rel)
    } else {
        dir.join(rel)
    };
    let input = json!({
        "name": source_path.file_stem().unwrap_or_default().to_string_lossy(),
        "format": pj["source_format"].as_str().unwrap(),
        "tables": case_tables(&source_path),
    })
    .to_string();

    // The wasm surface…
    let got_text = run_case(&input).unwrap_or_else(|e| panic!("{dir:?}: {e}"));
    let got: Value = serde_json::from_str(&got_text).unwrap();
    // …and the direct native pipeline over the same batches.
    let native = native_case(&input);

    assert_eq!(got_text, native, "{dir:?}: wasm != native");
    assert_eq!(
        got["detected"].as_str().unwrap(),
        pj["source_format"].as_str().unwrap(),
        "{dir:?}: detected"
    );
    let gp = &got["project"];
    assert_eq!(
        gp["name"].as_str().unwrap(),
        pj["name"].as_str().unwrap(),
        "{dir:?} name"
    );
    assert_eq!(
        gp["legacy_id"].as_str().unwrap_or(""),
        pj["legacy_id"].as_str().unwrap_or(""),
        "{dir:?} legacy_id"
    );
    let gviews = pj["views"].as_array().unwrap();
    let views = gp["views"].as_array().unwrap();
    assert_eq!(views.len(), gviews.len(), "{dir:?} view count");
    for (v, gv) in views.iter().zip(gviews) {
        assert_eq!(v["name"], gv["name"], "{dir:?} view name");
        assert_eq!(v["records"], gv["records"], "{dir:?} records");
        assert_eq!(
            v["deleted_records"], gv["deleted_records"],
            "{dir:?} deleted"
        );
        assert_eq!(v["hash"], gv["hash"], "{dir:?} view hash");
    }
    let glinks = pj["links"].as_array().unwrap();
    let links = gp["links"].as_array().unwrap();
    assert_eq!(links.len(), glinks.len(), "{dir:?} link count");
    for (l, gl) in links.iter().zip(glinks) {
        for k in ["parent", "child", "child_key", "parent_key", "kind"] {
            assert_eq!(l[k], gl[k], "{dir:?} link {k}");
        }
    }
    let gprogs = pj["programs"].as_array().unwrap();
    let progs = gp["programs"].as_array().unwrap();
    assert_eq!(progs.len(), gprogs.len(), "{dir:?} program count");
    for (p, g) in progs.iter().zip(gprogs) {
        assert_eq!(p["name"], g["name"], "{dir:?} program name");
        assert_eq!(p["text_sha256"], g["text_sha256"], "{dir:?} program sha");
    }
    let gcheck = pj["check_code"].as_array().unwrap();
    let checks = gp["check_code"].as_array().unwrap();
    assert_eq!(checks.len(), gcheck.len(), "{dir:?} check_code count");
    for (c, g) in checks.iter().zip(gcheck) {
        assert_eq!(c["view"], g["view"], "{dir:?} check view");
        assert_eq!(c["blocks"], g["blocks"], "{dir:?} check blocks");
    }
    let greports = pj["report"].as_array().unwrap();
    let reports = gp["report"].as_array().unwrap();
    assert_eq!(reports.len(), greports.len(), "{dir:?} report count");
    for (r, g) in reports.iter().zip(greports) {
        for k in ["kind", "item", "reason"] {
            assert_eq!(r[k], g[k], "{dir:?} report {k}");
        }
    }
    // Snapshot bytes: decode each, the dataset's content hash must
    // equal the golden view/link hash.
    let snaps = got["snapshots"].as_array().unwrap();
    assert_eq!(
        snaps.len(),
        views.len() + gp["datasets"].as_array().unwrap().len()
    );
    for (i, gv) in gviews.iter().enumerate() {
        let bytes = hex_decode(snaps[i].as_str().unwrap()).unwrap();
        let ds = epiora_core::snapshot::decode_snapshot("v", 1, &bytes).unwrap();
        assert_eq!(
            ds.content_hash_hex(),
            gv["hash"].as_str().unwrap(),
            "{dir:?} snapshot hash"
        );
    }
    let mut off = views.len();
    for gl in glinks {
        if gl["kind"].as_str().unwrap() != "grid" {
            continue;
        }
        let bytes = hex_decode(snaps[off].as_str().unwrap()).unwrap();
        let ds = epiora_core::snapshot::decode_snapshot("g", 1, &bytes).unwrap();
        assert_eq!(
            ds.content_hash_hex(),
            gl["hash"].as_str().unwrap(),
            "{dir:?} grid snapshot hash"
        );
        off += 1;
    }
    parity_line(&input, &Ok(native))
}

/// `native_and_wasm_bit_identical` — every golden project.
#[test]
fn native_and_wasm_bit_identical() {
    let g = golden_root();
    let mut dirs = Vec::new();
    for section in ["ei7", "ei3", "reimport"] {
        let base = g.join(section);
        if !base.is_dir() {
            continue;
        }
        collect(&base, &mut dirs);
    }
    dirs.sort();
    assert!(!dirs.is_empty(), "no golden projects found");
    let mut lines = Vec::with_capacity(dirs.len());
    for dir in &dirs {
        lines.push(assert_case(dir));
    }
    write_cases(&lines);
    eprintln!("parity: {} golden projects bit-identical", dirs.len());
}

/// Directories holding a `project.json` (recursive — ei7 projects
/// nest two levels).
fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    if dir.join("project.json").is_file() {
        out.push(dir.to_path_buf());
        return;
    }
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            collect(&p, out);
        }
    }
}
