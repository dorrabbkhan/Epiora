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
cd "$(git rev-parse --show-toplevel)/app"

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

step "Vitest (store-opfs browser suite: chromium, firefox, webkit)"
npx vitest run --project store-opfs

step "licence check"
# --excludePackages: the app package itself is not a dependency.
npx license-checker-rseidelsohn --production \
  --onlyAllow "MIT;Apache-2.0;ISC;BSD-2-Clause;BSD-3-Clause;0BSD;Unlicense;CC0-1.0;BlueOak-1.0.0" \
  --excludePackages 'epiora-app@0.3.0' \
  --summary

[ "$mode" = quick ] && exit 0

step "vite build"
npm run build

step "Playwright (chromium, firefox, webkit)"
npx playwright test
