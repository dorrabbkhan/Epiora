#!/usr/bin/env bash
# Engine gate for crates/ (v0.2 spec §7). Called by scripts/check.sh
# when crates/ exists; also runnable directly.
#
#   scripts/check-engine.sh quick   fmt + clippy + tests + cargo deny
#   scripts/check-engine.sh full    quick + every step later tasks
#                                   register (wasm build, size, parity,
#                                   bindings), each skipped until it
#                                   exists. E2E=1 adds the benchmark.
#
# Offline machines: set CARGO_NET_OFFLINE=true. cargo then resolves from
# the local registry cache and cargo-deny runs --offline against a copy
# of $CARGO_HOME/advisory-dbs mirrored into target/ (deny.toml's
# db-path), needed when the cargo home is read-only.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

mode="${1:-full}"
case "$mode" in
  quick|full) ;;
  *) echo "usage: $0 [quick|full]" >&2; exit 2 ;;
esac

step() { printf '\n== %s\n' "$1"; }

step "cargo fmt --check"
cargo fmt --check

step "cargo clippy"
cargo clippy --workspace --all-targets --all-features -- -D warnings

step "cargo test"
cargo test --workspace --all-features

step "cargo deny"
deny_args=()
if [ "${CARGO_NET_OFFLINE:-}" = "true" ]; then
  deny_args+=(--offline)
  src="${CARGO_HOME:-$HOME/.cargo}/advisory-dbs"
  if [ -d "$src" ] && [ ! -w "$src" ]; then
    # cp keeps the source's read-only modes, so recopy from scratch and
    # make the mirror writable for cargo-deny's lock file and re-copies.
    rm -rf target/advisory-dbs
    mkdir -p target/advisory-dbs
    cp -r "$src/." target/advisory-dbs/
    chmod -R u+w target/advisory-dbs
  fi
fi
cargo deny "${deny_args[@]}" check

[ "$mode" = quick ] && exit 0

step "wasm build"
if [ -x scripts/wasm-build.sh ]; then scripts/wasm-build.sh
else echo "skipped (arrives in v0.2 Task 10)"; fi

step "wasm size gate"
if [ -x scripts/wasm-size.sh ]; then scripts/wasm-size.sh
else echo "skipped (arrives in v0.2 Task 10)"; fi

step "native/wasm parity"
if [ -x scripts/parity.sh ]; then scripts/parity.sh
else echo "skipped (arrives in v0.2 Task 10)"; fi

step "TypeScript bindings diff"
if [ -x scripts/bindings-check.sh ]; then scripts/bindings-check.sh
else echo "skipped (arrives in v0.2 Task 10)"; fi

if [ "${E2E:-0}" = 1 ]; then
  step "chromium benchmark"
  if [ -x scripts/bench.sh ]; then scripts/bench.sh
  else echo "skipped (arrives in v0.2 Task 11)"; fi
fi
