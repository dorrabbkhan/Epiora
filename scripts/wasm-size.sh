#!/usr/bin/env bash
# scripts/wasm-size.sh <file.wasm> — prints the `gzip -9` size in bytes
# and exits 1 above the 409,600-byte budget (spec §5.4, E3).
set -euo pipefail
file="${1:?usage: wasm-size.sh <file.wasm>}"
bytes="$(gzip -9 -c "$file" | wc -c | tr -d ' ')"
echo "$bytes"
if [ "$bytes" -gt 409600 ]; then
  echo "wasm size gate: ${bytes} bytes > 409,600 after gzip -9" >&2
  exit 1
fi
