//! The parity case runner (`parity` feature): one canonical input —
//! `{name, tables: [{name, batches: [hex…]}]}` — through `legacyPush`
//! → `legacyDetect` → `legacyMap` produces one canonical JSON output —
//! `{detected, project, snapshots: [hex…]}`. `native_and_wasm_bit_identical`
//! drives it over every golden project and compares its bytes against
//! the direct `epiora-legacy` pipeline and the golden files; the wasm
//! module exports the same runner as `legacyCase` so the binding adds
//! no behaviour of its own.

use crate::{detect_and_map, project_json, project_snapshots};
use epiora_legacy::LegacyError;
use wasm_bindgen::prelude::*;

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, LegacyError> {
    let bad = || LegacyError::BatchInvalid {
        reason: format!("bad hex {s:?}"),
    };
    if !s.len().is_multiple_of(2) {
        return Err(bad());
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| bad()))
        .collect()
}

/// The canonical run shared by the wasm export and the test.
pub fn run_case(input: &str) -> Result<String, LegacyError> {
    let v: serde_json::Value = serde_json::from_str(input).map_err(|e| LegacyError::Source {
        code: "legacy_batch_invalid",
        reason: format!("case input JSON failed: {e}"),
    })?;
    let name = v["name"].as_str().unwrap_or("").to_string();
    let mut handle = crate::legacy_new();
    let err = |r: &str| LegacyError::BatchInvalid {
        reason: r.to_string(),
    };
    for t in v["tables"]
        .as_array()
        .ok_or_else(|| err("tables[] missing"))?
    {
        let tname = t["name"]
            .as_str()
            .ok_or_else(|| err("table name missing"))?;
        for b in t["batches"]
            .as_array()
            .ok_or_else(|| err("batches[] missing"))?
        {
            let bytes = hex_decode(b.as_str().ok_or_else(|| err("batch not a hex string"))?)?;
            handle.source.push(tname, &bytes)?;
        }
    }
    let fmt = crate::parse_format(v["format"].as_str().unwrap_or(""));
    let (format, project) = detect_and_map(&mut handle.source, fmt, &name)?;
    let mut out = String::from("{\"detected\":");
    out.push_str(&serde_json::json!(crate::format_str(format)).to_string());
    out.push_str(",\"project\":");
    out.push_str(&project_json(&project)?);
    out.push_str(",\"snapshots\":[");
    for (i, s) in project_snapshots(&project)?.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&hex_encode(s));
        out.push('"');
    }
    out.push_str("]}");
    Ok(out)
}

/// `legacyCase(input) -> canonical JSON` — the nodejs parity build's
/// entry point.
#[wasm_bindgen(js_name = legacyCase)]
pub fn legacy_case(input: String) -> Result<String, JsValue> {
    run_case(&input).map_err(|e| crate::js_error(&e))
}

#[cfg(test)]
mod tests {
    //! `native_and_wasm_bit_identical` — every golden project maps
    //! through the wasm surface (`legacyCase`, the exact runner the
    //! module exports) to bytes identical to the native
    //! `epiora-legacy` pipeline and the golden files: same project
    //! JSON fields, same snapshot bytes, same view hashes.

    use super::*;
    use epiora_legacy::batch::{RawColumn, RawData, RawTable};
    use epiora_legacy::BatchSource;
    use epiora_legacy::LegacySource;
    use rusqlite::types::Value as SqlValue;
    use rusqlite::Connection;
    use serde_json::{json, Value};
    use std::path::{Path, PathBuf};

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn golden_root() -> PathBuf {
        root().join("fixtures/legacy/golden")
    }

    /// A `rusqlite`-backed `LegacySource` — the same storage-class
    /// union the crate's golden tests use.
    struct SqliteSource {
        conn: Connection,
        tables: Vec<String>,
    }

