#!/usr/bin/env bash
# Builds the wasm binding (spec §5.1).
#
#   scripts/wasm-build.sh release   → crates/epiora-wasm/pkg/
#                                     (wasm-pack --target web, then the
#                                     pinned wasm-opt -Oz on the .wasm)
#   scripts/wasm-build.sh parity    → crates/epiora-wasm/pkg-parity/
#                                     (wasm-pack --target nodejs with
#                                     the `parity` feature)
#
# wasm-pack runs with --mode no-install and --no-opt: it may not
# download tool binaries — CI installs wasm-bindgen and wasm-opt from
# SHA-256-verified release tarballs — and the only wasm-opt pass is
# ours (-Oz, binaryen version_133). The repo-local verified copies in
# target/bin are preferred when present; WASM_PACK / WASM_OPT override.
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

mode="${1:-release}"
# The repo-local verified tool copies (installed like CI does) sit in
# target/bin; PATH tools are the fallback, WASM_PACK / WASM_OPT
# override. target/bin goes on PATH so wasm-pack's `which wasm-bindgen`
# lookup finds the verified CLI copy too.
[ -d target/bin ] && PATH="$PWD/target/bin:$PATH"
wasm_pack="${WASM_PACK:-}"
[ -z "$wasm_pack" ] && [ -x target/bin/wasm-pack ] && wasm_pack="target/bin/wasm-pack"
wasm_pack="${wasm_pack:-wasm-pack}"
wasm_opt="${WASM_OPT:-}"
[ -z "$wasm_opt" ] && [ -x target/bin/wasm-opt ] && wasm_opt="target/bin/wasm-opt"
wasm_opt="${wasm_opt:-wasm-opt}"

case "$mode" in
  release)
    "$wasm_pack" build crates/epiora-wasm --release --target web \
      --out-dir pkg --no-opt --mode no-install
    wasm="crates/epiora-wasm/pkg/epiora_wasm_bg.wasm"
    "$wasm_opt" -Oz "$wasm" -o "$wasm"
    ;;
  parity)
    "$wasm_pack" build crates/epiora-wasm --release --target nodejs \
      --out-dir pkg-parity --no-opt --mode no-install -- --features parity
    ;;
  *)
    echo "usage: $0 [release|parity]" >&2
    exit 2
    ;;
esac
