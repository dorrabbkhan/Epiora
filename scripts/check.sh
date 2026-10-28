#!/usr/bin/env bash
# Local replacement for CI. Run from anywhere in the repo.
#
#   ./scripts/check.sh        full local gate: run once before pushing
#   ./scripts/check.sh quick  iteration gate: only spikes whose folder changed
#
# Steps that need something a later task creates are skipped until it exists
# (e.g. scripts/check-spikes.sh arrives in v0.1 Task 1). E2E=1 opts into
# Playwright spikes locally; CI-only spikes (Windows, macOS, Android emulator)
# run on GitHub. ALLOW_FIXTURE_CHANGE=1 skips the fixture guard (owner only).
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

mode="${1:-}"
case "$mode" in ""|quick) ;; *) echo "usage: $0 [quick]" >&2; exit 2 ;; esac

base="${BASE:-$(git merge-base HEAD origin/main 2>/dev/null || git rev-list --max-parents=0 HEAD)}"
changed="$(git diff --name-only "$base"; git diff --name-only; git ls-files --others --exclude-standard)"

step() { printf '\n== %s\n' "$1"; }

# Fixture guard: fixtures never change together with engine or spike code,
# and only through fixtures/generate.R (AGENTS.md rule 1).
step "fixture guard"
if [ "${ALLOW_FIXTURE_CHANGE:-0}" != 1 ] && grep -q '^fixtures/' <<<"$changed"; then
  if grep -qE '^(crates|spikes)/' <<<"$changed"; then
    echo "fixtures/ changed together with crates/ or spikes/ — not allowed" >&2; exit 1
  fi
  echo "fixtures/ changed: allowed only via fixtures/generate.R in the fixture task"
fi

step "spike reports"
if [ -x scripts/check-spikes.sh ]; then scripts/check-spikes.sh; else echo "skipped (checker not created yet)"; fi

step "shell tests"
if [ -d scripts/tests ] && command -v bats >/dev/null; then bats scripts/tests; else echo "skipped"; fi

for dir in spikes/*/; do
  [ -d "$dir" ] || continue
  name="${dir%/}"
  if [ "$mode" = quick ] && ! grep -q "^$name/" <<<"$changed"; then continue; fi
  if [ -f "$dir/Cargo.toml" ]; then
    step "cargo test $name"
    cargo fmt --manifest-path "$dir/Cargo.toml" --check
    cargo clippy --manifest-path "$dir/Cargo.toml" --all-targets -- -D warnings
    cargo test --manifest-path "$dir/Cargo.toml"
  fi
  if [ -d "$dir/test" ] && ls "$dir"/test/*.test.mjs >/dev/null 2>&1; then
    step "node --test $name"; node --test "$dir"/test/
  fi
  if ls "$dir"/*.spec.mjs >/dev/null 2>&1; then
    if [ "${E2E:-0}" = 1 ]; then step "playwright $name"; npx playwright test "$dir"; else echo "playwright $name skipped (E2E=1 to run)"; fi
  fi
done

if [ "$mode" = quick ]; then printf '\nQUICK CHECKS PASSED (run ./scripts/check.sh before pushing)\n'
else printf '\nALL CHECKS PASSED\n'; fi
