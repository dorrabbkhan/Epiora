//! The parity case runner (`parity` feature): one canonical input —
//! `{name, tables: [{name, batches: [hex…]}]}` — through `legacyPush`
//! → `legacyDetect` → `legacyMap` produces one canonical JSON output —
//! `{detected, project, snapshots: [hex…]}`. `native_and_wasm_bit_identical`
//! drives it over every golden project and compares its bytes against
//! the direct `epiora-legacy` pipeline and the golden files; the wasm
//! module exports the same runner as `legacyCase` so the binding adds
//! no behaviour of its own.
//!
//! The test also writes every case's `{input, output}` to
//! `target/parity/epiora-legacy-wasm-cases.jsonl`; `parity/run.mjs`
//! feeds the same inputs to the real wasm32 `legacyCase` (the nodejs
//! `pkg-parity` build) and the workflow diffs the two files
//! byte-for-byte — a wasm/native divergence fails CI, not just the
//! test.

#[cfg(test)]
mod cases;
#[cfg(test)]
mod tests;

use crate::{detect_and_map, project_json, project_snapshots};
use epiora_legacy::LegacyError;
use wasm_bindgen::prelude::*;

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, LegacyError> {
    let bad = || LegacyError::BatchInvalid {
        reason: format!("bad hex {s:?}"),
    };
    if !s.len().is_multiple_of(2) {
        return Err(bad());
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| bad()))
        .collect()
}

/// The canonical run shared by the wasm export and the test.
pub fn run_case(input: &str) -> Result<String, LegacyError> {
    let v: serde_json::Value = serde_json::from_str(input).map_err(|e| LegacyError::Source {
        code: "legacy_batch_invalid",
        reason: format!("case input JSON failed: {e}"),
    })?;
    let name = v["name"].as_str().unwrap_or("").to_string();
    let mut handle = crate::legacy_new();
    let err = |r: &str| LegacyError::BatchInvalid {
        reason: r.to_string(),
    };
    for t in v["tables"]
        .as_array()
        .ok_or_else(|| err("tables[] missing"))?
    {
        let tname = t["name"]
            .as_str()
            .ok_or_else(|| err("table name missing"))?;
        for b in t["batches"]
            .as_array()
            .ok_or_else(|| err("batches[] missing"))?
        {
            let bytes = hex_decode(b.as_str().ok_or_else(|| err("batch not a hex string"))?)?;
            handle.source.push(tname, &bytes)?;
        }
    }
    let fmt = crate::parse_format(v["format"].as_str().unwrap_or(""));
    let (format, project) = detect_and_map(&mut handle.source, fmt, &name)?;
    let mut out = String::from("{\"detected\":");
    out.push_str(&serde_json::json!(crate::format_str(format)).to_string());
    out.push_str(",\"project\":");
    out.push_str(&project_json(&project)?);
    out.push_str(",\"snapshots\":[");
    for (i, s) in project_snapshots(&project)?.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push('"');
        out.push_str(&hex_encode(s));
        out.push('"');
    }
    out.push_str("]}");
    Ok(out)
}

/// `legacyCase(input) -> canonical JSON` — the nodejs parity build's
/// entry point.
#[wasm_bindgen(js_name = legacyCase)]
pub fn legacy_case(input: String) -> Result<String, JsValue> {
    run_case(&input).map_err(|e| crate::js_error(&e))
}
