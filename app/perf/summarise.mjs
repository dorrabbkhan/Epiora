#!/usr/bin/env node
// app/perf/summarise.mjs [results.jsonl] — turns the perf spec's
// `{scenario, ...}` JSONL lines into median + p90 per numeric field
// per scenario, and reports the gzip -9 sizes of import-wasm and of
// the first-load set (v0.3 plan Task 16, spec §10.3). Markdown on
// stdout; the perf workflow appends it to $GITHUB_STEP_SUMMARY and
// uploads it as an artifact. Measure only — nothing here gates.
//
// p90 is nearest-rank (the smallest sample at or above the 90th
// percentile); median averages the two middle samples at even n.

import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { join } from 'node:path';
import { gzipSync } from 'node:zlib';

const PERF_DIR = fileURLToPath(new URL('.', import.meta.url));
const APP = join(PERF_DIR, '..');
const REPO = join(APP, '..');
const RESULTS =
  process.argv[2] ?? join(REPO, 'target', 'perf', 'results.jsonl');
const DIST = join(APP, 'dist');
const IMPORT_WASM = join(
  REPO,
  'crates/epiora-import-wasm/pkg/epiora_import_wasm_bg.wasm',
);
const FIRST_LOAD_BUDGET = 5_000_000;

const gzip9 = (path) => gzipSync(readFileSync(path), { level: 9 }).length;

function median(sorted) {
  const n = sorted.length;
  const mid = Math.floor(n / 2);
  return n % 2 === 1 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/** Nearest-rank percentile: smallest sample ≥ p of the data. */
function pct(sorted, p) {
  return sorted[Math.max(0, Math.ceil(p * sorted.length) - 1)];
}

const fmt = (x) => (Number.isInteger(x) ? String(x) : x.toFixed(1));

// --- Scenario table -------------------------------------------------

const lines = readFileSync(RESULTS, 'utf8')
  .split('\n')
  .filter((l) => l.trim() !== '')
  .map((l) => JSON.parse(l));

const byScenario = new Map();
for (const line of lines) {
  const { scenario, ...fields } = line;
  delete fields.run;
  let entry = byScenario.get(scenario);
  if (entry === undefined) {
    entry = { n: 0, fields: new Map() };
    byScenario.set(scenario, entry);
  }
  entry.n++;
  for (const [k, v] of Object.entries(fields)) {
    if (typeof v !== 'number') continue;
    const list = entry.fields.get(k) ?? [];
    list.push(v);
    entry.fields.set(k, list);
  }
}

console.log('| scenario | n | field | median | p90 |');
console.log('|---|---|---|---|---|');
for (const [scenario, entry] of [...byScenario.entries()].sort()) {
  for (const [field, values] of [...entry.fields.entries()].sort()) {
    const sorted = [...values].sort((a, b) => a - b);
    console.log(
      `| ${scenario} | ${entry.n} | ${field} | ${fmt(median(sorted))} | ${fmt(pct(sorted, 0.9))} |`,
    );
  }
}

// --- Sizes (gzip -9) --------------------------------------------------

const indexHtml = readFileSync(join(DIST, 'index.html'), 'utf8');
const entryJs = indexHtml.match(/<script type="module"[^>]*src="([^"]+)"/)?.[1];
const entryCss = indexHtml.match(/<link rel="stylesheet"[^>]*href="([^"]+)"/)?.[1];
if (entryJs === undefined || entryCss === undefined) {
  throw new Error('dist/index.html: entry script or stylesheet not found');
}
const strip = (href) => href.replace(/^\//, '');
const assets = readdirSync(join(DIST, 'assets'));
const pick = (re, what) => {
  const hit = assets.find((f) => re.test(f));
  if (hit === undefined) throw new Error(`dist/assets: ${what} not found`);
  return `assets/${hit}`;
};

const firstLoad = [
  'index.html',
  strip(entryJs),
  strip(entryCss),
  pick(/^epiora_wasm_bg-.*\.wasm$/, 'core .wasm'),
  pick(/^sqlite3-.*\.wasm$/, 'sqlite3.wasm'),
];

console.log('\n### Sizes (gzip -9, bytes)');
console.log('| file | bytes |');
console.log('|---|---|');
let firstLoadTotal = 0;
for (const rel of firstLoad) {
  const bytes = gzip9(join(DIST, rel));
  firstLoadTotal += bytes;
  console.log(`| ${rel} | ${bytes} |`);
}
console.log(`| **first-load total** | **${firstLoadTotal}** |`);
console.log(
  `\nFirst-load budget ${FIRST_LOAD_BUDGET.toLocaleString('en-US')} bytes → ` +
    `headroom ${(FIRST_LOAD_BUDGET - firstLoadTotal).toLocaleString('en-US')} bytes (reported, not gated).`,
);

if (existsSync(IMPORT_WASM)) {
  console.log(`\nimport-wasm gzip -9: **${gzip9(IMPORT_WASM)} bytes** (\`${IMPORT_WASM.slice(REPO.length + 1)}\`)`);
} else {
  console.log(`\nimport-wasm: not found at ${IMPORT_WASM} — skipped`);
}
