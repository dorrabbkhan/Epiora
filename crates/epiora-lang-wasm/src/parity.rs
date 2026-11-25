//! The parity case runner (`parity` feature): one canonical input —
//! `{source, linked, recipe_text, program_text}` — through the same
//! calls the wasm exports make produces one canonical JSON output —
//! `{program, actions, print, stamp, view, view_sha256,
//! recipe_sha256, register_sha256}`. `native_and_wasm_bit_identical`
//! drives it over every Task-3 `.pgm` fixture and compares its bytes
//! against the direct `epiora-lang` pipeline; the wasm module exports
//! the same runner as `langCase`.

use crate::replay_result;
use epiora_lang::{parse_program, plan, print_command, register_json, LangError};
use sha2::Digest;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, LangError> {
    let bad = || LangError::RecipeReplayMismatch {
        message: format!("bad hex {s:?}"),
    };
    if !s.len().is_multiple_of(2) {
        return Err(bad());
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| bad()))
        .collect()
}

fn err(message: String) -> LangError {
    LangError::RecipeReplayMismatch { message }
}

/// The canonical run shared by the wasm export and the test.
pub fn run_case(input: &str) -> Result<String, LangError> {
    let v: serde_json::Value =
        serde_json::from_str(input).map_err(|e| err(format!("case input JSON failed: {e}")))?;
    let program_text = v["program_text"].as_str().unwrap_or("").to_string();
    let recipe_text = v["recipe_text"].as_str().unwrap_or("").to_string();
    let source = hex_decode(v["source"].as_str().unwrap_or(""))?;
    let mut linked = BTreeMap::new();
    if let Some(m) = v["linked"].as_object() {
        for (k, b) in m {
            linked.insert(
                k.clone(),
                hex_decode(b.as_str().ok_or_else(|| err("linked not hex".into()))?)?,
            );
        }
    }

    let program = parse_program(&program_text)?;
    let actions = plan(&program, epiora_lang::register());
    let (view, recipe) = replay_result(&source, &linked, &recipe_text)?;
    let materialized = view.materialize();
    let view_bytes = epiora_core::snapshot::encode_snapshot(&materialized);
    let src_ds = epiora_core::snapshot::decode_snapshot("source", 0, &source)
        .map_err(|e| err(format!("source snapshot: {e}")))?;
    let stamp = epiora_lang::working_view_stamp(&src_ds, &materialized, &recipe);
    let printed: Vec<String> = recipe.steps.iter().map(print_command).collect();
    let register_digest = sha2::Sha256::digest(register_json().as_bytes());
    let register_sha256 = register_digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    let mut out = String::from("{\"program\":");
    out.push_str(&serde_json::to_string(&program).map_err(|e| err(e.to_string()))?);
    out.push_str(",\"actions\":");
    out.push_str(&serde_json::to_string(&actions).map_err(|e| err(e.to_string()))?);
    out.push_str(",\"print\":");
    out.push_str(&serde_json::to_string(&printed).map_err(|e| err(e.to_string()))?);
    out.push_str(",\"stamp\":");
    out.push_str(&serde_json::to_string(&stamp).map_err(|e| err(e.to_string()))?);
    out.push_str(",\"view\":\"");
    out.push_str(&hex_encode(&view_bytes));
    out.push_str("\",\"view_sha256\":\"");
    out.push_str(&materialized.content_hash_hex());
    out.push_str("\",\"recipe_sha256\":\"");
    out.push_str(&recipe.sha256_hex());
    out.push_str("\",\"register_sha256\":\"");
    out.push_str(&register_sha256);
    out.push_str("\"}");
    Ok(out)
}

/// `langCase(input) -> canonical JSON` — the nodejs parity build's
/// entry point.
#[wasm_bindgen(js_name = langCase)]
pub fn lang_case(input: String) -> Result<String, JsValue> {
    run_case(&input).map_err(|e| crate::js_error(&e))
}

#[cfg(test)]
mod tests {
    //! `native_and_wasm_bit_identical` — every Task-3 `.pgm` fixture
    //! parses, plans and replays through the wasm surface
    //! (`langCase`, the exact runner the module exports) to bytes
    //! identical to the native `epiora-lang` pipeline: same program
    //! and action JSON, same printed commands, same snapshot bytes,
    //! same view/recipe hashes and `working_view` stamps.

    use super::*;
    use epiora_core::dataset::{Column, ColumnData, Dataset};
    use epiora_core::snapshot::encode_snapshot;
    use epiora_lang::Statement;
    use epiora_lang::{AsOf, Recipe};
    use epiora_legacy::batch::{decode_batch, RawData, RawTable};
    use std::path::{Path, PathBuf};

