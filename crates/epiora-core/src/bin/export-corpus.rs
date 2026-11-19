//! `export-corpus <out-dir>` (v1.0 Task 17, spec §8.3): writes, for
//! every corpus fixture plus the built-in `hostile_text` dataset
//! case, `<out>/<case>/analysis.R` (and `data.csv` when the case
//! reads a dataset, and `expected.json`). `r-export.yml` runs each
//! script and compares the `EPIORA` lines to `expected.json`.

use epiora_core::corpus::{corpus_lines_with, load_datasets};
use epiora_core::export::corpus_datasets::{dataset_echo_export, hostile_text_dataset};
use epiora_core::export::corpus_export::case_export;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn expected_stats(export: &epiora_core::export::corpus_export::CaseExport) -> Value {
    let mut values = Map::new();
    for (path, stat) in &export.expected {
        values.insert(
            path.clone(),
            json!({"value": stat.value, "class": stat.class}),
        );
    }
    json!({"kind": "stats", "values": Value::Object(values)})
}

fn write_case(dir: &Path, script: &str, data: Option<&[u8]>, expected: &Value) {
    fs::create_dir_all(dir).expect("create case dir");
    fs::write(dir.join("analysis.R"), script).expect("analysis.R");
    if let Some(csv) = data {
        fs::write(dir.join("data.csv"), csv).expect("data.csv");
    }
    fs::write(
        dir.join("expected.json"),
        serde_json::to_string_pretty(expected).expect("expected serializes"),
    )
    .expect("expected.json");
}

fn main() {
    let out: PathBuf = std::env::args().nth(1).map(Into::into).unwrap_or_else(|| {
        eprintln!("usage: export-corpus <out-dir> [fixtures-dir]");
        std::process::exit(2);
    });
    let root: PathBuf = std::env::args().nth(2).map_or_else(
        || Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures"),
        Into::into,
    );
    let datasets = load_datasets(&root).unwrap_or_else(|e| panic!("datasets: {e}"));
    let lines = corpus_lines_with(&root, &datasets).unwrap_or_else(|e| panic!("corpus: {e}"));
    let mut n = 0_usize;
    let mut failures = Vec::new();
    for line in &lines {
        match case_export(&line.fixture, &line.output.actual, &datasets) {
            Ok(export) => {
                let dir = out.join(line.rel_path.trim_end_matches(".json"));
                write_case(
                    &dir,
                    &export.script,
                    export.data_csv.as_deref(),
                    &expected_stats(&export),
                );
                n += 1;
            }
            Err(e) => failures.push(format!("{}: {e}", line.rel_path)),
        }
    }
    // The built-in hostile-text dataset-echo case (Review Focus 4).
    let ds = hostile_text_dataset().expect("hostile dataset is well-formed");
    let (script, csv, cells) = dataset_echo_export(&ds);
    write_case(
        &out.join("dataset_echo/hostile_text"),
        &script,
        Some(&csv),
        &json!({"kind": "dataset", "cells": Value::Object(cells)}),
    );
    n += 1;
    for f in &failures {
        eprintln!("export-corpus: {f}");
    }
    if !failures.is_empty() {
        std::process::exit(1);
    }
    println!("export-corpus: wrote {n} cases to {}", out.display());
}
