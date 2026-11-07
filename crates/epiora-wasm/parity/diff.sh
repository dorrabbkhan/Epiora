#!/usr/bin/env bash
# crates/epiora-wasm/parity/diff.sh <a.jsonl> <b.jsonl>
# The native and wasm parity files must be identical bit for bit
# (spec §5.2): exit 0 when they are, exit 1 naming the fixture ids of
# the differing lines otherwise.
set -euo pipefail
a="${1:?usage: diff.sh <a.jsonl> <b.jsonl>}"
b="${2:?usage: diff.sh <a.jsonl> <b.jsonl>}"
if cmp -s "$a" "$b"; then
  exit 0
fi
# The top-level "id" (engine corpus) or "case" (import corpus) key is
# the only unescaped one on a parity line — a receipt's inner ids are
# backslash-escaped inside the JSON string.
ids="$(diff "$a" "$b" | grep -o '"\(id\|case\)":"[^"]*"' | sort -u || true)"
echo "parity diff: files differ" >&2
if [ -n "$ids" ]; then
  echo "$ids" | sed 's/.*"\(id\|case\)":"\(.*\)".*/  \2/' | sort -u >&2
fi
exit 1
