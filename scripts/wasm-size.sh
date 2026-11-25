#!/usr/bin/env bash
# scripts/wasm-size.sh <file.wasm> [limit-bytes] — prints the
# `gzip -9` size in bytes and exits 1 when it exceeds the limit
# (default 409,600 — the engine .wasm budget, spec §5.4, E3). The
# import .wasm gate passes the owner's 573,616 limit (v0.3 Task 17,
# PR #52).
#
# scripts/wasm-size.sh report — prints (never gates) the gzip -9
# sizes of the lazily-loaded modules: epiora-legacy-wasm,
# epiora-lang-wasm and the mdb module. Missing builds are reported,
# not fatal (v1.1 Task 13).
set -euo pipefail
if [ "${1:-}" = "report" ]; then
  for pair in \
    "epiora-legacy-wasm crates/epiora-legacy-wasm/pkg/epiora_legacy_wasm_bg.wasm" \
    "epiora-lang-wasm crates/epiora-lang-wasm/pkg/epiora_lang_wasm_bg.wasm" \
    "mdb app/public/mdb/mdb.wasm"; do
    set -- $pair
    if [ -f "$2" ]; then
      printf '%s: %s\n' "$1" "$(gzip -9 -c "$2" | wc -c | tr -d ' ')"
    else
      printf '%s: not built\n' "$1"
    fi
  done
  exit 0
fi
file="${1:?usage: wasm-size.sh <file.wasm> [limit-bytes]|report}"
limit="${2:-409600}"
bytes="$(gzip -9 -c "$file" | wc -c | tr -d ' ')"
echo "$bytes"
if [ "$bytes" -gt "$limit" ]; then
  echo "wasm size gate: ${bytes} bytes > ${limit} after gzip -9" >&2
  exit 1
fi
