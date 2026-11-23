#!/usr/bin/env bash
# Builds the epiora-mdb wasm module into app/public/mdb/ with the pinned
# emsdk toolchain (spike report docs/spikes/mdb-wasm.md):
#   emsdk 6.0.13 @ d7b4130688bfa348392b921ab04015aa15c4f154
#   sdk-releases-eb132757325583b787fe9414b09afea9442b72ad-64bit
# Usage: EMSDK=/path/to/emsdk ./crates/epiora-mdb/build-wasm.sh
set -euo pipefail

cd "$(dirname "$0")"
EMSDK="${EMSDK:-/tmp/emsdk}"
EMCC="$EMSDK/upstream/emscripten/emcc"
[[ -x "$EMCC" ]] || { echo "emcc not found at $EMCC (set EMSDK)"; exit 1; }

VENDOR=vendor/mdbtools-1.0.1
GEN=$(mktemp -d)
trap 'rm -rf "$GEN"' EXIT
mkdir -p "$GEN/include"
sed -e 's/@HAVE_ICONV_H@/0/' \
    -e 's/@HAVE_XLOCALE_H@/0/' \
    -e 's/@GLIB_INCLUDE_HEADER@/mdbfakeglib.h/' \
    -e 's/@LOCALE_T@/locale_t/' \
    "$VENDOR/include/mdbtools.h.in" > "$GEN/include/mdbtools.h"

"$EMCC" -O2 -o mdb.js \
    "$VENDOR"/src/libmdb/*.c c/harness.c c/eptb.c c/codepages.c \
    -I"$GEN/include" -I"$VENDOR/include" -Ic \
    -DTLS=_Thread_local \
    -sMODULARIZE -sEXPORT_ES6 \
    -sFORCE_FILESYSTEM -sALLOW_MEMORY_GROWTH \
    -sENVIRONMENT=web,node \
    -sEXPORTED_FUNCTIONS=_malloc,_free,_epiora_mdb_open,_epiora_mdb_last_error,_epiora_mdb_last_format,_epiora_mdb_format,_epiora_mdb_tables,_epiora_mdb_batch,_epiora_mdb_close \
    -sEXPORTED_RUNTIME_METHODS=ccall,FS,getValue,HEAPU8,UTF8ToString

OUT=../../app/public/mdb
mkdir -p "$OUT"
mv mdb.js mdb.wasm "$OUT/"
ls -la "$OUT"/mdb.*
