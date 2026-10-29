use std::io::Read;
use tauri::Manager;

/// First 16 bytes of `epiora.db` in the app config dir, hex-encoded. A
/// SQLite3MultipleCiphers-encrypted file does not start with the plain
/// SQLite magic `SQLite format 3\0`, which is what the spike reports.
#[tauri::command]
fn db_header(app: tauri::AppHandle) -> Result<String, String> {
    let path = app
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("epiora.db");
    let mut f = std::fs::File::open(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut buf = [0u8; 16];
    f.read(&mut buf).map_err(|e| e.to_string())?;
    Ok(buf.iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .invoke_handler(tauri::generate_handler![db_header])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
