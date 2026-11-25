//! epiora-lang-wasm: the browser binding for `epiora-lang` (v1.1 spec
//! §5–6, plan Task 13). Type conversion only — .pgm text in, parsed
//! program / action plan / replayed view snapshot + recipe and view
//! hashes out. No language logic.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

#[cfg(feature = "parity")]
mod parity;

use epiora_core::dataset::DatasetLink;
use epiora_core::snapshot::{decode_snapshot, encode_snapshot};
use epiora_lang::{
    parse_program, plan, print_command, register_json, replay, Command, LangError, Recipe,
    StoredLinks, WorkingView,
};
use js_sys::{Error, Reflect};
use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

/// A `LangError` crossing the boundary becomes a JS `Error` whose
/// `code` is the crate's stable code and whose `message` is `Display`
/// — the `LangErrorJson` the worker maps onto its own error contract.
fn js_error(e: &LangError) -> JsValue {
    let err = Error::new(&e.to_string());
    let _ = Reflect::set(
        &err,
        &JsValue::from_str("code"),
        &JsValue::from_str(e.code()),
    );
    err.into()
}

fn to_json<T: serde::Serialize>(v: &T) -> Result<JsValue, JsValue> {
    let text = serde_json::to_string(v).map_err(|e| {
        js_error(&LangError::RecipeReplayMismatch {
            message: format!("JSON serialization failed: {e}"),
        })
    })?;
    js_sys::JSON::parse(&text).map_err(|_| {
        js_error(&LangError::RecipeReplayMismatch {
            message: "JSON parse failed".to_string(),
        })
    })
}

/// `parseProgram(text) -> ProgramJson | LangErrorJson` — the parsed
/// program (spanned statements), or a thrown `Error` carrying the
/// stable `code`.
#[wasm_bindgen(js_name = parseProgram)]
pub fn parse_program_js(text: &str) -> Result<JsValue, JsValue> {
    let program = parse_program(text).map_err(|e| js_error(&e))?;
    to_json(&program)
}

/// `planProgram(text) -> ActionJson[]` — parse + `plan` against the
/// embedded register (a `RUNPGM` without a project source becomes an
/// `Unsupported` action, matching `plan`'s no-programs semantics).
#[wasm_bindgen(js_name = planProgram)]
pub fn plan_program_js(text: &str) -> Result<JsValue, JsValue> {
    let program = parse_program(text).map_err(|e| js_error(&e))?;
    to_json(&plan(&program, epiora_lang::register()))
}

/// `printCommand(json) -> string` — the canonical spelling of one
/// `Command` JSON (what `applyCommand` appends to a recipe).
#[wasm_bindgen(js_name = printCommand)]
pub fn print_command_js(json: JsValue) -> Result<String, JsValue> {
    let cmd: Command = serde_wasm_bindgen::from_value(json).map_err(|e| {
        js_error(&LangError::RecipeReplayMismatch {
            message: format!("command JSON failed: {e}"),
        })
    })?;
    Ok(print_command(&cmd))
}

/// `register() -> RegisterJson` — the embedded register verbatim.
#[wasm_bindgen(js_name = register)]
pub fn register_js() -> Result<JsValue, JsValue> {
    js_sys::JSON::parse(register_json()).map_err(|_| {
        js_error(&LangError::RecipeReplayMismatch {
            message: "register JSON parse failed".to_string(),
        })
    })
}

/// `source`/`linked` snapshots decode by caller-chosen name — the
/// dataset's identity lives in the worker's store, not the snapshot
/// bytes. `links` are the project's stored `dataset_links` rows
/// (`{parent, child, child_key, parent_key, kind}` — the serde shape
/// of `DatasetLink`) so a keyless `RELATE` resolves its key pair.
pub(crate) fn replay_result(
    source: &[u8],
    linked: &BTreeMap<String, Vec<u8>>,
    recipe_text: &str,
    links: &[DatasetLink],
) -> Result<(WorkingView, Recipe), LangError> {
    let src =
        decode_snapshot("source", 0, source).map_err(|e| LangError::RecipeReplayMismatch {
            message: format!("source snapshot: {e}"),
        })?;
    let mut tables = BTreeMap::new();
    for (name, bytes) in linked {
        let ds = decode_snapshot(name, 0, bytes).map_err(|e| LangError::RecipeReplayMismatch {
            message: format!("linked snapshot {name:?}: {e}"),
        })?;
        tables.insert(name.to_ascii_lowercase(), ds);
    }
    let recipe = Recipe::parse(recipe_text)?;
    let view = replay(&src, &StoredLinks::new(tables, links.to_vec()), &recipe)?;
    Ok((view, recipe))
}

/// `replay(source, linked, recipeText, links?) -> { view: Uint8Array,
/// viewSha256: string, recipeSha256: string }` — the materialized
/// view snapshot plus the two hashes the worker writes into a
/// receipt's `working_view` stamp. `links` is the project's stored
/// `dataset_links` as `DatasetLinkJson[]`; omitted behaves as before.
#[wasm_bindgen(js_name = replay)]
pub fn replay_js(
    source: &[u8],
    linked: JsValue,
    recipe_text: &str,
    links: JsValue,
) -> Result<JsValue, JsValue> {
    let tables: BTreeMap<String, Vec<u8>> =
        serde_wasm_bindgen::from_value(linked).map_err(|e| {
            js_error(&LangError::RecipeReplayMismatch {
                message: format!("linked map failed: {e}"),
            })
        })?;
    let links: Vec<DatasetLink> = if links.is_undefined() || links.is_null() {
        Vec::new()
    } else {
        serde_wasm_bindgen::from_value(links).map_err(|e| {
            js_error(&LangError::RecipeReplayMismatch {
                message: format!("links JSON failed: {e}"),
            })
        })?
    };
    let (view, recipe) =
        replay_result(source, &tables, recipe_text, &links).map_err(|e| js_error(&e))?;
    let materialized = view.materialize();
    let snapshot = encode_snapshot(&materialized);
    let out = js_sys::Object::new();
    let _ = Reflect::set(
        &out,
        &JsValue::from_str("view"),
        &js_sys::Uint8Array::from(&snapshot[..]),
    );
    let _ = Reflect::set(
        &out,
        &JsValue::from_str("viewSha256"),
        &JsValue::from_str(&materialized.content_hash_hex()),
    );
    let _ = Reflect::set(
        &out,
        &JsValue::from_str("recipeSha256"),
        &JsValue::from_str(&recipe.sha256_hex()),
    );
    Ok(out.into())
}
