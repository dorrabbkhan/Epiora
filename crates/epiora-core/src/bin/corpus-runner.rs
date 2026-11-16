//! `corpus-runner <fixtures-dir>` — runs every corpus fixture through
//! `run_case` and prints one `parity_line` per fixture, in sorted path
//! order, to stdout (spec §5.2). The wasm parity run emits the same
//! lines; `diff` decides parity bit for bit.
//!
//! `corpus-runner --snapshot-hex <dataset-id> [fixtures-dir]` prints
//! the lowercase hex of a dataset's `encode_snapshot` bytes.
//!
//! `corpus-runner --hash-golden [fixtures-dir]` prints one TSV row
//! per corpus line — the fixture id, its repo-relative path, the
//! SHA-256 of the fixture bytes, the `inputs.dataset` file path and
//! SHA-256 (or `-`), and the SHA-256 of the parity line — the
//! `results_unchanged` golden of v1.0 Task 10.

use epiora_core::corpus::{corpus_lines, dataset_files, load_dataset_dir};
use sha2::Digest;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("corpus-runner: {e}");
            ExitCode::FAILURE
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let digest = sha2::Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for b in digest {
        let _ = write!(out, "{b:02x}");
    }
    out
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args()
        .skip(1)
        .skip_while(|a| a == "--")
        .collect();
    // `corpus-runner --snapshot-hex <dataset-id> [fixtures-dir]` prints
    // the lowercase hex of the dataset's `encode_snapshot` bytes (v0.3
    // Task 1); the Node binding test compares them against the wasm
    // encoder byte for byte.
    if args.first().is_some_and(|a| a == "--snapshot-hex") {
        let id = args
            .get(1)
            .ok_or("usage: corpus-runner --snapshot-hex <dataset-id> [fixtures-dir]")?;
        let root = PathBuf::from(args.get(2).map(String::as_str).unwrap_or("fixtures"));
        let datasets =
            load_dataset_dir(&root.join("data")).map_err(|e| format!("load datasets: {e}"))?;
        let dataset = datasets
            .get(id)
            .ok_or_else(|| format!("dataset {id:?} not found"))?;
        let mut hex = String::new();
        for byte in epiora_core::snapshot::encode_snapshot(dataset) {
            hex.push_str(&format!("{byte:02x}"));
        }
        println!("{hex}");
        return Ok(());
    }
    if args.first().is_some_and(|a| a == "--hash-golden") {
        let root = PathBuf::from(args.get(1).map(String::as_str).unwrap_or("fixtures"));
        return hash_golden(&root);
    }
    let root = PathBuf::from(args.first().ok_or("usage: corpus-runner <fixtures-dir>")?);
    let lines = corpus_lines(&root)?;
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    for line in &lines {
        use std::io::Write;
        writeln!(lock, "{}", line.output.parity_line()).map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(())
}

/// Prints the `results_unchanged` golden rows. A dataset-backed
/// fixture's parity line depends on `data/<id>.dataset.json` as well
/// as the fixture file, so both hashes go in the row.
fn hash_golden(root: &Path) -> Result<(), String> {
    let datasets = dataset_files(root)?;
    let repo_root = root.to_string_lossy().trim_end_matches('/').to_string();
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    for line in corpus_lines(root)? {
        let fixture_path = format!("{repo_root}/{}", line.rel_path);
        let fixture_sha = sha256_hex(
            &std::fs::read(root.join(&line.rel_path))
                .map_err(|e| format!("read {fixture_path}: {e}"))?,
        );
        let (dataset_path, dataset_sha) = match line.fixture["inputs"]["dataset"].as_str() {
            Some(id) => match datasets.get(id) {
                Some(p) => (
                    format!("{repo_root}/{}", p.strip_prefix(root).unwrap_or(p).to_string_lossy()),
                    sha256_hex(
                        &std::fs::read(p).map_err(|e| format!("read {}: {e}", p.display()))?,
                    ),
                ),
                None => return Err(format!("{fixture_path}: unknown dataset {id:?}")),
            },
            None => ("-".to_string(), "-".to_string()),
        };
        let parity_sha = sha256_hex(line.output.parity_line().as_bytes());
        use std::io::Write;
        writeln!(
            lock,
            "{}\t{fixture_path}\t{fixture_sha}\t{dataset_path}\t{dataset_sha}\t{parity_sha}",
            line.output.id
        )
        .map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(())
}
