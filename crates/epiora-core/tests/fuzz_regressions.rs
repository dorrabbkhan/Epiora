//! Replays committed fuzz crash inputs (spec §6.5): every file under
//! `fuzz/regressions/<target>/` goes back through the same
//! `fuzz_harness` function the cargo-fuzz target calls, so a crash
//! input that still fails makes this test fail. `.gitkeep` is ignored;
//! an empty folder is a pass. Also smokes the four harnesses on empty
//! input, garbage bytes and a real corpus dataset.

#![cfg(feature = "fuzzing")]

use epiora_core::fuzz_harness;
use std::path::{Path, PathBuf};

fn regressions_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fuzz")
        .join("regressions")
}

fn harness(target: &str) -> fn(&[u8]) {
    match target {
        "dataset_json" => fuzz_harness::dataset_json,
        "dataset_columns" => fuzz_harness::dataset_columns,
        "freq_any" => fuzz_harness::freq_any,
        "tables_any" => fuzz_harness::tables_any,
        other => panic!("unknown fuzz target {other}"),
    }
}

#[test]
fn replays_regressions() {
    for target in ["dataset_json", "dataset_columns", "freq_any", "tables_any"] {
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
    let oswego = std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/data/oswego.dataset.json"),
    )
    .expect("read oswego.dataset.json");
    let garbage = [0xff; 64];
    for data in [&[][..], &garbage[..], &oswego[..]] {
        fuzz_harness::dataset_json(data);
        fuzz_harness::dataset_columns(data);
        fuzz_harness::freq_any(data);
        fuzz_harness::tables_any(data);
    }
}