    fn root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    /// EI7's raw-read column typing (the corpus runner's rule):
    /// Access byte/int → Integer, float → Real, text/memo/guid →
    /// Text, datetime serial → DateTime seconds since 1970-01-01,
    /// YesNo → 0/1 Integer.
    fn raw_table(name: &str, raw: &RawTable) -> Dataset {
        let mut cols = Vec::new();
        for c in &raw.columns {
            let data = match &c.data {
                RawData::Int(v) => ColumnData::Integer(v.clone()),
                RawData::Float(v) => ColumnData::Real(v.clone()),
                RawData::Text(v) => ColumnData::Text(v.clone()),
                RawData::Bool(v) => {
                    ColumnData::Integer(v.iter().map(|x| x.map(i64::from)).collect())
                }
                RawData::DateTime(v) => ColumnData::DateTime(
                    v.iter()
                        .map(|x| x.map(|s| ((s - 25569.0) * 86400.0).round() as i64))
                        .collect(),
                ),
                RawData::BinarySkipped => continue,
            };
            cols.push(Column {
                name: c.name.clone(),
                data,
            });
        }
        Dataset::new(name, 1, cols).expect("raw table as dataset")
    }

    /// Every table of an .mdb as `(name, dataset)` — the session's
    /// linked set, lower-cased like `LinkedDatasets::get` expects.
    fn mdb_tables(path: &Path) -> BTreeMap<String, Dataset> {
        let mdb = epiora_mdb::Mdb::open(path).unwrap();
        let mut out = BTreeMap::new();
        for t in mdb.tables().unwrap() {
            let batch = mdb.batch(&t, 0, u32::MAX).unwrap();
            let raw = decode_batch(&batch).unwrap();
            out.insert(t.to_ascii_lowercase(), raw_table(&t, &raw));
        }
        out
    }

    /// Find an .mdb under `dir` by file stem (case-insensitive).
    fn find_mdb(dir: &Path, stem: &str) -> Option<PathBuf> {
        for e in std::fs::read_dir(dir).ok()? {
            let p = e.ok()?.path();
            if p.is_dir() {
                if let Some(f) = find_mdb(&p, stem) {
                    return Some(f);
                }
            } else if p.extension().is_some_and(|x| x == "mdb")
                && p.file_stem()
                    .is_some_and(|s| s.to_string_lossy().eq_ignore_ascii_case(stem))
            {
                return Some(p);
            }
        }
        None
    }

    /// The `.pgm` fixtures: `fixtures/ei7/*.pgm` + `fixtures/ei7/v11/*.pgm`.
    fn pgm_files() -> Vec<PathBuf> {
        let mut out = Vec::new();
        for dir in [root().join("fixtures/ei7"), root().join("fixtures/ei7/v11")] {
            for e in std::fs::read_dir(&dir).unwrap() {
                let p = e.unwrap().path();
                if p.extension().is_some_and(|x| x == "pgm") {
                    out.push(p);
                }
            }
        }
        out.sort();
        out
    }

    /// `READ '…/File.mdb':table` — the (file stem, table) a program's
    /// last READ names; the recipe replays the Manage steps that
    /// follow it.
    fn last_read(program: &epiora_lang::Program) -> (Option<String>, Vec<epiora_lang::Command>) {
        let mut source: Option<String> = None;
        let mut steps: Vec<epiora_lang::Command> = Vec::new();
        for stmt in &program.statements {
            match &stmt.node {
                Statement::Read(dref) => {
                    source = Some(dref.name.clone());
                    steps.clear();
                }
                Statement::Manage(c) => steps.push(c.clone()),
                _ => {}
            }
        }
        (source, steps)
    }

