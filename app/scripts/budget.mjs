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
// v1.0 Task 14 (spec §10: the calculator route's first load is
// reported, not gated): a second per-file table measures the
// `/calc/$id` first load — index.html, the entry JS/CSS, the core
// engine .wasm, plus the lazily loaded calc route chunk(s) and the
// `calc.worker` chunk, all read from the manifest. sqlite3.wasm and
// import-wasm are excluded: calculators are store-less. The second
// table never changes the exit code.
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
const pick = (set, re, what) => {
  const hit = assets.find((f) => re.test(f));
  if (hit === undefined) fail(`${dist}/assets: ${what} not found`);
  set.add(`assets/${hit}`);
};
const CORE_WASM = /^epiora_wasm_bg-.*\.wasm$/;
pick(files, CORE_WASM, 'core .wasm');
pick(files, /^sqlite3-.*\.wasm$/, 'sqlite3.wasm');

// --- The `/calc/$id` first-load set --------------------------------------
// index.html + the entry JS/CSS + the core engine .wasm (the
// store-less calc worker makes engine calls itself) + the lazily
// loaded calc route chunk(s) and the `calc.worker` chunk. The entry
// files were collected above; re-derive them here so this set stands
// on its own.

const calcFiles = new Set(['index.html']);
for (const entry of Object.values(manifest)) {
  if (entry.isEntry !== true) continue;
  calcFiles.add(entry.file);
  for (const css of entry.css ?? []) calcFiles.add(css);
}
pick(calcFiles, CORE_WASM, 'core .wasm');

// Lazily loaded chunks belonging to the calculator: manifest modules
// under the calc tree — `src/ui/routes/calc*.tsx`,
// `src/ui/screens/calc/*`, `src/calc/*` — plus the chunks each one
// imports. Worker chunks are not manifest keys: Vite lists them under
// the importing chunk's `assets`.
const CALC_MODULE = /(^|[/\\_.-])calc([/\\_.-]|$)/i;
const addChunk = (key) => {
  const entry = manifest[key];
  if (entry === undefined || entry.isEntry === true) return;
  calcFiles.add(entry.file);
  for (const css of entry.css ?? []) calcFiles.add(css);
  for (const imp of entry.imports ?? []) addChunk(imp);
};
let calcWorkerListed = false;
for (const [key, entry] of Object.entries(manifest)) {
  if (
    entry.isEntry !== true &&
    (CALC_MODULE.test(key) ||
      CALC_MODULE.test(entry.src ?? '') ||
      CALC_MODULE.test(entry.name ?? ''))
  ) {
    addChunk(key);
  }
  for (const a of entry.assets ?? []) {
    if (/^assets\/calc\.worker-[^/]+\.js$/.test(a)) {
      calcFiles.add(a);
      calcWorkerListed = true;
    }
  }
}
if (!calcWorkerListed) {
  fail(`${manifestPath}: no calc.worker chunk listed under assets`);
}

// --- Sum and gate --------------------------------------------------------

const printTable = (set, totalLabel) => {
  let sum = 0;
  console.log('| file | gzip -9 bytes |');
  console.log('|---|---|');
  for (const rel of [...set].sort()) {
    const path = join(dist, rel);
    if (!existsSync(path)) fail(`${rel} listed but missing from ${dist}`);
    const bytes = gzip9(path);
    sum += bytes;
    console.log(`| ${rel} | ${bytes} |`);
  }
  console.log(`| **${totalLabel}** | **${sum}** |`);
  return sum;
};

console.log('## First load\n');
const total = printTable(files, 'first-load total');
console.log(
  `\nfirst-load budget: ${total} bytes vs ${limit} limit → ` +
    `${total < limit ? 'PASS' : 'FAIL'} (fails at or above the limit)`,
);

console.log('\n## `/calc/$id` first load\n');
const calcTotal = printTable(calcFiles, '/calc/$id first-load total');
console.log(`\n/calc/$id first load: ${calcTotal} bytes (reported, not gated)`);

if (total >= limit) process.exit(1);
