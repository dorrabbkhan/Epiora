//! `corpus-runner <fixtures-dir>` — runs every corpus fixture through
//! `run_case` and prints one `parity_line` per fixture, in sorted path
//! order, to stdout (spec §5.2). The wasm parity run emits the same
//! lines; `diff` decides parity bit for bit.

use epiora_core::corpus::{load_dataset_dir, run_case};
use std::path::PathBuf;
use std::process::ExitCode;

const FAMILIES: &[&str] = &["special", "2x2", "stratified", "rxc", "freq"];

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("corpus-runner: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("usage: corpus-runner <fixtures-dir>")?,
    );
    let datasets =
        load_dataset_dir(&root.join("data")).map_err(|e| format!("load datasets: {e}"))?;
    let mut lines = Vec::new();
    for family in FAMILIES {
        let dir = root.join(family);
        if !dir.is_dir() {
            continue;
        }
        let mut paths = Vec::new();
        for entry in std::fs::read_dir(&dir).map_err(|e| format!("read {}: {e}", dir.display()))? {
            let path = entry
                .map_err(|e| format!("read {}: {e}", dir.display()))?
                .path();
            if path.extension().is_some_and(|x| x == "json") {
                paths.push(path);
            }
        }
        paths.sort();
        for path in paths {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("read {}: {e}", path.display()))?;
            let fixture: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| format!("parse {}: {e}", path.display()))?;
            let output =
                run_case(&fixture, &datasets).map_err(|e| format!("{}: {e}", path.display()))?;
            lines.push(output.parity_line());
        }
    }
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    for line in &lines {
        use std::io::Write;
        writeln!(lock, "{line}").map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(())
}
