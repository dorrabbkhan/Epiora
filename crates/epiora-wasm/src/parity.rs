//! `runCorpusCase` — the wasm side of the native/wasm parity diff
//! (spec §5.2). One fixture JSON document and the corpus datasets go
//! in; the same `parity_line` `corpus-runner` prints comes out. The
//! fixture is parsed with serde_json's `float_roundtrip`, matching the
//! native parser bit for bit.

use epiora_core::corpus::{load_dataset, run_case};
use epiora_core::dataset::Dataset;
use epiora_core::EngineError;
use serde_json::Value;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

fn corpus(reason: impl Into<String>) -> JsValue {
    crate::js_error(&EngineError::CorpusFormat {
        reason: reason.into(),
    })
}

/// `datasets_json` is a JSON object mapping dataset id to the
/// `*.dataset.json` document — the same files `corpus-runner` loads
/// from `fixtures/data/`.
#[wasm_bindgen(js_name = runCorpusCase)]
pub fn run_corpus_case(fixture_json: &str, datasets_json: &str) -> Result<String, JsValue> {
    let fixture: Value =
        serde_json::from_str(fixture_json).map_err(|e| corpus(format!("fixture JSON: {e}")))?;
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
    run_case(&fixture, &datasets)
        .map(|output| output.parity_line())
        .map_err(|e| crate::js_error(&e))
}