    /// The (file, table) split of a READ target — `'f.mdb':t` or `t`.
    fn read_target(s: &str) -> (Option<String>, String) {
        let unquoted = s.trim_matches(|c| c == '\'' || c == '"');
        match unquoted.rsplit_once(':') {
            Some((file, table)) => {
                let stem = Path::new(file)
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string());
                (stem, table.to_string())
            }
            None => (None, unquoted.to_string()),
        }
    }

    /// The input JSON for one `.pgm`: program text, the recipe after
    /// its last READ, and the source + linked table snapshots the
    /// program's file carries (when the sample exists).
    fn case_input(path: &Path) -> serde_json::Value {
        let text = std::fs::read_to_string(path).unwrap();
        let program = parse_program(&text).unwrap();
        let (read, steps) = last_read(&program);
        let recipe = Recipe {
            as_of: AsOf {
                date: 20_000,
                seconds: 43_200,
            },
            steps: steps.clone(),
        };
        let mut source_hex = String::new();
        let mut linked = serde_json::Map::new();
        if let Some(read_name) = &read {
            let (stem, table) = read_target(read_name);
            if let Some(stem) = stem {
                if let Some(mdb_path) = find_mdb(&root().join("fixtures/legacy/samples"), &stem) {
                    let tables = mdb_tables(&mdb_path);
                    if let Some(src) = tables.get(&table.to_ascii_lowercase()) {
                        source_hex = hex_encode(&encode_snapshot(src));
                        for (name, ds) in &tables {
                            if name != &table.to_ascii_lowercase() {
                                linked.insert(
                                    name.clone(),
                                    serde_json::Value::String(hex_encode(&encode_snapshot(ds))),
                                );
                            }
                        }
                    }
                }
            }
        }
        serde_json::json!({
            "program_text": text,
            "recipe_text": recipe.canonical_text(),
            "source": source_hex,
            "linked": linked,
            "steps": steps.len(),
        })
    }

    /// `langCase` output == the direct pipeline's, for one program —
    /// or both fail with the same `LangError` (`code` + message), so
    /// error behaviour is pinned too.
    fn assert_case(path: &Path) {
        let input = case_input(path);
        let input_text = serde_json::to_string(&input).unwrap();
        let got = run_case(&input_text);
        let native = native_case(&input);
        match (got, native) {
            (Ok(g), Ok(n)) => assert_eq!(g, n, "{path:?}: wasm != native"),
            (Err(g), Err(n)) => assert_eq!(
                (g.code(), g.to_string()),
                (n.code(), n.to_string()),
                "{path:?}: error mismatch"
            ),
            (g, n) => panic!("{path:?}: one side failed — wasm {g:?} native {n:?}"),
        }
    }

    /// The identical input through `epiora-lang` directly — the
    /// native reference `langCase` must match byte-for-byte.
    fn native_case(input: &serde_json::Value) -> Result<String, LangError> {
        let program_text = input["program_text"].as_str().unwrap().to_string();
        let recipe_text = input["recipe_text"].as_str().unwrap().to_string();
        let source = hex_decode(input["source"].as_str().unwrap())?;
        let mut linked = BTreeMap::new();
        for (k, b) in input["linked"].as_object().unwrap() {
            linked.insert(k.clone(), hex_decode(b.as_str().unwrap())?);
        }
        let program = parse_program(&program_text)?;
        let actions = plan(&program, epiora_lang::register());
        let recipe = Recipe::parse(&recipe_text)?;
        let src = epiora_core::snapshot::decode_snapshot("source", 0, &source)
            .map_err(|e| err(format!("source snapshot: {e}")))?;
        let mut tables = BTreeMap::new();
        for (name, bytes) in &linked {
            let ds = epiora_core::snapshot::decode_snapshot(name, 0, bytes)
                .map_err(|e| err(format!("linked snapshot {name:?}: {e}")))?;
            tables.insert(name.to_ascii_lowercase(), ds);
        }
        let view = epiora_lang::replay(&src, &crate::MapLinks(tables), &recipe)?;
        let materialized = view.materialize();
        let view_bytes = encode_snapshot(&materialized);
        let stamp = epiora_lang::working_view_stamp(&src, &materialized, &recipe);
        let printed: Vec<String> = recipe.steps.iter().map(print_command).collect();
        let register_digest = sha2::Sha256::digest(register_json().as_bytes());
        let register_sha256 = register_digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>();
        let mut out = String::from("{\"program\":");
        out.push_str(&serde_json::to_string(&program).map_err(|e| err(e.to_string()))?);
        out.push_str(",\"actions\":");
        out.push_str(&serde_json::to_string(&actions).map_err(|e| err(e.to_string()))?);
        out.push_str(",\"print\":");
        out.push_str(&serde_json::to_string(&printed).map_err(|e| err(e.to_string()))?);
        out.push_str(",\"stamp\":");
        out.push_str(&serde_json::to_string(&stamp).map_err(|e| err(e.to_string()))?);
        out.push_str(",\"view\":\"");
        out.push_str(&hex_encode(&view_bytes));
        out.push_str("\",\"view_sha256\":\"");
        out.push_str(&materialized.content_hash_hex());
        out.push_str("\",\"recipe_sha256\":\"");
        out.push_str(&recipe.sha256_hex());
        out.push_str("\",\"register_sha256\":\"");
        out.push_str(&register_sha256);
        out.push_str("\"}");
        Ok(out)
    }

    /// `native_and_wasm_bit_identical` — every Task-3 program.
    #[test]
    fn native_and_wasm_bit_identical() {
        let files = pgm_files();
        assert!(!files.is_empty(), "no .pgm fixtures found");
        let mut replays = 0usize;
        for f in &files {
            assert_case(f);
        }
        // Programs with a resolvable READ source must replay to real
        // bytes — a parse-only pass would be vacuous parity.
        for f in &files {
            let input = case_input(f);
            if !input["source"].as_str().unwrap().is_empty() && input["steps"].as_u64().unwrap() > 0
            {
                let out = run_case(&serde_json::to_string(&input).unwrap()).unwrap();
                let v: serde_json::Value = serde_json::from_str(&out).unwrap();
                assert!(
                    !v["view"].as_str().unwrap().is_empty() && v["stamp"].is_object(),
                    "{f:?}: expected a stamped replay"
                );
                replays += 1;
            }
        }
        assert!(replays > 0, "no fixture produced a real replay");
        eprintln!(
            "parity: {} programs bit-identical ({} stamped replays)",
            files.len(),
            replays
        );
    }
}
