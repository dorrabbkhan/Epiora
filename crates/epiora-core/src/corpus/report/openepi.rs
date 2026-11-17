//! The OpenEpi leg of the validation report (v1.0 spec §9, Task 12):
//! which registry statistics `openepi/*.json` map labels onto (for
//! the Reference column) and the `## OpenEpi check` section — one row
//! per captured calculator with its mapped-value and mismatch counts,
//! measured by `corpus::openepi::compare_openepi`.

use crate::corpus::openepi::{capture_files, compare_openepi, normalize_key};
use crate::EngineError;
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

fn corpus(reason: impl Into<String>) -> EngineError {
    EngineError::CorpusFormat {
        reason: reason.into(),
    }
}

/// The registry statistic ids the `openepi/` capture files map labels
/// onto — empty when the directory is absent, matching the ei7 leg's
/// absent-file behaviour.
pub(super) fn covered_statistics(fixtures_dir: &Path) -> BTreeSet<String> {
    let mut covered = BTreeSet::new();
    let dir = fixtures_dir.join("openepi");
    let Ok(files) = capture_files(&dir) else {
        return covered;
    };
    for path in files {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(doc) = serde_json::from_str::<Value>(&text) else {
            continue;
        };
        let Some(cases) = doc.get("cases").and_then(Value::as_array) else {
            continue;
        };
        for case in cases {
            if let Some(values) = case.get("values").and_then(Value::as_array) {
                for entry in values {
                    if let Some(stat) = entry.get("statistic").and_then(Value::as_str) {
                        covered.insert(normalize_key(stat));
                    }
                }
            }
        }
    }
    covered
}

/// The `## OpenEpi check` Markdown section: one row per calculator,
/// the OpenEpi column counting mapped values and mismatches.
pub(super) fn section(fixtures_dir: &Path) -> Result<String, EngineError> {
    let dir = fixtures_dir.join("openepi");
    if !dir.is_dir() {
        return Err(corpus(format!("{}: no such directory", dir.display())));
    }
    let report = compare_openepi(&dir)?;
    let mut out = String::from("\n## OpenEpi check\n\n");
    out.push_str(&format!(
        "`openepi/*.json`: {} displayed values mapped to engine statistics across {} \
         calculators, {} mismatch(es) at `openepi_display` (half a unit in the last \
         displayed decimal, spec §9), {} documented method difference(s) (the output's \
         `method` names the tool showing the differing method — `docs/decisions.md` \
         `### v1.0 Task 13 — method choices`).",
        report.checked,
        report.calculators.len(),
        report.mismatches.len(),
        report.method_differences.len()
    ));
    if report.unmapped.is_empty() {
        out.push_str(" Every printed label maps to an engine statistic.\n");
    } else {
        out.push_str(&format!(
            " {} printed labels have no engine statistic (enumerated in \
             `openepi/labels.json`).\n",
            report.unmapped.len()
        ));
    }
    out.push_str("\n| Calculator | OpenEpi |\n|---|---|\n");
    for c in &report.calculators {
        out.push_str(&format!(
            "| {} | {} mapped, {} mismatches, {} documented differences |\n",
            c.id, c.checked, c.mismatches, c.differences
        ));
    }
    for m in &report.mismatches {
        out.push_str(&format!("\n- {m}"));
    }
    if !report.mismatches.is_empty() {
        out.push('\n');
    }
    if !report.method_differences.is_empty() {
        out.push_str("\nDocumented method differences:\n");
        for m in &report.method_differences {
            out.push_str(&format!("\n- {m}"));
        }
        out.push('\n');
    }
    Ok(out)
}
