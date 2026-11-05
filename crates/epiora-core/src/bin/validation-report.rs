//! `validation-report <fixtures-dir> <run-url>` — prints the v0.2
//! validation report (platform §12.10) to stdout: one row per
//! `validation::REGISTRY` statistic measured by running the corpus,
//! plus headers naming the commit, the run URL, the wasm `gzip -9`
//! size and the Chromium bench median. The last two arrive from the
//! `engine` and `bench` jobs as `EPIORA_WASM_GZIP_BYTES` /
//! `EPIORA_BENCH_MEDIAN_MS`; when unset they render "not measured".

use epiora_core::corpus::report::build_report;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("validation-report: {e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let usage = "usage: validation-report <fixtures-dir> <run-url>";
    let root = PathBuf::from(args.next().ok_or(usage)?);
    let run_url = args.next().ok_or(usage)?;
    let size =
        std::env::var("EPIORA_WASM_GZIP_BYTES").unwrap_or_else(|_| "not measured".to_string());
    let median =
        std::env::var("EPIORA_BENCH_MEDIAN_MS").unwrap_or_else(|_| "not measured".to_string());
    let report = build_report(&root, &run_url, &size, &median).map_err(|e| e.to_string())?;
    print!("{report}");
    Ok(())
}
