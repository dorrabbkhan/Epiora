#!/usr/bin/env bash
# Download the pinned 7mind/sqlcipher-wasm v1.4.0 release assets and the
# MIT-licensed JS wrapper + LICENSE from the same tag, verifying SHA-256
# of every download (AGENTS.md: pinned by version and SHA-256, licence
# recorded from the download itself).
#
#   vendor/fetch-vendor.sh   →  vendor/dist/sqlcipher.mjs
#                               vendor/lib/sqlite-api.mjs  (imports ../dist/sqlcipher.mjs)
#                               vendor/LICENSE
set -euo pipefail
cd "$(dirname "$0")"

ZIP=sqlcipher-wasm-1.4.0.zip
ZIP_SHA256=a2bc6de71afac3a64d826f2308f63660c53cd9bbd98d2ff7b52a3da30ce10cb7
API_SHA256=312b0c9e62a35350970be054accaeb7177dfaa358f9e2a5843978f58e2d1e2a8
LICENSE_SHA256=b35f6d420c13971d97ae7f474ecc9b616eed878974198225366d3fa3dcd1187e
TAG=v1.4.0

curl -fsSL -o "$ZIP" \
  "https://github.com/7mind/sqlcipher-wasm/releases/download/$TAG/sqlcipher-wasm.zip"
echo "$ZIP_SHA256  $ZIP" | sha256sum -c -

mkdir -p dist lib
# The ES-module glue embeds the wasm as base64; the zip's .wasm/.cjs
# siblings are not needed in the browser.
unzip -o -j "$ZIP" sqlcipher.mjs -d dist/
rm "$ZIP"

curl -fsSL -o lib/sqlite-api.mjs \
  "https://raw.githubusercontent.com/7mind/sqlcipher-wasm/$TAG/lib/sqlite-api.mjs"
echo "$API_SHA256  lib/sqlite-api.mjs" | sha256sum -c -

curl -fsSL -o LICENSE \
  "https://raw.githubusercontent.com/7mind/sqlcipher-wasm/$TAG/LICENSE"
echo "$LICENSE_SHA256  LICENSE" | sha256sum -c -

echo "vendored: $(wc -c < dist/sqlcipher.mjs) bytes dist/sqlcipher.mjs," \
  "$(wc -c < lib/sqlite-api.mjs) bytes lib/sqlite-api.mjs"
