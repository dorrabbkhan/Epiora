// Tests for the first-load budget gate (v0.3 plan Task 17, spec
// §10.2): `budget.mjs <dist>` sums the gzip -9 sizes of index.html,
// the entry chunk's JS + CSS from Vite's dist/.vite/manifest.json,
// the core engine .wasm and sqlite3.wasm, and exits 1 at or above
// 5,000,000 bytes.
//
// Run: `node --test app/scripts`.

import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { gzipSync } from 'node:zlib';
import assert from 'node:assert/strict';
import test from 'node:test';

const BUDGET = fileURLToPath(new URL('./budget.mjs', import.meta.url));
const BUDGET_BYTES = 5_000_000;

const gzip9 = (buf) => gzipSync(buf, { level: 9 }).length;

/** Deterministic incompressible bytes (xorshift32, seed 20261105 —
 *  same generator as perf/gen-csv.mjs). */
function incompressible(n) {
  const buf = Buffer.alloc(n);
  let x = 20261105;
  for (let i = 0; i < n; i++) {
    x ^= x << 13;
    x ^= x >>> 17;
    x ^= x << 5;
    x >>>= 0;
    buf[i] = x & 0xff;
  }
  return buf;
}

/** Writes a minimal `dist`: index.html, `.vite/manifest.json` and the
 *  given `assets/` files. `entries` is the manifest value list. */
function makeDist(t, manifest, assets) {
  const dir = mkdtempSync(join(tmpdir(), 'epiora-budget-'));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  mkdirSync(join(dir, '.vite'), { recursive: true });
  mkdirSync(join(dir, 'assets'), { recursive: true });
  writeFileSync(join(dir, 'index.html'), assets['index.html'] ?? '<!doctype html>');
  writeFileSync(join(dir, '.vite', 'manifest.json'), JSON.stringify(manifest));
  for (const [name, contents] of Object.entries(assets)) {
    if (name === 'index.html') continue;
    writeFileSync(join(dir, 'assets', name), contents);
  }
  return dir;
}

const entryManifest = {
  'index.html': {
    file: 'assets/index-aaa.js',
    name: 'index',
    src: 'index.html',
    isEntry: true,
    css: ['assets/index-bbb.css'],
  },
  'lazy.ts': {
    file: 'assets/lazy-ccc.js',
    name: 'lazy',
    src: 'lazy.ts',
    isDynamicEntry: true,
  },
};

function runBudget(dist, ...extra) {
  return spawnSync('node', [BUDGET, dist, ...extra], { encoding: 'utf8' });
}

test('sums_first_load_set_only', (t) => {
  const files = {
    'index.html':
      '<!doctype html><script type="module" src="/assets/index-aaa.js"></script>' +
      '<link rel="stylesheet" href="/assets/index-bbb.css">',
    'index-aaa.js': 'console.log("entry");\n'.repeat(50),
    'index-bbb.css': ':root { color: red; }\n'.repeat(20),
    'epiora_wasm_bg-x.wasm': 'fake-core-wasm'.repeat(100),
    'sqlite3-y.wasm': 'fake-sqlite'.repeat(100),
    // Ships in dist but is NOT part of the first-load set — sizeable
    // so the assertion would notice if it were counted.
    'epiora_import_wasm_bg-z.wasm': incompressible(400_000),
    // A non-entry lazy chunk — not first-load either.
    'lazy-ccc.js': 'lazy chunk\n'.repeat(30),
  };
  const dist = makeDist(t, entryManifest, files);
  const expected =
    gzip9(Buffer.from(files['index.html'])) +
    gzip9(Buffer.from(files['index-aaa.js'])) +
    gzip9(Buffer.from(files['index-bbb.css'])) +
    gzip9(Buffer.from(files['epiora_wasm_bg-x.wasm'])) +
    gzip9(Buffer.from(files['sqlite3-y.wasm']));

  const r = runBudget(dist);
  assert.equal(r.status, 0, r.stderr);
  const m = /\*\*first-load total\*\* \| \*\*(\d+)\*\*/.exec(r.stdout);
  assert.ok(m !== null, `total line missing from:\n${r.stdout}`);
  assert.equal(Number(m[1]), expected);
  // The import-wasm and lazy chunk names must not appear in the table.
  assert.ok(!r.stdout.includes('epiora_import_wasm_bg'));
  assert.ok(!r.stdout.includes('lazy-ccc'));
  assert.match(r.stdout, /PASS/);
});

test('fails_at_5_000_000', (t) => {
  const files = {
    'index.html': '<!doctype html>',
    'index-aaa.js': incompressible(BUDGET_BYTES), // gzip ≈ same size
    'index-bbb.css': ':root{}\n',
    'epiora_wasm_bg-x.wasm': 'core'.repeat(10),
    'sqlite3-y.wasm': 'sqlite'.repeat(10),
  };
  const dist = makeDist(t, entryManifest, files);

  const r = runBudget(dist);
  assert.equal(r.status, 1, `expected exit 1:\n${r.stdout}\n${r.stderr}`);
  assert.match(r.stdout, /FAIL/);

  // Same dist, explicit limit one byte above the true total → the
  // at-or-above rule passes it; a limit equal to the total fails.
  const probe = runBudget(dist, '999999999');
  const m = /\*\*first-load total\*\* \| \*\*(\d+)\*\*/.exec(probe.stdout);
  const total = Number(m[1]);
  assert.equal(runBudget(dist, String(total + 1)).status, 0);
  assert.equal(runBudget(dist, String(total)).status, 1);
});
