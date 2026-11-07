//! Replays committed fuzz crash inputs (v0.3 spec §6.5): every file
//! under `fuzz/regressions/<target>/` goes back through the same
//! library function the cargo-fuzz target calls
//! (`epiora_import::text_import`, `epiora_import::sheet_import`,
//! `epiora_import::diff_any`), so a crash input that still fails makes
//! this test fail. `.gitkeep` is ignored; a missing or empty folder is
//! a pass. Also smokes the three harnesses on empty input and garbage
//! bytes.

#![cfg(feature = "fuzzing")]

use std::path::{Path, PathBuf};

fn regressions_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz")
        .join("regressions")
}

fn harness(target: &str) -> fn(&[u8]) {
    match target {
        "text_import" => epiora_import::text_import,
        "sheet_import" => epiora_import::sheet_import,
        "diff_any" => epiora_import::diff_any,
        other => panic!("unknown fuzz target {other}"),
    }
}

#[test]
fn replays_regressions() {
    for target in ["text_import", "sheet_import", "diff_any"] {
        let dir = regressions_dir().join(target);
        if !dir.is_dir() {
            continue;
        }
        let run = harness(target);
        let mut inputs: Vec<PathBuf> = std::fs::read_dir(&dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|e| e.unwrap_or_else(|e| panic!("dir entry: {e}")).path())
            .filter(|p| p.is_file() && p.file_name().is_some_and(|n| n != ".gitkeep"))
            .collect();
        inputs.sort();
        for input in inputs {
            let bytes =
                std::fs::read(&input).unwrap_or_else(|e| panic!("read {}: {e}", input.display()));
            run(&bytes);
        }
    }
}

#[test]
fn harness_handles_empty_and_garbage() {
    let garbage = [0xff; 64];
    for data in [&[][..], &garbage[..]] {
        epiora_import::text_import(data);
        epiora_import::sheet_import(data);
        epiora_import::diff_any(data);
    }
}
