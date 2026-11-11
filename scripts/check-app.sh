#!/usr/bin/env bash
# App gate for app/ (v0.3 spec §10.1). Called by scripts/check.sh when
# app/ exists; also runnable directly.
#
#   scripts/check-app.sh quick   npm ci (if needed), tsc --noEmit,
#                                ESLint, Stylelint, tokens regen +
#                                diff check, Vitest, licence check
#   scripts/check-app.sh full    quick + vite build + Playwright
#                                (chromium, firefox, webkit)
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
cd "$root/app"

mode="${1:-full}"
case "$mode" in
  quick|full) ;;
  *) echo "usage: $0 [quick|full]" >&2; exit 2 ;;
esac

step() { printf '\n== %s\n' "$1"; }

step "npm ci"
if [ ! -d node_modules ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  npm ci
else
  echo "node_modules up to date"
fi

step "wasm packages (epiora-wasm, epiora-import-wasm)"
# The worker's @engine/@import aliases point at crates/*/pkg, which is a
# git-ignored build artifact — build both before tsc, Vitest or vite
# build needs them.
"$root/scripts/wasm-build.sh" release epiora-wasm
"$root/scripts/wasm-build.sh" release epiora-import-wasm

step "store-stdio binary (native conformance)"
# Task 14: the native backend's TypeScript conformance suite runs
# against the test-only store-stdio binary. The store crate is pure
# Rust, so this needs no Tauri system deps.
if command -v cargo >/dev/null && [ -d "$root/app/src-tauri" ]; then
  cargo build --manifest-path "$root/app/src-tauri/Cargo.toml" \
    -p epiora-store-native --features stdio
else
  echo "skipped (no cargo or no app/src-tauri yet)"
fi

step "tsc --noEmit"
npx tsc --noEmit

step "ESLint"
npx eslint .

step "Stylelint"
npx stylelint "**/*.css"

step "tokens regenerated"
node src/design/build-tokens.mjs
git diff --exit-code -- src/design/tokens.css

step "Vitest (unit)"
npx vitest run --project unit

step "node --test scripts (budget gate tests)"
# Plain node:test files under app/scripts (v0.3 Task 17: budget.mjs).
if compgen -G 'scripts/*.test.mjs' >/dev/null; then
  node --test scripts/*.test.mjs
else
  echo "no script tests"
fi

step "Vitest (store-opfs browser suite: chromium, firefox, webkit)"
npx vitest run --project store-opfs

step "licence check"
# --excludePackages: the app package itself is not a dependency.
npx license-checker-rseidelsohn --production \
  --onlyAllow "MIT;Apache-2.0;ISC;BSD-2-Clause;BSD-3-Clause;0BSD;Unlicense;CC0-1.0;BlueOak-1.0.0" \
  --excludePackages 'epiora-app@0.3.0' \
  --summary

[ "$mode" = quick ] && exit 0

step "native store (Task 14: cargo deny + cargo test)"
# `cargo test --workspace` compiles the epiora-app bin, which links
# webkit2gtk/gtk — run it only where the Tauri system deps exist (the
# app.yml native-store job installs them). The store crate itself is
# always tested above via native.test.ts + store-stdio.
if command -v cargo >/dev/null && [ -d "$root/app/src-tauri" ]; then
  (cd "$root/app/src-tauri" && cargo deny check)
  if pkg-config --exists webkit2gtk-4.1; then
    (cd "$root/app/src-tauri" && cargo test --workspace)
  else
    echo "cargo test --workspace skipped (no webkit2gtk-4.1 dev files; CI's native-store job runs it)"
    (cd "$root/app/src-tauri" && cargo test -p epiora-store-native --all-features)
  fi
else
  echo "skipped (no cargo or no app/src-tauri yet)"
fi

step "vite build"
# VITE_TEST_HOOKS=1 exposes the Task 8 store test hook through the
# worker RPC so the e2e suite can simulate quota exhaustion (Task 11).
# Only this e2e build lane sets the flag; production builds run
# `vite build` without it.
VITE_TEST_HOOKS=1 npm run build

step "Playwright (chromium, firefox, webkit)"
npx playwright test
