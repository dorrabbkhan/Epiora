#!/usr/bin/env bash
# Chromium performance gate (spec §5.3): builds the release wasm pkg,
# installs the pinned Playwright from bench/package-lock.json, and runs
# the 1,000,000-row bench in Chromium. Writes
# crates/epiora-wasm/bench/bench-results.json and fails if the median of
# 5 runs of `datasetFromColumns` + stratified `tables` is ≥ 1000 ms.
#
#   scripts/bench.sh
#
# Chromium is installed only under CI (`npx playwright install
# --with-deps chromium`); locally it must already be present — the
# version is pinned by the exact @playwright/test in package-lock.json.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

scripts/wasm-build.sh release

cd crates/epiora-wasm/bench
npm ci
if [ -n "${CI:-}" ]; then
  npx playwright install --with-deps chromium
fi
npx playwright test
