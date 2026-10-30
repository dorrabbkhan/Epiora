// Epiora Tauri spike page: proves (a) a Rust wasm function runs in the
// mobile webview and (b) the SQL plugin opens a native SQLite file built
// with SQLite3MultipleCiphers, keyed via `PRAGMA key`.

async function showRiskRatio() {
  const el = document.querySelector("#rr");
  try {
    // Oswego 2x2: ate vanilla ice cream x ill — 43 ill / 11 well among
    // exposed, 3 ill / 18 well among unexposed.
    const bytes = await fetch("stats_wasm.wasm").then((r) => r.arrayBuffer());
    const { instance } = await WebAssembly.instantiate(bytes);
    const rr = instance.exports.risk_ratio(43, 11, 3, 18);
    el.textContent = `2x2 RR = ${rr.toFixed(2)}`;
  } catch (e) {
    el.textContent = `2x2 RR = wasm error: ${e}`;
  }
}

async function showDatabase() {
  const el = document.querySelector("#db");
  try {
    const Database = window.__TAURI__.sql;
    const db = await Database.load("sqlite:epiora.db");
    // sqlite3mc keys the connection; it must run before any file access on
    // this pooled connection (the plugin exposes no key option — see report).
    await db.execute("PRAGMA key = 'epiora-spike-key'");
    await db.execute("CREATE TABLE IF NOT EXISTS marker (v TEXT NOT NULL)");
    await db.execute("INSERT INTO marker (v) VALUES ('epiora-sqlite3mc-ok')");
    const rows = await db.select("SELECT v FROM marker ORDER BY rowid LIMIT 1");
    const version = await db.select("SELECT sqlite3mc_version() AS v");
    const cipher = await db.select("PRAGMA cipher");
    const header = await window.__TAURI__.core.invoke("db_header");
    // `PRAGMA cipher` returns one row whose column name is the cipher name
    // itself ({ "chacha20": "chacha20" }), so read the value positionally.
    const cipherName = Object.values(cipher[0] ?? {})[0];
    // First bytes of a plain SQLite file, hex — proves epiora.db is ciphertext.
    const sqliteMagic = "53514c69746520666f726d61742033";
    const enc = header.startsWith(sqliteMagic) ? "PLAINTEXT" : "encrypted";
    el.textContent =
      `sqlite3mc: ${version[0].v} | cipher: ${cipherName}` +
      ` | read-back: ${rows[0].v} | header: ${header} (${enc})`;
  } catch (e) {
    el.textContent = `sqlite3mc: error: ${e}`;
  }
}

window.addEventListener("DOMContentLoaded", () => {
  showRiskRatio();
  showDatabase();
});
