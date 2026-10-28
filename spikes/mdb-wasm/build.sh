#!/usr/bin/env bash
# Build libmdb (mdbtools 1.0.1) + harness.c to WebAssembly with Emscripten.
#
#   EMSDK=/path/to/emsdk ./build.sh          # download + build
#   MDBTOOLS_SRC=/path/to/mdbtools-1.0.1 ./build.sh   # reuse existing source
#
# Outputs (committed so `node --test` runs without Emscripten):
#   dist/mdb.js  dist/mdb.wasm
#
# mdbtools is built --disable-glib (its internal fakeglib replaces glib),
# --disable-iconv (deterministic wcstombs path; emscripten's iconv lacks the
# UCS-2LE alias JET4 needs) and --disable-man. The tarball is pinned by
# version and SHA-256.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"

MDB_VERSION=1.0.1
MDB_SHA256=ff9c425a88bc20bf9318a332eec50b17e77896eef65a0e69415ccb4e396d1812
MDB_URL="https://github.com/mdbtools/mdbtools/releases/download/v${MDB_VERSION}/mdbtools-${MDB_VERSION}.tar.gz"

EMSDK="${EMSDK:-/tmp/emsdk}"
build="$here/build"
dist="$here/dist"
mkdir -p "$build" "$dist"

# shellcheck disable=SC1091
source "$EMSDK/emsdk_env.sh" >/dev/null

src="${MDBTOOLS_SRC:-$build/mdbtools-$MDB_VERSION}"
if [ ! -d "$src" ]; then
  tarball="$build/mdbtools-$MDB_VERSION.tar.gz"
  curl -sSL -o "$tarball" "$MDB_URL"
  echo "$MDB_SHA256  $tarball" | sha256sum -c -
  tar xzf "$tarball" -C "$build"
fi

if [ ! -f "$src/Makefile" ]; then
  (cd "$src" && emconfigure ./configure --disable-glib --disable-man --disable-iconv)
fi
emmake make -C "$src/src/libmdb" -j"$(nproc)"

# shellcheck disable=SC2046
emcc -O2 "$here/harness.c" "$src/src/libmdb/.libs/libmdb.a" \
  -I"$src/include" \
  -o "$dist/mdb.js" \
  -sMODULARIZE=1 -sEXPORT_ES6=1 \
  -sFORCE_FILESYSTEM=1 -sALLOW_MEMORY_GROWTH=1 \
  -sEXPORTED_FUNCTIONS=_mdb_version,_mdb_tables,_mdb_export_csv,_malloc,_free \
  -sEXPORTED_RUNTIME_METHODS=ccall,cwrap,FS

ls -l "$dist"
