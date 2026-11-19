//! `build_report_json` — the JSON form of the validation report
//! (v1.0 plan Task 24, spec §7.4). The same `build_rows` measurements
//! the Markdown report prints, as one object per `validation::REGISTRY`
//! statistic — the public `/validation` page reads this file and shows
//! no number that is not in it. `engine.yml`'s `validation-report` job
//! uploads it; the app build fetches it as `public/validation.json`.

use super::{build_rows, Row};
use crate::EngineError;
use serde_json::{json, Value};
use std::path::Path;

/// One `statistics[]` entry: `id`, `method`, `class`, `references`
/// (`r` always — every registry fixture is R-generated; `ei7` /
/// `openepi` only when the check maps a label to the statistic),
/// `cases` (the Markdown report's `Fixtures` column) and
/// `worst_error` — absent when the statistic was not measured,
/// `null` when a leaf could not be compared numerically (the
/// Markdown report's `inf`).
fn entry(row: &Row) -> Value {
    let mut references = json!({ "r": row.r_reference });
    if row.ei7 {
        references["ei7"] = json!("Epi Info 7");
    }
    if row.openepi {
        references["openepi"] = json!("OpenEpi");
    }
    let mut out = json!({
        "id": row.statistic,
        "method": row.method,
        "class": row.class,
        "references": references,
        "cases": row.fixture_count,
    });
    if row.worst_error.is_finite() {
        out["worst_error"] = json!(row.worst_error);
    } else {
        out["worst_error"] = Value::Null;
    }
    out
}

/// The JSON validation report: `engine_version` (the receipt stamp's
/// `CARGO_PKG_VERSION`), `commit` (the build-time `EPIORA_GIT_SHA`),
/// `run_url` and one entry per registry statistic, in registry order.
pub fn build_report_json(fixtures_dir: &Path, run_url: &str) -> Result<String, EngineError> {
    let rows = build_rows(fixtures_dir)?;
    let doc = json!({
        "engine_version": env!("CARGO_PKG_VERSION"),
        "commit": env!("EPIORA_GIT_SHA"),
        "run_url": run_url,
        "statistics": rows.iter().map(entry).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&doc)
        .map_err(|e| EngineError::CorpusFormat {
            reason: format!("validation json does not serialize: {e}"),
        })
        .map(|mut s| {
            s.push('\n');
            s
        })
}
