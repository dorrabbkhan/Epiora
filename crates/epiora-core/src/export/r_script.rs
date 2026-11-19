//! `r_script` — `export_r_procedure`/`export_r_calculator` (v1.0
//! Task 17, spec §8.1). A script is the provenance header, the
//! `epi_walk` prelude, a reader prelude when it has a `data.csv`, the
//! verbatim corpus R the driver needs, and one driver call. `epi_walk`
//! prints `EPIORA <statistic path> <value %.17g>` per leaf — the same
//! paths `corpus::compare` walks. The drivers live in `r_calc`
//! (calculator writer dispatch) and `r_proc` (dataset drivers).

use super::csv::{dataset_csv, missing_markers};
use super::r_calc::calc_call;
use super::r_emit::{header, needs_epitools, reader, ATTACH_EPITOOLS, PRELUDE};
use super::r_proc::{freq_driver, means_driver, tables_driver, var_spec};
use super::r_snippets::snippet_set;
use crate::dataset::Dataset;
use crate::EngineError;
use serde_json::Value;

/// One generated script plus its `data.csv` when the analysis reads a
/// dataset (procedures always; calculators never).
#[derive(Debug)]
pub struct RExport {
    /// `analysis.R`, UTF-8 text.
    pub script: String,
    /// `data.csv` bytes (UTF-8) when the script reads one.
    pub data_csv: Option<Vec<u8>>,
}

pub(crate) fn invalid(reason: impl Into<String>) -> EngineError {
    EngineError::InvalidOptions {
        reason: reason.into(),
    }
}

pub(crate) fn not_exportable(reason: impl Into<String>) -> EngineError {
    EngineError::ExportNotAvailable {
        reason: reason.into(),
    }
}

/// `"key"` from `inputs`, or `InvalidOptions` when absent.
pub(crate) fn req<'a>(inputs: &'a Value, key: &str) -> Result<&'a Value, EngineError> {
    inputs
        .get(key)
        .ok_or_else(|| invalid(format!("inputs.{key} is missing")))
}

pub(crate) fn opt<'a>(inputs: &'a Value, key: &str) -> Option<&'a Value> {
    inputs.get(key).filter(|v| !v.is_null())
}

/// `list(name = …)` built from the sorted union of `packages` plus
/// the `pkg::fn` references the verbatim R makes (scanned from the
/// emitted text so the header can never drift from the script).
fn header_for(title: &str, receipt_sha256: &str, script_body: &str, extra: &[&str]) -> String {
    let mut pkgs: Vec<String> = vec!["stats".into()];
    for p in extra {
        pkgs.push((*p).to_string());
    }
    let bytes = script_body.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b':' && bytes[i + 1] == b':' {
            let mut start = i;
            while start > 0
                && (bytes[start - 1].is_ascii_alphanumeric()
                    || bytes[start - 1] == b'.'
                    || bytes[start - 1] == b'_')
            {
                start -= 1;
            }
            let name = &script_body[start..i];
            if !name.is_empty() {
                pkgs.push(name.to_string());
            }
            i += 1;
        }
        i += 1;
    }
    pkgs.sort();
    pkgs.dedup();
    let refs: Vec<&str> = pkgs.iter().map(String::as_str).collect();
    header(title, &refs, receipt_sha256)
}

/// Header + prelude + verbatim snippets + body = `analysis.R`.
fn assemble(
    title: &str,
    receipt_sha256: &str,
    seeds: &[&str],
    body: &str,
) -> Result<String, EngineError> {
    let snippets = snippet_set(seeds)?;
    let mut script = String::new();
    script.push_str(PRELUDE);
    script.push('\n');
    script.push_str(&snippets);
    script.push_str("\n\n");
    script.push_str(body);
    let extra: &[&str] = if needs_epitools(&script) {
        script.insert_str(PRELUDE.len(), ATTACH_EPITOOLS);
        &["epitools"]
    } else {
        &[]
    };
    let head = header_for(title, receipt_sha256, &script, extra);
    Ok(format!("{head}{script}"))
}

/// The R script for one calculator run (spec §8.2): no `data.csv` —
/// the inputs are embedded as literals.
pub fn export_r_calculator(
    id: &str,
    inputs: &Value,
    receipt_sha256: &str,
) -> Result<RExport, EngineError> {
    let (seed, call) = calc_call("run", id, inputs)?;
    let body = format!("{call}\n");
    let script = assemble(&format!("calc.{id}"), receipt_sha256, &[seed], &body)?;
    Ok(RExport {
        script,
        data_csv: None,
    })
}

/// The app-facing procedure export (spec §8.2): `procedure` is
/// `tables`/`freq`/`means`, `options` the JSON options the run took.
pub fn export_r_procedure(
    ds: &Dataset,
    procedure: &str,
    options: &Value,
    receipt_sha256: &str,
) -> Result<RExport, EngineError> {
    let markers = missing_markers(ds);
    let (seeds, body) = match procedure {
        "tables" => {
            let stratify_by: Vec<String> = options["stratify_by"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            tables_driver(
                "run",
                "dataset",
                ds,
                &markers,
                &var_spec(&options["exposure"])?,
                &var_spec(&options["outcome"])?,
                &stratify_by,
                options["include_missing"].as_bool().unwrap_or(false),
            )?
        }
        "freq" => {
            let variables: Vec<String> = options["variables"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default();
            freq_driver(
                "run",
                "dataset",
                ds,
                &markers,
                &variables,
                options["stratify_by"].as_str(),
                options["include_missing"].as_bool().unwrap_or(false),
            )?
        }
        "means" => means_driver(
            "run",
            "dataset",
            ds,
            &markers,
            options["variable"]
                .as_str()
                .ok_or_else(|| invalid("options.variable"))?,
            options["group_by"].as_str(),
            options["include_missing"].as_bool().unwrap_or(false),
        )?,
        other => return Err(not_exportable(format!("procedure {other:?}"))),
    };
    let body = reader(ds.row_count()) + &body;
    let script = assemble(
        &format!("procedure {procedure}"),
        receipt_sha256,
        &seeds,
        &body,
    )?;
    Ok(RExport {
        script,
        data_csv: Some(dataset_csv(ds)),
    })
}
