// Epiora desktop shell (v0.3 plan Task 14, spec §7.3): a Tauri 2 window
// over the same `app/dist` the web build serves. At startup the
// keychain key (`keystore.rs`) opens the native SQLite3MultipleCiphers
// store; the managed `StoreState` then answers the five store commands
// (`commands.rs`) for the worker's relay. The key never crosses IPC —
// a startup failure is remembered as a code, not retried with a
// plaintext fallback.

// Prevents an additional console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod keystore;

use commands::StoreState;
use keystore::{KeySource, OsKeychain};
use tauri::Manager;

const DB_FILE: &str = "epiora.db";

fn open_store(app: &tauri::App) -> Result<StoreState, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("app data dir: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("app data dir: {e}"))?;
    let path = dir.join(DB_FILE);
    Ok(match OsKeychain.get_or_create() {
        Err(e) => StoreState::failed(e.code(), e.to_string()),
        Ok(key) => match epiora_store_native::NativeStore::open(&path, &key) {
            Ok(store) => StoreState::ready(store),
            Err(e) => StoreState::failed(e.code().as_str(), e.to_string()),
        },
    })
}

fn main() {
    let result = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            app.manage(open_store(app).map_err(|e| e.to_string())?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::store_exec,
            commands::store_query,
            commands::store_tx_begin,
            commands::store_tx_commit,
            commands::store_tx_rollback,
        ])
        .run(tauri::generate_context!());
    if let Err(e) = result {
        eprintln!("epiora: {e}");
        std::process::exit(1);
    }
}
