#!/usr/bin/env node
// app/perf/summarise.mjs [results.jsonl] — turns the perf spec's
// `{scenario, ...}` JSONL lines into median + p90 per numeric field
// per scenario, counts failed and skipped samples, and reports the
// gzip -9 sizes of import-wasm and of the first-load set (v0.3 plan
// Task 16, spec §10.3). Markdown on stdout; the perf workflow appends
// it to $GITHUB_STEP_SUMMARY and uploads it as an artifact. Measure
// only — nothing here gates.
//
// A line with `error` is a failed sample (counted, with its recorded
// diagnostics listed under Failures); a line with `skipped` is a
// dependent scenario that never ran. median/p90 cover successful
// samples only. A missing results.jsonl prints a clear line instead
// of throwing.
//
// Sizes come from PERF_DIST (default app/dist); PERF_COMPARE_DIST adds
// a second column so a run can show the VITE_TEST_HOOKS=1 build next
// to the plain production build it stands in for.
//
// p90 is nearest-rank (the smallest sample at or above the 90th
// percentile); median averages the two middle samples at even n.

import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { basename, join, resolve } from 'node:path';
import { gzipSync } from 'node:zlib';

const PERF_DIR = fileURLToPath(new URL('.', import.meta.url));
const APP = join(PERF_DIR, '..');
const REPO = join(APP, '..');
const RESULTS =
  process.argv[2] ?? join(REPO, 'target', 'perf', 'results.jsonl');
const DIST = resolve(process.env.PERF_DIST ?? join(APP, 'dist'));
const COMPARE_DIST =
  process.env.PERF_COMPARE_DIST === undefined
    ? null
    : resolve(process.env.PERF_COMPARE_DIST);
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

if (!existsSync(RESULTS)) {
  console.log(
    `**No \`results.jsonl\` at \`${RESULTS}\` — the perf spec produced ` +
      'no output (it never ran, or every sample died before its first ' +
      'recorded line).**\n',
  );
} else {
  const lines = readFileSync(RESULTS, 'utf8')
    .split('\n')
    .filter((l) => l.trim() !== '')
    .map((l) => JSON.parse(l));

  const ok = new Map(); // scenario -> { n, fields: Map<name, number[]> }
  const failed = new Map(); // scenario -> [{ run, error }]
  const skipped = new Map(); // scenario -> [{ run, reason }]
  const scenarioOrder = [];
  const seen = (map, scenario) => {
    if (!map.has(scenario)) {
      map.set(scenario, []);
      if (!scenarioOrder.includes(scenario)) scenarioOrder.push(scenario);
    }
    return map.get(scenario);
  };
  const okEntry = (scenario) => {
    if (!ok.has(scenario)) {
      ok.set(scenario, { n: 0, fields: new Map() });
      if (!scenarioOrder.includes(scenario)) scenarioOrder.push(scenario);
    }
    return ok.get(scenario);
  };

  for (const line of lines) {
    const { scenario, run, error, skipped: reason, ...fields } = line;
    if (typeof error === 'string') {
      seen(failed, scenario).push({ run, error, fields });
      continue;
    }
    if (reason !== undefined) {
      seen(skipped, scenario).push({ run, reason });
      continue;
    }
    const entry = okEntry(scenario);
    entry.n++;
    for (const [k, v] of Object.entries(fields)) {
      if (typeof v !== 'number') continue;
      const list = entry.fields.get(k) ?? [];
      list.push(v);
      entry.fields.set(k, list);
    }
  }

  console.log('| scenario | ok | failed | skipped | field | median | p90 |');
  console.log('|---|---|---|---|---|---|---|');
  for (const scenario of [...scenarioOrder].sort()) {
    const nOk = ok.get(scenario)?.n ?? 0;
    const nFailed = failed.get(scenario)?.length ?? 0;
    const nSkipped = skipped.get(scenario)?.length ?? 0;
    const fields = ok.get(scenario)?.fields ?? new Map();
    if (fields.size === 0) {
      console.log(`| ${scenario} | ${nOk} | ${nFailed} | ${nSkipped} | — | — | — |`);
      continue;
    }
    for (const [field, values] of [...fields.entries()].sort()) {
      const sorted = [...values].sort((a, b) => a - b);
      console.log(
        `| ${scenario} | ${nOk} | ${nFailed} | ${nSkipped} | ${field} | ` +
          `${fmt(median(sorted))} | ${fmt(pct(sorted, 0.9))} |`,
      );
    }
  }

  if (failed.size > 0) {
    console.log('\n### Failures');
    for (const [scenario, list] of [...failed.entries()].sort()) {
      for (const f of list) {
        const extra = Object.entries(f.fields)
          .map(([k, v]) => `${k}=${JSON.stringify(v)}`)
          .join(' · ');
        console.log(
          `- run ${f.run} · ${scenario} · ${f.error}${extra === '' ? '' : ` · ${extra}`}`,
        );
      }
    }
  }
  if (skipped.size > 0) {
    console.log('\n### Skips');
    for (const [scenario, list] of [...skipped.entries()].sort()) {
      for (const s of list) {
        console.log(`- run ${s.run} · ${scenario} · ${s.reason}`);
      }
    }
  }
}

