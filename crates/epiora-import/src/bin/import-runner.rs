//! `import-runner <fixtures-import-dir>` — runs every
//! `fixtures/import/<case>` through `epiora_import::import` with the
//! `settings` overrides from that case's `expected.json` and prints one
//! [`parity_line`] per case, in sorted case order, to stdout (plan
//! Task 6). The wasm parity run emits the same lines; the parity
//! `diff.sh` compares them bit for bit.

use epiora_import::{parity_line, ImportSettings};
use std::path::Path;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("import-runner: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let root = std::env::args()
        .nth(1)
        .ok_or("usage: import-runner <fixtures-import-dir>")?;
    let mut cases = Vec::new();
    for entry in std::fs::read_dir(&root).map_err(|e| format!("read {root}: {e}"))? {
        let path = entry.map_err(|e| format!("read {root}: {e}"))?.path();
        if path.is_dir() {
            cases.push(path);
        }
    }
    cases.sort();
    let stdout = std::io::stdout();
    let mut lock = stdout.lock();
    for case in cases {
        use std::io::Write;
        writeln!(lock, "{}", case_line(&case)?).map_err(|e| format!("stdout: {e}"))?;
    }
    Ok(())
}

/// One case's line: `expected.json` supplies the input file name and
/// the `ImportSettings` overrides; the case directory name is the id.
fn case_line(dir: &Path) -> Result<String, String> {
    let case = dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| format!("{}: bad case directory name", dir.display()))?;
    let expected_path = dir.join("expected.json");
    let expected: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&expected_path)
            .map_err(|e| format!("read {}: {e}", expected_path.display()))?,
    )
    .map_err(|e| format!("parse {}: {e}", expected_path.display()))?;
    let file = expected["file"]
        .as_str()
        .ok_or_else(|| format!("{}: expected.json has no \"file\"", dir.display()))?;
    let settings: ImportSettings = serde_json::from_value(expected["settings"].clone())
        .map_err(|e| format!("{}: expected.json \"settings\": {e}", dir.display()))?;
    let input_path = dir.join(file);
    let bytes =
        std::fs::read(&input_path).map_err(|e| format!("read {}: {e}", input_path.display()))?;
    Ok(parity_line(
        case,
        &epiora_import::import(&bytes, file, &settings),
    ))
}
