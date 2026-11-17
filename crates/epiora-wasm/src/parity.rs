//! `runCorpusCase` — the wasm side of the native/wasm parity diff
//! (spec §5.2). One fixture JSON document and the corpus datasets go
//! in; the same lines `corpus-runner` prints come out: the case's
//! `parity_line`, then — for dataset cases — its Epi Info command
//! line (v1.0 Task 11), joined by `\n`. `explainCorpusPlan` and
//! `runCorpusExplainLine` produce the trailing explain section: one
//! explain trace per `validation::REGISTRY` statistic, in registry
//! order. The fixture is parsed with serde_json's `float_roundtrip`,
//! matching the native parser bit for bit.

use epiora_core::corpus::{command_line, explain_line, explain_plan, load_dataset, run_case};
use epiora_core::dataset::Dataset;
use epiora_core::EngineError;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

fn corpus(reason: impl Into<String>) -> JsValue {
    crate::js_error(&EngineError::CorpusFormat {
        reason: reason.into(),
    })
}

fn datasets_of(datasets_json: &str) -> Result<BTreeMap<String, Dataset>, JsValue> {
    let raw: BTreeMap<String, Value> =
        serde_json::from_str(datasets_json).map_err(|e| corpus(format!("datasets JSON: {e}")))?;
    let mut datasets = BTreeMap::new();
    for (id, doc) in &raw {
        // Round-tripping through serde_json is lossless for the dataset
        // format (float_roundtrip parses the emitted digits identically).
        let text = serde_json::to_string(doc).map_err(|e| corpus(format!("dataset {id}: {e}")))?;
        let dataset: Dataset =
            load_dataset(&text).map_err(|e| corpus(format!("dataset {id}: {e}")))?;
        datasets.insert(id.clone(), dataset);
    }
    Ok(datasets)
}

fn fixture_of(fixture_json: &str) -> Result<Value, JsValue> {
    serde_json::from_str(fixture_json).map_err(|e| corpus(format!("fixture JSON: {e}")))
}

/// `datasets_json` is a JSON object mapping dataset id to the
/// `*.dataset.json` document — the same files `corpus-runner` loads
/// from `fixtures/data/`. Returns one or more `\n`-separated parity
/// lines (the case line plus the Epi Info command line of dataset
/// cases, v1.0 Task 11).
#[wasm_bindgen(js_name = runCorpusCase)]
pub fn run_corpus_case(fixture_json: &str, datasets_json: &str) -> Result<String, JsValue> {
    let fixture = fixture_of(fixture_json)?;
    let datasets = datasets_of(datasets_json)?;
    let mut out = run_case(&fixture, &datasets)
        .map(|output| output.parity_line())
        .map_err(|e| crate::js_error(&e))?;
    if let Some(command) = command_line(&fixture).map_err(|e| crate::js_error(&e))? {
        out.push('\n');
        out.push_str(&command);
    }
    Ok(out)
}

/// The `(registry statistic, covering fixture key)` pairs of the
/// parity explain section (v1.0 Task 11) — `corpus::explain_plan`
/// serialized; `run.mjs` reads each `key + ".json"` and passes it to
/// `runCorpusExplainLine`.
#[wasm_bindgen(js_name = explainCorpusPlan)]
pub fn explain_corpus_plan() -> Result<JsValue, JsValue> {
    explain_plan()
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(|e| corpus(format!("explain plan does not serialize: {e}")))
}

/// One explain parity line — the same bytes the trailing section of
/// `corpus-runner` prints for `statistic` (v1.0 Task 11).
#[wasm_bindgen(js_name = runCorpusExplainLine)]
pub fn run_corpus_explain_line(
    fixture_json: &str,
    datasets_json: &str,
    statistic: &str,
) -> Result<String, JsValue> {
    let fixture = fixture_of(fixture_json)?;
    let datasets = datasets_of(datasets_json)?;
    explain_line(&fixture, &datasets, statistic).map_err(|e| crate::js_error(&e))
}