// --- Sizes (gzip -9) --------------------------------------------------

const distName = (dir) => basename(dir.replace(/[\\/]+$/, ''));

function firstLoadSet(dist) {
  const indexHtml = readFileSync(join(dist, 'index.html'), 'utf8');
  const entryJs = indexHtml.match(
    /<script type="module"[^>]*src="([^"]+)"/,
  )?.[1];
  const entryCss = indexHtml.match(
    /<link rel="stylesheet"[^>]*href="([^"]+)"/,
  )?.[1];
  if (entryJs === undefined || entryCss === undefined) {
    throw new Error(`${dist}/index.html: entry script or stylesheet not found`);
  }
  const strip = (href) => href.replace(/^\//, '');
  const assets = readdirSync(join(dist, 'assets'));
  const pick = (re, what) => {
    const hit = assets.find((f) => re.test(f));
    if (hit === undefined) throw new Error(`${dist}/assets: ${what} not found`);
    return `assets/${hit}`;
  };
  return [
    ['index.html', 'index.html'],
    ['entry JS', strip(entryJs)],
    ['entry CSS', strip(entryCss)],
    ['core .wasm', pick(/^epiora_wasm_bg-.*\.wasm$/, 'core .wasm')],
    ['sqlite3.wasm', pick(/^sqlite3-.*\.wasm$/, 'sqlite3.wasm')],
  ].map(([role, rel]) => ({ role, rel, bytes: gzip9(join(dist, rel)) }));
}

console.log('\n### Sizes (gzip -9, bytes)');
if (!existsSync(join(DIST, 'index.html'))) {
  console.log(`\`${DIST}\` has no index.html — no dist to measure.`);
} else {
  const primary = firstLoadSet(DIST);
  const compare =
    COMPARE_DIST !== null && existsSync(join(COMPARE_DIST, 'index.html'))
      ? firstLoadSet(COMPARE_DIST)
      : null;
  if (compare === null) {
    console.log(`| file (${distName(DIST)}) | bytes |`);
    console.log('|---|---|');
  } else {
    console.log(
      `| file | ${distName(DIST)} | bytes | ${distName(COMPARE_DIST)} | bytes |`,
    );
    console.log('|---|---|---|---|---|');
  }
  let total = 0;
  let compareTotal = 0;
  for (const p of primary) {
    const c = compare?.find((r) => r.role === p.role) ?? null;
    total += p.bytes;
    compareTotal += c?.bytes ?? 0;
    console.log(
      compare === null
        ? `| ${p.rel} | ${p.bytes} |`
        : `| ${p.role} | \`${p.rel}\` | ${p.bytes} | \`${c?.rel ?? '—'}\` | ${c?.bytes ?? '—'} |`,
    );
  }
  console.log(
    compare === null
      ? `| **first-load total** | **${total}** |`
      : `| **first-load total** | | **${total}** | | **${compareTotal}** |`,
  );
  console.log(
    `\nFirst-load budget ${FIRST_LOAD_BUDGET.toLocaleString('en-US')} bytes → ` +
      `headroom ${(FIRST_LOAD_BUDGET - total).toLocaleString('en-US')} bytes on \`${distName(DIST)}\` (reported, not gated).`,
  );
}

if (existsSync(IMPORT_WASM)) {
  console.log(`\nimport-wasm gzip -9: **${gzip9(IMPORT_WASM)} bytes** (\`${IMPORT_WASM.slice(REPO.length + 1)}\`)`);
} else {
  console.log(`\nimport-wasm: not found at ${IMPORT_WASM} — skipped`);
}
