#!/usr/bin/env bash
# scripts/wasm-size.sh <file.wasm> [limit-bytes] — prints the
# `gzip -9` size in bytes and exits 1 when it exceeds the limit
# (default 409,600 — the engine .wasm budget, spec §5.4, E3). The
# import .wasm gate passes the owner's 573,616 limit (v0.3 Task 17,
# PR #52).
set -euo pipefail
file="${1:?usage: wasm-size.sh <file.wasm> [limit-bytes]}"
limit="${2:-409600}"
bytes="$(gzip -9 -c "$file" | wc -c | tr -d ' ')"
echo "$bytes"
if [ "$bytes" -gt "$limit" ]; then
  echo "wasm size gate: ${bytes} bytes > ${limit} after gzip -9" >&2
  exit 1
fi
