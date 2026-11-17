//! `run_all` — the corpus walk shared by `corpus-runner`, the
//! `--hash-golden` generator and the in-process corpus tests
//! (v1.0 Task 10). One entry per fixture, in corpus-runner order:
//! the FAMILIES order, each family's `*.json` files sorted by path.
//! Families with no directory under the root are skipped (the
//! `calc/random_numbers` family is golden-only, spec §5.2).

use super::{load_dataset_dir, run_case, CaseOutput};
use crate::dataset::Dataset;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The corpus families, in corpus-runner order.
pub const FAMILIES: &[&str] = &[
    "special",
    "2x2",
    "stratified",
    "rxc",
    "freq",
    "means",
    "calc/proportion",
    "calc/two_by_two",
    "calc/two_by_n",
    "calc/r_by_c",
    "calc/dose_response",
    "calc/matched_case_control",
    "calc/screening",
    "calc/smr",
    "calc/binomial",
    "calc/poisson",
    "calc/rate_one",
    "calc/rate_compare",
    "calc/mean_ci",
    "calc/median_ci",
    "calc/t_test_summary",
    "calc/anova_summary",
    "calc/ss_proportion",
    "calc/ss_unmatched_cc",
    "calc/ss_cohort",
    "calc/ss_mean_difference",
    "calc/power_unmatched_cc",
    "calc/power_cohort",
    "calc/power_mean_difference",
    "calc/random_numbers",
];

/// One corpus case: where its fixture lives and what it produced.
pub struct CorpusLine {
    /// The fixture path relative to the corpus root —
    /// `2x2/oswego_chocolate.json`, `calc/proportion/textbook.json`.
    pub rel_path: String,
    /// The parsed fixture document.
    pub fixture: Value,
    /// What `run_case` produced.
    pub output: CaseOutput,
}

/// The corpus datasets of `root` (`root/data/*.dataset.json`).
pub fn load_datasets(root: &Path) -> Result<BTreeMap<String, Dataset>, String> {
    load_dataset_dir(&root.join("data")).map_err(|e| format!("load datasets: {e}"))
}

/// Runs every fixture under `root` in corpus-runner order.
pub fn corpus_lines(root: &Path) -> Result<Vec<CorpusLine>, String> {
    let datasets = load_datasets(root)?;
    corpus_lines_with(root, &datasets)
}

/// `corpus_lines` over an already-loaded dataset map.
pub fn corpus_lines_with(
    root: &Path,
    datasets: &BTreeMap<String, Dataset>,
) -> Result<Vec<CorpusLine>, String> {
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
            let fixture: Value = serde_json::from_str(&text)
                .map_err(|e| format!("parse {}: {e}", path.display()))?;
            let output =
                run_case(&fixture, datasets).map_err(|e| format!("{}: {e}", path.display()))?;
            let rel_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            lines.push(CorpusLine {
                rel_path,
                fixture,
                output,
            });
        }
    }
    Ok(lines)
}

/// Maps every `inputs.dataset` id a fixture may name to the
/// `data/*.dataset.json` file it loads from, keyed by the dataset id
/// inside the file — the second hash a parity line can depend on.
pub fn dataset_files(root: &Path) -> Result<BTreeMap<String, PathBuf>, String> {
    let dir = root.join("data");
    let mut out = BTreeMap::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(&dir).map_err(|e| format!("read {}: {e}", dir.display()))? {
        let path = entry
            .map_err(|e| format!("read {}: {e}", dir.display()))?
            .path();
        if !path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(".dataset.json"))
        {
            continue;
        }
        let text =
            std::fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
        let doc: Value =
            serde_json::from_str(&text).map_err(|e| format!("parse {}: {e}", path.display()))?;
        if let Some(id) = doc["id"].as_str() {
            out.insert(id.to_string(), path);
        }
    }
    Ok(out)
}
