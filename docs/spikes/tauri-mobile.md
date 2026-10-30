# Tauri 2 shell on Android and iOS

**Status:** go-with-changes

## Question

Plan Task 10 (spec §7.1 `epiora-app`: "the same code in a Tauri 2 shell
for desktop, Android and iOS"; §11.1: "whole-database encryption in
Tauri apps with SQLite3MultipleCiphers"; §14 Phase 0): can a Tauri 2
app on Android and iOS (a) run a Rust wasm statistics function in the
webview and (b) open a native SQLite file through Tauri's SQL plugin
built with SQLite3MultipleCiphers?

## Method

`spikes/tauri-shell/` was scaffolded with
`npm create tauri-app@4.7.5 tauri-shell --template vanilla --manager npm
--identifier com.epiora.spike -y`, then edited:

- `stats-wasm/` is a `cdylib` crate exporting
  `#[no_mangle] pub extern "C" fn risk_ratio(a: f64, b: f64, c: f64,
  d: f64) -> f64` (the plan's signature), built by
  `./build-wasm.sh` (`cargo build --target wasm32-unknown-unknown
  --release`) into `src/stats_wasm.wasm`. No wasm-bindgen — the page
  calls `instance.exports.risk_ratio(43, 11, 3, 18)` after
  `WebAssembly.instantiate` and renders `2x2 RR = 5.57` (Oswego
  2×2: 43/11 vs 3/18, matching `fixtures/2x2/oswego_vanilla.json`
  `risk_ratio.estimate` 5.574074…, `toFixed(2)` → "5.57").
- `tauri-plugin-sql` (sqlx) opens `sqlite:epiora.db` in the app config
  dir. sqlx's sqlite driver builds its bundled SQLite through
  `libsqlite3-sys`; `src-tauri/Cargo.toml` `[patch.crates-io]` points it
  at `src-tauri/vendor/libsqlite3-sys`, a copy of 0.30.1 whose build.rs
  compiles the `sqlite3mc-src` crate's untouched
  `sqlite3mc_amalgamation.c` (SQLite3MultipleCiphers 2.5.1 wrapping
  SQLite 3.53.4) instead of the stock `sqlite3.c`. The page then runs
  `PRAGMA key = 'epiora-spike-key'` (the plugin exposes no key option —
  plugins-workspace PR #2553 is still open), creates a table, inserts a
  row and reads it back, and displays `sqlite3mc_version()`,
  `PRAGMA cipher` and the file's first 16 bytes via the `db_header`
  command to prove the file is not plain SQLite.
- `.github/workflows/spike-tauri.yml`: the `android` job builds a debug
  APK for `x86_64`, runs it on an API-34 emulator and asserts
  `adb shell uiautomator dump` contains `RR = 5.57`; the `ios` job runs
  `tauri ios build --debug --target aarch64-sim`, installs the .app on
  a created iPhone 15 simulator and asserts `simctl launch` returns the
  app PID.

## Environment

- GitHub Actions `ubuntu-latest` (Android SDK
  `/usr/local/lib/android/sdk`, NDK 30.0.16248370 via
  `ANDROID_NDK_LATEST_HOME`, Temurin JDK 17.0.21-9) and `macos-latest`
  (Xcode 26.6, iOS Simulator SDK 26.5); rustc 1.99.1; Node 22;
  @tauri-apps/cli 2.12.4, tauri 2.12.x, tauri-plugin-sql 2.6.0
  (resolved versions: `src-tauri/Cargo.lock`), sqlite3mc-src
  205.1.0+sqlite3mc-2.5.1-sqlite-3.53.4 (lockfile checksum
  f155b7d3…4478c).
- No RNG used; no seeded randomness.

## Measurements

- First run
  [40970540222](https://github.com/dorrabbkhan/Epiora/actions/runs/40970540222):
  FAIL as expected — `spikes/tauri-shell` did not exist yet; both jobs
  stopped at `npm ci` ("No such file or directory"). Recorded as the
  plan Step 2 "expected FAIL".
- Run
  [40970658633](https://github.com/dorrabbkhan/Epiora/actions/runs/40970658633):
  - iOS: **pass** — `tauri ios build --debug --target aarch64-sim`
    produced `gen/apple/build/arm64-sim/tauri-shell.app`; created
    iPhone 15 sim `422372B7-…`, `simctl launch` printed
    `com.epiora.spike: 8655` (PID).
  - Android: APK built but emulator check **failed** —
    `app-universal-debug.apk` 154,164,876 bytes (debug build, x86_64
    `.so` only), then `reactivecircus/android-emulator-runner` fell back
    to `-accel off` ("Linux VM where hardware acceleration is not
    available") and the emulator never booted within the timeout
    ("Timeout waiting for emulator to boot").
  - Host-side proof of the sqlite3mc link (local command):
    `cargo run` in a scratch crate depending on `sqlx 0.8.6` with the
    same `[patch.crates-io]` override — `PRAGMA key` + create/insert/
    select read back the row, `sqlite3mc_version()` → "SQLite3 Multiple
    Ciphers 2.5.1", `PRAGMA cipher` → "chacha20", and the file's first
    16 bytes are ciphertext (no `SQLite format 3` magic).
- Runs
  [40972949756](https://github.com/dorrabbkhan/Epiora/actions/runs/40972949756),
  [40975058698](https://github.com/dorrabbkhan/Epiora/actions/runs/40975058698),
  [40976143137](https://github.com/dorrabbkhan/Epiora/actions/runs/40976143137),
  [40977142033](https://github.com/dorrabbkhan/Epiora/actions/runs/40977142033),
  [40978035931](https://github.com/dorrabbkhan/Epiora/actions/runs/40978035931),
  [40978903685](https://github.com/dorrabbkhan/Epiora/actions/runs/40978903685),
  [40979790307](https://github.com/dorrabbkhan/Epiora/actions/runs/40979790307):
  intermediate Android failures, all recorded — with KVM enabled the
  `default` image still never reached adb (silent post-boot hang,
  emulator 37.2.13); `google_apis` fixed the boot, then the action's
  per-line `sh -c` script semantics and `GITHUB_WORKSPACE` not being
  visible to it each needed a fix. iOS stayed green in every run.
- Run
  [40980661439](https://github.com/dorrabbkhan/Epiora/actions/runs/40980661439):
  **both jobs pass.** Android: `app-universal-debug.apk`
  154,164,876 bytes installed on the API-34 google_apis x86_64
  emulator, app launched via monkey, `uiautomator dump` contains
  `RR = 5.57` ("FOUND: RR = 5.57" in the step log, `ui.xml` in the
  `tauri-android` artifact). iOS: `tauri-shell.app` built for
  aarch64-sim and launched on an iPhone 15 simulator
  (`simctl launch` printed the app PID).
- APK size and cold-start on the D7 reference device: **human step
  (plan Task 10 Step 4) — pending the owner's real-device run.** The
  debug universal APK size above is the CI number; it is not the D7
  measurement.

## Decision

**Tauri for desktop and mobile** (plan Step 5 option, verbatim) — the
spike proves the mobile shell runs on both platforms' CI, so Status is
go-with-changes: the option stands, with these recorded changes to how
v0.3 should build it:

1. Database encryption must not rely on a raw `PRAGMA key` execute —
   sqlx pools open fresh connections for concurrent work and each new
   connection is unkeyed. For v0.3 the key must be applied per
   connection (upstream tauri-plugin-sql pragma support, a small fork,
   or connect options via our own wrapper).
2. The sqlite3mc link is a `[patch.crates-io]` on a vendored
   `libsqlite3-sys` — acceptable to keep in v0.3 but it must track
   libsqlite3-sys/sqlx version bumps, or move to a first-class -sys
   crate when one exists for sqlite3mc.
3. `src-tauri/gen/` is generated by `tauri android|ios init` in CI
   rather than committed; revisit for release signing.

## Consequences

- Tauri 2 works on Android emulator and iOS simulator CI with only the
  KVM udev-rule gotcha; Android emulator tests (spec §12.7) are viable
  on `ubuntu-latest` once `/dev/kvm` perms are granted.
- Whole-database encryption in Tauri via SQLite3MultipleCiphers
  (spec §11.1) is proven end-to-end with `PRAGMA key` and the chacha20
  default cipher; the per-connection keying gap (Decision item 1) is a
  v0.3 blocker note, not an architecture change.
- A Rust wasm stats function runs in the mobile webview with no
  wasm-bindgen — the v0.2 `epiora-wasm` plan can reuse this loading
  pattern.

## Licences

- tauri 2.x (MIT OR Apache-2.0); tauri-plugin-sql 2.6.0 (MIT OR
  Apache-2.0); sqlx 0.8.6 (MIT OR Apache-2.0) — all Cargo deps, static.
- libsqlite3-sys 0.30.1 (MIT) — vendored, statically linked, only the
  bundled C source is swapped (see vendor README note).
- SQLite3MultipleCiphers amalgamation via sqlite3mc-src
  205.1.0+sqlite3mc-2.5.1-sqlite-3.53.4 (MIT AND CC0-1.0 AND Unlicense
  AND (CC0-1.0 OR Apache-2.0) per its crate; SQLite itself public
  domain) — statically linked.
- @tauri-apps/cli 2.12.4 (MIT OR Apache-2.0) — devDependency only.
- reactivecircus/android-emulator-runner@v2, actions/*,
  dtolnay/rust-toolchain — MIT-licensed Actions; CI only.
- stats-wasm / spike code — AGPL-3.0-only (repo licence).
