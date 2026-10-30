#!/usr/bin/env bash
# Spike report checker. Usage: scripts/check-spikes.sh [DIR]
#
# Exits 0 when every *.md file in DIR (default: docs/spikes) except
# TEMPLATE.md and README.md (the v0.1 index, not a report) contains the headings:
#   ## Question  ## Method  ## Environment  ## Measurements
#   ## Decision  ## Consequences  ## Licences
# and a line "**Status:** go|no-go|go-with-changes".
# Exits 1 and names the file and the missing heading otherwise.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

dir="${1:-docs/spikes}"
if [ ! -d "$dir" ]; then
  echo "check-spikes: directory not found: $dir" >&2
  exit 1
fi

headings=(Question Method Environment Measurements Decision Consequences Licences)
fail=0

for f in "$dir"/*.md; do
  [ -e "$f" ] || continue
  case "$(basename "$f")" in TEMPLATE.md|README.md) continue ;; esac
  for h in "${headings[@]}"; do
    if ! grep -q "^## $h\b" "$f"; then
      echo "$f: missing '## $h'" >&2
      fail=1
    fi
  done
  if ! grep -Eq '^\*\*Status:\*\*[[:space:]]+(go|go-with-changes|no-go)[[:space:]]*$' "$f"; then
    echo "$f: missing or invalid '**Status:** go|no-go|go-with-changes' line" >&2
    fail=1
  fi
done

exit "$fail"
