//! `native_and_wasm_bit_identical` — every Task-3 `.pgm` fixture
//! parses, plans and replays through the wasm surface (`langCase`,
//! the exact runner the module exports) to bytes identical to the
//! native `epiora-lang` pipeline: same program and action JSON, same
//! printed commands, same snapshot bytes, same view/recipe hashes and
//! `working_view` stamps. Each case's `{input, output}` line lands in
//! `target/parity/epiora-lang-wasm-cases.jsonl`, which
//! `parity/run.mjs` replays through the real wasm32 `langCase`; the
//! workflow diffs the two files byte-for-byte.

use super::cases::{case_input, native_case, pgm_files, root};
use super::run_case;
use epiora_lang::LangError;
use std::path::{Path, PathBuf};

/// The JSONL the nodejs parity run replays.
fn cases_path() -> PathBuf {
    root().join("target/parity/epiora-lang-wasm-cases.jsonl")
}

/// One `{"input":…,"ok":…}` or `{"input":…,"err":{…}}` parity line —
/// `run.mjs` must reproduce it byte-for-byte.
fn parity_line(input: &str, result: &Result<String, LangError>) -> String {
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

/// `langCase` output == the direct pipeline's, for one program —
/// or both fail with the same `LangError` (`code` + message), so
/// error behaviour is pinned too. Returns the case's parity line.
fn assert_case(path: &Path) -> String {
    let input = case_input(path);
    let input_text = serde_json::to_string(&input).unwrap();
    let got = run_case(&input_text);
    let native = native_case(&input);
    match (&got, &native) {
        (Ok(g), Ok(n)) => assert_eq!(g, n, "{path:?}: wasm != native"),
        (Err(g), Err(n)) => assert_eq!(
            (g.code(), g.to_string()),
            (n.code(), n.to_string()),
            "{path:?}: error mismatch"
        ),
        (g, n) => panic!("{path:?}: one side failed — wasm {g:?} native {n:?}"),
    }
    parity_line(&input_text, &native)
}

/// `native_and_wasm_bit_identical` — every Task-3 program.
#[test]
fn native_and_wasm_bit_identical() {
    let files = pgm_files();
    assert!(!files.is_empty(), "no .pgm fixtures found");
    let mut lines = Vec::with_capacity(files.len());
    let mut replays = 0usize;
    for f in &files {
        lines.push(assert_case(f));
    }
    // Programs with a resolvable READ source must replay to real
    // bytes — a parse-only pass would be vacuous parity.
    for f in &files {
        let input = case_input(f);
        if !input["source"].as_str().unwrap().is_empty() && input["steps"].as_u64().unwrap() > 0 {
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
    write_cases(&lines);
    eprintln!(
        "parity: {} programs bit-identical ({} stamped replays)",
        files.len(),
        replays
    );
}
