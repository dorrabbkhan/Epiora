// Playwright checks for the D3 query-engine spike (plan v0.1 Task 6).
//
//   npx playwright test spikes/query-bench        (from the repo root)
//
// Serves the repo root over a loopback HTTP server so bench.html can fetch
// the wasm assets straight out of node_modules (repo root locally,
// spikes/query-bench/node_modules after `npm ci` in CI). Runs in the
// system Chrome channel so no Playwright browser download is needed.
import { test, expect } from '@playwright/test';
import http from 'node:http';
import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..');
const MIME = {
  '.html': 'text/html', '.mjs': 'text/javascript', '.js': 'text/javascript',
  '.cjs': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json',
  '.csv': 'text/csv', '.map': 'application/json',
};

// Resolve against repo root first, then the spike dir (its own node_modules in CI).
function serveFile(resolvedRoots, urlPath, res) {
  (async () => {
    for (const root of resolvedRoots) {
      const file = path.join(root, urlPath);
      if (!file.startsWith(root)) continue;
      try {
        const data = await fs.readFile(file);
        res.writeHead(200, { 'content-type': MIME[path.extname(file)] ?? 'application/octet-stream' });
        res.end(data);
        return;
      } catch { /* try next root */ }
    }
    res.writeHead(404).end('not found: ' + urlPath);
  })();
}

const ROOTS = [ROOT, HERE];

test.use({ channel: 'chrome', viewport: { width: 800, height: 600 } });
test.describe.configure({ timeout: 180_000 });

let server, baseURL;
test.beforeAll(async () => {
  server = http.createServer((req, res) =>
    serveFile(ROOTS, decodeURIComponent(new URL(req.url, 'http://x').pathname), res));
  await new Promise((r) => server.listen(0, '127.0.0.1', r));
  baseURL = `http://127.0.0.1:${server.address().port}`;
});
test.afterAll(() => server?.close());

test('six rows: 2 engines x 3 queries on 1,000,000 rows', async ({ page }) => {
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  page.on('console', (m) => { if (m.type() === 'error') errors.push(m.text()); });

  await page.goto(`${baseURL}/spikes/query-bench/bench.html`);
  await page.waitForFunction(() => window.__results !== undefined, null, { timeout: 170_000 });
  const rows = await page.evaluate(() => window.__results);
  expect(errors, 'page errors during bench run').toEqual([]);

  expect(rows).toHaveLength(6);
  const engines = [...new Set(rows.map((r) => r.engine))].sort();
  expect(engines).toEqual(['duckdb', 'sqlite']);
  const queries = ['weekly_epi_curve', 'crosstab_exposed_ill', 'district_under5_by_sex'];
  for (const engine of ['sqlite', 'duckdb']) {
    for (const q of queries) {
      expect(rows.filter((r) => r.engine === engine && r.query === q),
        `${engine}/${q} present`).toHaveLength(1);
    }
  }
  for (const r of rows) {
    expect(r.rows).toBe(1_000_000);
    expect(typeof r.bundle_gz_bytes).toBe('number');
    expect(r.bundle_gz_bytes).toBeGreaterThan(0);
    expect(r.median_ms !== null || r.error !== null,
      `${r.engine}/${r.query} has median_ms or error`).toBe(true);
  }
  await fs.writeFile(path.join(HERE, 'results.json'), JSON.stringify(rows, null, 2) + '\n');
});

test('records OOM as error', async ({ page }) => {
  // rows=50000000 makes each engine ingest the seeded rows in bounded
  // chunks. sqlite-wasm runs with a 256 MiB hard heap limit, so its own
  // allocator returns SQLITE_NOMEM mid-ingest — an allocation failure
  // inside the engine, not a limit of the harness. DuckDB-wasm ingests
  // the same 50M rows successfully, so its queries still run afterwards.
  test.setTimeout(600_000);
  await page.goto(`${baseURL}/spikes/query-bench/bench.html?rows=50000000`);
  await page.waitForFunction(() => window.__results !== undefined, null, { timeout: 570_000 });
  const rows = await page.evaluate(() => window.__results);
  // The failure and the later successful queries are a measurement: print
  // the rows so the run log keeps them (results.json is the 1M case only).
  console.log('OOM rows: ' + JSON.stringify(rows));

  expect(rows).toHaveLength(6);
  const affected = rows.filter((r) => r.error !== null);
  expect(affected.length).toBeGreaterThan(0);
  for (const r of affected) {
    // The recorded error names the engine's out-of-memory failure.
    expect(r.error, `${r.engine}/${r.query} error names the failure`)
      .toMatch(/out of memory|memory|SQLITE_NOMEM/i);
  }
  // Later queries still ran: at least one query AFTER the first failed
  // row actually executed and produced a median, i.e. the harness kept
  // going past the engine failure instead of re-throwing a cached error.
  const firstError = rows.findIndex((r) => r.error !== null);
  const later = rows.slice(firstError + 1);
  expect(later.some((r) => r.median_ms !== null),
    'a query after the first failed row executed (median_ms set)').toBe(true);
  for (const r of later) {
    expect(r.median_ms !== null || r.error !== null).toBe(true);
  }
  expect(rows.at(-1), 'last query row exists').toBeTruthy();
});
