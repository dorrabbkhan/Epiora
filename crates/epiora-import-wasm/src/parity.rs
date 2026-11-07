//! `runImportCase` — the wasm side of the native/wasm parity diff
//! (v0.3 spec §6.5). One fixture case goes in (input bytes, file name,
//! `expected.json`'s `settings` JSON, case id); the same
//! [`epiora_import::parity_line`] `import-runner` prints comes out.
//! The settings are parsed with serde_json's `float_roundtrip`,
//! matching the native parser bit for bit.

use epiora_import::{ImportError, ImportSettings};
use wasm_bindgen::prelude::*;

/// `runImportCase(bytes, fileName, settingsJson, caseId)` — never
/// throws: the import outcome (Ok or its `ImportError`) is folded into
/// the parity line; only a malformed `settings` JSON throws, as
/// `invalid_settings`.
#[wasm_bindgen(js_name = runImportCase)]
pub fn run_import_case(
    bytes: &[u8],
    file_name: &str,
    settings_json: &str,
    case_id: &str,
) -> Result<String, JsValue> {
    let settings: ImportSettings = serde_json::from_str(settings_json).map_err(|e| {
        crate::js_error(&ImportError::InvalidSettings {
            reason: format!("settings JSON: {e}"),
        })
    })?;
    Ok(epiora_import::parity_line(
        case_id,
        &epiora_import::import(bytes, file_name, &settings),
    ))
}
