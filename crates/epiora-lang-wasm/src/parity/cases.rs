//! Case construction for `native_and_wasm_bit_identical`: the `.pgm`
//! fixture list, the `case_input` builder (program text, the recipe
//! after its last READ, the source + linked snapshots its file
//! carries) and `native_case` — the direct `epiora-lang` pipeline the
//! `langCase` runner must match byte-for-byte.

use super::{err, hex_decode, hex_encode};
use epiora_core::dataset::{Column, ColumnData, Dataset, DatasetLink, LinkKind};
use epiora_core::snapshot::encode_snapshot;
use epiora_lang::Statement;
use epiora_lang::{
    parse_expr, parse_program, plan, print_command, register_json, AsOf, Command, DatasetRef,
    LangError, Recipe, StoredLinks,
};
use epiora_legacy::batch::{decode_batch, RawData, RawTable};
use sha2::Digest;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub fn root() -> PathBuf {
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
            RawData::Bool(v) => ColumnData::Integer(v.iter().map(|x| x.map(i64::from)).collect()),
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
pub fn pgm_files() -> Vec<PathBuf> {
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
pub fn case_input(path: &Path) -> serde_json::Value {
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

/// The identical input through `epiora-lang` directly — the
/// native reference `langCase` must match byte-for-byte.
pub fn native_case(input: &serde_json::Value) -> Result<String, LangError> {
    let program_text = input["program_text"].as_str().unwrap().to_string();
    let recipe_text = input["recipe_text"].as_str().unwrap().to_string();
    let source = hex_decode(input["source"].as_str().unwrap())?;
    let mut linked = BTreeMap::new();
    for (k, b) in input["linked"].as_object().unwrap() {
        linked.insert(k.clone(), hex_decode(b.as_str().unwrap())?);
    }
    let links: Vec<DatasetLink> = match &input["links"] {
        serde_json::Value::Array(_) => serde_json::from_value(input["links"].clone())
            .map_err(|e| err(format!("links JSON failed: {e}")))?,
        _ => Vec::new(),
    };
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
    let view = epiora_lang::replay(&src, &StoredLinks::new(tables, links), &recipe)?;
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

/// A plain `(name, [col,…])` dataset for the hand-built cases.
fn ds(name: &str, cols: Vec<(&str, ColumnData)>) -> Dataset {
    Dataset::new(
        name,
        1,
        cols.into_iter()
            .map(|(n, d)| Column {
                name: n.to_string(),
                data: d,
            })
            .collect(),
    )
    .expect("dataset")
}

/// Task 13b's two extra cases — `(name, input)` pairs in `case_input`'s
/// shape, plus `links` where a case carries stored `dataset_links`:
/// a keyless `RELATE` resolved through the stored link, and a
/// `CANCEL READ` program (a `Manage` step that resets the view to the
/// source — never a stored recipe step, but legal in recipe text).
pub fn extra_cases() -> Vec<(&'static str, serde_json::Value)> {
    let as_of = AsOf {
        date: 20_000,
        seconds: 43_200,
    };
    let parent = ds(
        "parent",
        vec![
            (
                "k",
                ColumnData::Text(
                    vec!["3", "1", "2"]
                        .into_iter()
                        .map(|s| Some(s.to_string()))
                        .collect(),
                ),
            ),
            ("v", ColumnData::Integer(vec![Some(10), Some(20), Some(30)])),
        ],
    );
    let child = ds(
        "child",
        vec![
            (
                "ck",
                ColumnData::Text(
                    vec!["1", "1", "1"]
                        .into_iter()
                        .map(|s| Some(s.to_string()))
                        .collect(),
                ),
            ),
            ("h", ColumnData::Integer(vec![Some(7), Some(8), Some(9)])),
        ],
    );
    let link = DatasetLink {
        parent: "parent".into(),
        child: "child".into(),
        parent_key: "k".into(),
        child_key: "ck".into(),
        kind: LinkKind::Relate,
    };

    let relate_recipe = Recipe {
        as_of,
        steps: vec![Command::Relate {
            dataset: DatasetRef::named("child"),
            keys: vec![],
        }],
    };
    let relate = serde_json::json!({
        "program_text": "READ 'p.mdb':parent\nRELATE child\nFREQ k",
        "recipe_text": relate_recipe.canonical_text(),
        "source": hex_encode(&encode_snapshot(&parent)),
        "linked": {"child": hex_encode(&encode_snapshot(&child))},
        "links": [serde_json::to_value(&link).unwrap()],
        "steps": 1,
    });

    let cancel_recipe = Recipe {
        as_of,
        steps: vec![
            Command::Select(parse_expr("v > 15").unwrap()),
            Command::CancelRead,
        ],
    };
    let cancel = serde_json::json!({
        "program_text": "READ 'p.mdb':parent\nSELECT v > 15\nCANCEL READ",
        "recipe_text": cancel_recipe.canonical_text(),
        "source": hex_encode(&encode_snapshot(&parent)),
        "linked": {},
        "steps": 2,
    });

    vec![
        ("keyless_relate_with_stored_link", relate),
        ("cancel_read_program", cancel),
    ]
}
