//! `validation-report <fixtures-dir> <run-url> [--format <markdown|json>]` —
//! prints the validation report (platform §12.10) to stdout: one row per
//! `validation::REGISTRY` statistic measured by running the corpus,
//! plus headers naming the commit, the run URL, the wasm `gzip -9`
//! size and the Chromium bench median, then the `## Epi Info 7 check`
//! and `## OpenEpi check` sections (v1.0 spec §9). The last two
//! arrive from the
//! `engine` and `bench` jobs as `EPIORA_WASM_GZIP_BYTES` /
//! `EPIORA_BENCH_MEDIAN_MS`; when unset they render "not measured".
//! Markdown is the default and is byte-identical to before v1.0 Task
//! 24; `--format json` (spec §7.4) prints the machine-readable report
//! the `/validation` page reads — engine.yml uploads it and the app
//! build fetches it as `public/validation.json`.

use epiora_core::corpus::report::{build_report, build_report_json};
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
    let usage = "usage: validation-report <fixtures-dir> <run-url> [--format markdown|json]";
    let root = PathBuf::from(args.next().ok_or(usage)?);
    let run_url = args.next().ok_or(usage)?;
    let format = match args.next() {
        None => "markdown".to_string(),
        Some(flag) if flag == "--format" => args.next().ok_or(usage)?,
        Some(flag) if flag.starts_with("--format=") => flag["--format=".len()..].to_string(),
        Some(_) => return Err(usage.to_string()),
    };
    match format.as_str() {
        "json" => {
            let report = build_report_json(&root, &run_url).map_err(|e| e.to_string())?;
            print!("{report}");
            Ok(())
        }
        "markdown" | "md" => {
            let or_unmeasured = |v: String| {
                if v.is_empty() {
                    "not measured".to_string()
                } else {
                    v
                }
            };
            let size = or_unmeasured(
                std::env::var("EPIORA_WASM_GZIP_BYTES")
                    .unwrap_or_else(|_| "not measured".to_string()),
            );
            let median = or_unmeasured(
                std::env::var("EPIORA_BENCH_MEDIAN_MS")
                    .unwrap_or_else(|_| "not measured".to_string()),
            );
            let report =
                build_report(&root, &run_url, &size, &median).map_err(|e| e.to_string())?;
            print!("{report}");
            Ok(())
        }
        _ => Err(format!(
            "unknown --format {format:?} (expected markdown or json)"
        )),
    }
}
