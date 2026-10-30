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

/// Ensures `epiora.db` exists as a keyed SQLite3MultipleCiphers file before
/// the page asks the SQL plugin to open it. The plugin's auto-create would
/// leave a plain SQLite header in place; sqlx executes its `key` pragma
/// first on connect, so opening with it produces a keyed file.
#[tauri::command]
async fn db_prepare(app: tauri::AppHandle) -> Result<String, String> {
    use sqlx::{sqlite::SqliteConnectOptions, ConnectOptions, Connection, Executor};

    let dir = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = dir.join("epiora.db");
    let mut conn = SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true)
        .pragma("key", "'epiora-spike-key'")
        .connect()
        .await
        .map_err(|e| e.to_string())?;
    (&mut conn)
        .execute("CREATE TABLE IF NOT EXISTS marker (v TEXT NOT NULL)")
        .await
        .map_err(|e| e.to_string())?;
    conn.close().await.map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::default().build())
        .invoke_handler(tauri::generate_handler![db_header, db_prepare])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