    impl SqliteSource {
        fn open(path: &Path) -> Self {
            let conn = Connection::open(path).unwrap();
            let tables = conn
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type IN ('table','view') \
                     AND name NOT LIKE 'sqlite_%' ORDER BY rowid",
                )
                .unwrap()
                .query_map([], |r| r.get::<_, String>(0))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            Self { conn, tables }
        }
    }

    impl LegacySource for SqliteSource {
        fn tables(&self) -> Vec<String> {
            self.tables.clone()
        }

        fn table(&mut self, name: &str) -> Result<RawTable, LegacyError> {
            let real = self
                .tables
                .iter()
                .find(|t| t.eq_ignore_ascii_case(name))
                .cloned()
                .unwrap();
            let quoted = format!("\"{}\"", real.replace('"', "\"\""));
            let decls: Vec<String> = self
                .conn
                .prepare(&format!("PRAGMA table_info({quoted})"))
                .unwrap()
                .query_map([], |r| r.get::<_, String>(2))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let mut stmt = self
                .conn
                .prepare(&format!("SELECT * FROM {quoted}"))
                .unwrap();
            let ncols = stmt.column_count();
            let col_names: Vec<String> = (0..ncols)
                .map(|i| stmt.column_name(i).unwrap().to_string())
                .collect();
            let mut rows: Vec<Vec<SqlValue>> = Vec::new();
            let mut q = stmt.query([]).unwrap();
            while let Some(r) = q.next().unwrap() {
                rows.push(
                    (0..ncols)
                        .map(|i| r.get::<_, SqlValue>(i).unwrap_or(SqlValue::Null))
                        .collect(),
                );
            }
            let mut columns = Vec::with_capacity(ncols);
            for (i, cname) in col_names.iter().enumerate() {
                let vals: Vec<&SqlValue> = rows.iter().map(|r| &r[i]).collect();
                let has = |f: &dyn Fn(&&SqlValue) -> bool| {
                    vals.iter().any(|v| !matches!(v, SqlValue::Null) && f(v))
                };
                let data = if has(&|v| matches!(v, SqlValue::Text(_))) {
                    RawData::Text(
                        vals.iter()
                            .map(|v| match v {
                                SqlValue::Null => None,
                                SqlValue::Text(s) => Some(s.clone()),
                                SqlValue::Integer(i) => Some(i.to_string()),
                                SqlValue::Real(f) => Some(format!("{f}")),
                                SqlValue::Blob(_) => None,
                            })
                            .collect(),
                    )
                } else if has(&|v| matches!(v, SqlValue::Real(_))) {
                    RawData::Float(
                        vals.iter()
                            .map(|v| match v {
                                SqlValue::Integer(i) => Some(*i as f64),
                                SqlValue::Real(f) => Some(*f),
                                _ => None,
                            })
                            .collect(),
                    )
                } else if has(&|v| matches!(v, SqlValue::Integer(_))) {
                    RawData::Int(
                        vals.iter()
                            .map(|v| match v {
                                SqlValue::Integer(i) => Some(*i),
                                _ => None,
                            })
                            .collect(),
                    )
                } else if has(&|v| matches!(v, SqlValue::Blob(_))) {
                    RawData::BinarySkipped
                } else {
                    let decl = decls.get(i).cloned().unwrap_or_default().to_lowercase();
                    if decl.contains("blob") {
                        RawData::BinarySkipped
                    } else {
                        RawData::Int(vals.iter().map(|_| None).collect())
                    }
                };
                columns.push(RawColumn {
                    name: cname.clone(),
                    source_type: 0,
                    data,
                });
            }
            Ok(RawTable {
                name: real,
                code_page: None,
                decode_errors: 0,
                columns,
            })
        }
    }

    /// The inverse of `batch::decode_batch` — EPTB v1 from a RawTable
    /// (the same bytes the worker's TS encoder writes).
    fn encode_table(t: &RawTable) -> Vec<u8> {
        fn raw_len(d: &RawData) -> usize {
            match d {
                RawData::Int(v) => v.len(),
                RawData::Float(v) => v.len(),
                RawData::Bool(v) => v.len(),
                RawData::Text(v) => v.len(),
                RawData::DateTime(v) => v.len(),
                RawData::BinarySkipped => 0,
            }
        }
        let rows = t
            .columns
            .iter()
            .find_map(|c| match &c.data {
                RawData::BinarySkipped => None,
                other => Some(raw_len(other)),
            })
            .unwrap_or(0) as u32;
        let mut b = Vec::new();
        b.extend_from_slice(b"EPTB");
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&rows.to_le_bytes());
        b.extend_from_slice(&(t.columns.len() as u16).to_le_bytes());
        let mut ext = Vec::new();
        ext.push(1u8);
        ext.extend_from_slice(&(t.name.len() as u16).to_le_bytes());
        ext.extend_from_slice(t.name.as_bytes());
        if let Some(cp) = &t.code_page {
            ext.push(2u8);
            ext.extend_from_slice(&(cp.len() as u16).to_le_bytes());
            ext.extend_from_slice(cp.as_bytes());
        }
        if t.decode_errors > 0 {
            ext.push(3u8);
            ext.extend_from_slice(&4u16.to_le_bytes());
            ext.extend_from_slice(&t.decode_errors.to_le_bytes());
        }
        b.extend_from_slice(&(ext.len() as u16).to_le_bytes());
        b.extend_from_slice(&ext);
        let bitmap_len = (rows as usize).div_ceil(8);
        for c in &t.columns {
            b.extend_from_slice(&(c.name.len() as u32).to_le_bytes());
            b.extend_from_slice(c.name.as_bytes());
            b.push(c.source_type);
            let kind = match &c.data {
                RawData::Int(_) => 0u8,
                RawData::Float(_) => 1,
                RawData::Text(_) => 2,
                RawData::Bool(_) => 3,
                RawData::DateTime(_) => 4,
                RawData::BinarySkipped => 5,
            };
            b.push(kind);
            if kind == 5 {
                continue;
            }
            let missing: Vec<bool> = match &c.data {
                RawData::Int(v) => v.iter().map(|x| x.is_none()).collect(),
                RawData::Float(v) => v.iter().map(|x| x.is_none()).collect(),
                RawData::Text(v) => v.iter().map(|x| x.is_none()).collect(),
                RawData::Bool(v) => v.iter().map(|x| x.is_none()).collect(),
                RawData::DateTime(v) => v.iter().map(|x| x.is_none()).collect(),
                RawData::BinarySkipped => Vec::new(),
            };
            let mut bitmap = vec![0u8; bitmap_len];
            for (r, m) in missing.iter().enumerate() {
                if *m {
                    bitmap[r / 8] |= 1 << (r % 8);
                }
            }
            b.extend_from_slice(&bitmap);
            match &c.data {
                RawData::Int(v) => {
                    for x in v.iter().flatten() {
                        b.extend_from_slice(&x.to_le_bytes());
                    }
                }
                RawData::Float(v) | RawData::DateTime(v) => {
                    for x in v.iter().flatten() {
                        b.extend_from_slice(&x.to_le_bytes());
                    }
                }
                RawData::Bool(v) => {
                    for x in v.iter().flatten() {
                        b.push(u8::from(*x));
                    }
                }
                RawData::Text(v) => {
                    let mut blob = Vec::new();
                    let mut offs = Vec::new();
                    for s in v.iter().flatten() {
                        offs.push(blob.len() as u32);
                        blob.extend_from_slice(s.as_bytes());
                        blob.push(0);
                    }
                    b.extend_from_slice(&(blob.len() as u32).to_le_bytes());
                    b.extend_from_slice(&blob);
                    for o in offs {
                        b.extend_from_slice(&o.to_le_bytes());
                    }
                }
                RawData::BinarySkipped => {}
            }
        }
        b
    }

    /// Every batch of one source file as `{name, batches:[hex]}` —
    /// `.mdb`/`.accdb` through `epiora-mdb`, `.db` through the sqlite
    /// source + EPTB encoder.
    fn case_tables(path: &Path) -> Vec<Value> {
        let ext = path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        if ext == "db" {
            let mut src = SqliteSource::open(path);
            return src
                .tables()
                .iter()
                .map(|t| {
                    let raw = src.table(t).unwrap();
                    json!({"name": t, "batches": [hex_encode(&encode_table(&raw))]})
                })
                .collect();
        }
        let mdb = epiora_mdb::Mdb::open(path).unwrap();
        mdb.tables()
            .unwrap()
            .iter()
            .map(|t| {
                let batch = mdb.batch(t, 0, u32::MAX).unwrap();
                json!({"name": t, "batches": [hex_encode(&batch)]})
            })
            .collect()
    }

    /// One `legacyCase` run asserted against a golden `project.json`.
    fn assert_case(dir: &Path) {
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
    }

    /// The identical batches through `epiora-legacy` directly — the
    /// native reference `legacyCase` must match byte-for-byte.
    fn native_case(input: &str) -> String {
        let v: Value = serde_json::from_str(input).unwrap();
        let name = v["name"].as_str().unwrap_or("").to_string();
        let mut src = BatchSource::new();
        for t in v["tables"].as_array().unwrap() {
            let tname = t["name"].as_str().unwrap();
            for b in t["batches"].as_array().unwrap() {
                src.push(tname, &hex_decode(b.as_str().unwrap()).unwrap())
                    .unwrap();
            }
        }
        let fmt = crate::parse_format(v["format"].as_str().unwrap_or(""));
        let (format, project) = detect_and_map(&mut src, fmt, &name).unwrap();
        let mut out = String::from("{\"detected\":");
        out.push_str(&serde_json::json!(crate::format_str(format)).to_string());
        out.push_str(",\"project\":");
        out.push_str(&project_json(&project).unwrap());
        out.push_str(",\"snapshots\":[");
        for (i, s) in project_snapshots(&project).unwrap().iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            out.push_str(&hex_encode(s));
            out.push('"');
        }
        out.push_str("]}");
        out
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
        assert!(!dirs.is_empty(), "no golden projects found");
        for dir in &dirs {
            assert_case(dir);
        }
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
}
