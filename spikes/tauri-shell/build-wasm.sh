#!/usr/bin/env bash
# Build the stats wasm module the Tauri webview loads (src/stats_wasm.wasm).
set -euo pipefail
cd "$(dirname "$0")/stats-wasm"

cargo build --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/stats_wasm.wasm ../src/stats_wasm.wasm
ls -l ../src/stats_wasm.wasm
