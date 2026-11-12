#!/usr/bin/env node
// app/scripts/budget.mjs <dist> [limit] — the first-load size gate
// (v0.3 plan Task 17, spec §10.2, platform spec §12.8). Sums the
// gzip -9 sizes of the first-load set:
//
//   index.html
//   + the entry chunk's JS file and CSS listed for the entry in
//     Vite's <dist>/.vite/manifest.json
//   + the core engine .wasm (epiora_wasm_bg-*.wasm)
//   + sqlite3.wasm (sqlite3-*.wasm)
//
// and exits 1 when the total is at or above the limit (default
// 5,000,000 bytes). Everything else shipped in dist — the lazily
// loaded import-wasm, non-entry chunks, icons, the service worker —
// is not first-load and is never counted. Prints a per-file table
// that the workflow tees into the step summary.
//
// The manifest is emitted by `build.manifest: true` in
// app/vite.config.ts.

import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { gzipSync } from 'node:zlib';

const FIRST_LOAD_BUDGET = 5_000_000;

const fail = (msg) => {
  console.error(`budget: ${msg}`);
  process.exit(1);
};

const arg = process.argv[2];
if (arg === undefined) {
  console.error('usage: budget.mjs <dist> [limit-bytes]');
  process.exit(2);
}
const limit =
  process.argv[3] === undefined ? FIRST_LOAD_BUDGET : Number(process.argv[3]);
if (!Number.isInteger(limit) || limit <= 0) {
  console.error(`budget: bad limit ${JSON.stringify(process.argv[3])}`);
  process.exit(2);
}
const dist = resolve(arg);
const gzip9 = (path) => gzipSync(readFileSync(path), { level: 9 }).length;

// --- The first-load set ------------------------------------------------

const files = new Set(['index.html']);

const manifestPath = join(dist, '.vite', 'manifest.json');
if (!existsSync(manifestPath)) {
  fail(`${manifestPath} not found — the build needs build.manifest: true`);
}
const manifest = JSON.parse(readFileSync(manifestPath, 'utf8'));
for (const entry of Object.values(manifest)) {
  if (entry.isEntry !== true) continue;
  files.add(entry.file);
  for (const css of entry.css ?? []) files.add(css);
}

let assets;
try {
  assets = readdirSync(join(dist, 'assets'));
} catch {
  fail(`${dist}/assets not readable`);
}
const pick = (re, what) => {
  const hit = assets.find((f) => re.test(f));
  if (hit === undefined) fail(`${dist}/assets: ${what} not found`);
  files.add(`assets/${hit}`);
};
pick(/^epiora_wasm_bg-.*\.wasm$/, 'core .wasm');
pick(/^sqlite3-.*\.wasm$/, 'sqlite3.wasm');

// --- Sum and gate --------------------------------------------------------

let total = 0;
console.log('| file | gzip -9 bytes |');
console.log('|---|---|');
for (const rel of [...files].sort()) {
  const path = join(dist, rel);
  if (!existsSync(path)) fail(`${rel} listed but missing from ${dist}`);
  const bytes = gzip9(path);
  total += bytes;
  console.log(`| ${rel} | ${bytes} |`);
}
console.log(`| **first-load total** | **${total}** |`);
console.log(
  `\nfirst-load budget: ${total} bytes vs ${limit} limit → ` +
    `${total < limit ? 'PASS' : 'FAIL'} (fails at or above the limit)`,
);
if (total >= limit) process.exit(1);
