//! The parity case runner (`parity` feature): one canonical input —
//! `{source, linked, recipe_text, program_text}` — through the same
//! calls the wasm exports make produces one canonical JSON output —
//! `{program, actions, print, stamp, view, view_sha256,
//! recipe_sha256, register_sha256}`. `native_and_wasm_bit_identical`
//! drives it over every Task-3 `.pgm` fixture and compares its bytes
//! against the direct `epiora-lang` pipeline; the wasm module exports
//! the same runner as `langCase`.
//!
//! The test also writes every case's `{input, output}` to
//! `target/parity/epiora-lang-wasm-cases.jsonl`; `parity/run.mjs`
//! feeds the same inputs to the real wasm32 `langCase` (the nodejs
//! `pkg-parity` build) and the workflow diffs the two files
//! byte-for-byte — a wasm/native divergence fails CI, not just the
//! test.

#[cfg(test)]
mod cases;
#[cfg(test)]
mod tests;

use crate::replay_result;
use epiora_lang::{parse_program, plan, print_command, register_json, LangError};
use sha2::Digest;
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

fn hex_encode(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_decode(s: &str) -> Result<Vec<u8>, LangError> {
    let bad = || LangError::RecipeReplayMismatch {
        message: format!("bad hex {s:?}"),
    };
    if !s.len().is_multiple_of(2) {
        return Err(bad());
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).map_err(|_| bad()))
        .collect()
}

fn err(message: String) -> LangError {
    LangError::RecipeReplayMismatch { message }
}

/// The canonical run shared by the wasm export and the test.
pub fn run_case(input: &str) -> Result<String, LangError> {
    let v: serde_json::Value =
        serde_json::from_str(input).map_err(|e| err(format!("case input JSON failed: {e}")))?;
    let program_text = v["program_text"].as_str().unwrap_or("").to_string();
    let recipe_text = v["recipe_text"].as_str().unwrap_or("").to_string();
    let source = hex_decode(v["source"].as_str().unwrap_or(""))?;
    let mut linked = BTreeMap::new();
    if let Some(m) = v["linked"].as_object() {
        for (k, b) in m {
            linked.insert(
                k.clone(),
                hex_decode(b.as_str().ok_or_else(|| err("linked not hex".into()))?)?,
            );
        }
    }

    let program = parse_program(&program_text)?;
    let actions = plan(&program, epiora_lang::register());
    let (view, recipe) = replay_result(&source, &linked, &recipe_text)?;
    let materialized = view.materialize();
    let view_bytes = epiora_core::snapshot::encode_snapshot(&materialized);
    let src_ds = epiora_core::snapshot::decode_snapshot("source", 0, &source)
        .map_err(|e| err(format!("source snapshot: {e}")))?;
    let stamp = epiora_lang::working_view_stamp(&src_ds, &materialized, &recipe);
    let printed: Vec<String> = recipe.steps.iter().map(print_command).collect();
    let register_digest = sha2::Sha256::digest(register_json().as_bytes());
    let register_sha256 = register_digest
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();

    let mut out = String::from("{\"program\":");
    out.push_str(&serde_json::to_string(&program).map_err(|e| err(e.to_string()))?);
    out.push_str(",\"actions\":");
    out.push_str(&serde_json::to_string(&actions).map_err(|e| err(e.to_string()))?);
    out.push_str(",\"print\":");
    out.push_str(&serde_json::to_string(&printed).map_err(|e| err(e.to_string()))?);
    out.push_str(",\"stamp\":");
    out.push_str(&serde_json::to_string(&stamp).map_err(|e| err(e.to_string()))?);
    out.push_str(",\"view\":\"");
    out.push_str(&hex_encode(&view_bytes));
    out.push_str("\",\"view_sha256\":\"");
    out.push_str(&materialized.content_hash_hex());
    out.push_str("\",\"recipe_sha256\":\"");
    out.push_str(&recipe.sha256_hex());
    out.push_str("\",\"register_sha256\":\"");
    out.push_str(&register_sha256);
    out.push_str("\"}");
    Ok(out)
}

/// `langCase(input) -> canonical JSON` — the nodejs parity build's
/// entry point.
#[wasm_bindgen(js_name = langCase)]
pub fn lang_case(input: String) -> Result<String, JsValue> {
    run_case(&input).map_err(|e| crate::js_error(&e))
}
