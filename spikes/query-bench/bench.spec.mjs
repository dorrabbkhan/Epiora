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
  // rows=50000000 forces the serialized dataset past V8's maximum string
  // length (~536M chars): a real allocation failure, recorded per row.
  await page.goto(`${baseURL}/spikes/query-bench/bench.html?rows=50000000`);
  await page.waitForFunction(() => window.__results !== undefined, null, { timeout: 170_000 });
  const rows = await page.evaluate(() => window.__results);

  expect(rows).toHaveLength(6);
  const affected = rows.filter((r) => r.error !== null);
  expect(affected.length).toBeGreaterThan(0);
  // Later queries still ran: every query after the first failure still
  // produced a result row (median_ms or error), i.e. the harness continued.
  const firstError = rows.findIndex((r) => r.error !== null);
  for (const r of rows.slice(firstError + 1)) {
    expect(r.median_ms !== null || r.error !== null).toBe(true);
  }
  expect(rows.at(-1), 'last query row exists').toBeTruthy();
});
