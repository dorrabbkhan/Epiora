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

step "corpus-runner smoke run"
if [ -f crates/epiora-core/src/bin/corpus-runner.rs ]; then
  cargo run -p epiora-core --features corpus --bin corpus-runner -- fixtures > target/corpus-native.jsonl
  wc -l target/corpus-native.jsonl
else echo "skipped (arrives in v0.2 Task 8)"; fi

step "wasm build (release + parity, same GIT_SHA)"
if [ -x scripts/wasm-build.sh ]; then
  scripts/wasm-build.sh release
  scripts/wasm-build.sh parity
else echo "skipped (arrives in v0.2 Task 10)"; fi

step "wasm size gate (409,600 bytes after gzip -9)"
if [ -x scripts/wasm-size.sh ]; then scripts/wasm-size.sh crates/epiora-wasm/pkg/epiora_wasm_bg.wasm
else echo "skipped (arrives in v0.2 Task 10)"; fi

step "native/wasm parity diff"
if [ -x crates/epiora-wasm/parity/diff.sh ]; then
  node crates/epiora-wasm/parity/run.mjs fixtures > target/corpus-wasm.jsonl
  wc -l target/corpus-wasm.jsonl
  crates/epiora-wasm/parity/diff.sh target/corpus-native.jsonl target/corpus-wasm.jsonl
else echo "skipped (arrives in v0.2 Task 10)"; fi

step "TypeScript bindings diff"
if [ -d crates/epiora-wasm/bindings ]; then
  # Regenerate then diff: the export dir and number mapping are pinned
  # in .cargo/config.toml (crates/epiora-wasm/bindings, u64 → number).
  cargo test -p epiora-core --features ts --lib export_bindings
  git diff --exit-code -- crates/epiora-wasm/bindings
  if [ -n "$(git ls-files --others --exclude-standard -- crates/epiora-wasm/bindings)" ]; then
    echo "untracked files in crates/epiora-wasm/bindings" >&2
    exit 1
  fi
else echo "skipped (arrives in v0.2 Task 10)"; fi

step "node binding tests"
if [ -d crates/epiora-wasm/tests/node ]; then
  # The native Oswego content hash comes out of the corpus-runner
  # receipts produced above (receipt.dataset.sha256).
  NATIVE_OSWEGO_HASH="$(node -e '
    const fs = require("fs");
    for (const line of fs.readFileSync("target/corpus-native.jsonl", "utf8").split("\n")) {
      if (!line.trim()) continue;
      const row = JSON.parse(line);
      if (row.receipt) {
        const receipt = JSON.parse(row.receipt);
        if (receipt.dataset.id === "oswego") { console.log(receipt.dataset.sha256); break; }
      }
    }')" \
  NATIVE_OSWEGO_SNAPSHOT_HEX="$(cargo run -p epiora-core --features corpus --bin corpus-runner -- --snapshot-hex oswego fixtures)" \
    node --test "crates/epiora-wasm/tests/node/"*.test.mjs
else echo "skipped (arrives in v0.2 Task 10)"; fi

if [ "${E2E:-0}" = 1 ]; then
  step "chromium benchmark"
  if [ -x scripts/bench.sh ]; then scripts/bench.sh
  else echo "skipped (arrives in v0.2 Task 11)"; fi
fi
