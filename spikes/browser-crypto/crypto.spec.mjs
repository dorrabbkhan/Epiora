// Playwright checks for the D6 browser-encryption spike (plan v0.1 Task 7).
//
//   npx playwright test spikes/browser-crypto        (from the repo root)
//
// Serves the repo root over a loopback HTTP server so page.html can fetch
// the vendored sqlcipher-wasm module and @sqlite.org/sqlite-wasm straight
// out of node_modules (repo root locally, spikes/browser-crypto/node_modules
// after `npm ci` in CI). Runs in the system Chrome channel so no Playwright
// browser download is needed.
import { test, expect } from '@playwright/test';
import http from 'node:http';
import { promises as fs } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..');
const MIME = {
  '.html': 'text/html', '.mjs': 'text/javascript', '.js': 'text/javascript',
  '.cjs': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json',
};
const OPFS_DB = 'epiora-d6.db';
const PROBE_KEY = 'epiora-d6-probe-key';
const MARKER = 'EPIORA_PLAINTEXT_MARKER';

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

async function openPage(page, query = '') {
  const errors = [];
  page.on('pageerror', (e) => errors.push(String(e)));
  await page.goto(`${baseURL}/spikes/browser-crypto/page.html${query}`);
  await page.waitForFunction(() => window.__ready === true, null, { timeout: 120_000 });
  return errors;
}

test('field round trip', async ({ page }) => {
  const errors = await openPage(page);
  const roundTrip = await page.evaluate(async () =>
    await window.fieldCrypto.decrypt('pw', await window.fieldCrypto.encrypt('pw', 'Jane Doe')));
  expect(roundTrip).toBe('Jane Doe');
  expect(errors, 'page errors during field round trip').toEqual([]);
});

test('wrong passphrase rejects', async ({ page }) => {
  await openPage(page);
  const blob = await page.evaluate(() => window.fieldCrypto.encrypt('pw', 'Jane Doe'));
  await expect(page.evaluate((b) => window.fieldCrypto.decrypt('pw2', b), blob))
    .rejects.toThrow();
});

test('encrypted db hides plaintext', async ({ page }) => {
  await openPage(page);
  const probe = await page.evaluate(() => window.dbCrypto.probe());
  expect(probe.opfs).toBe(true);
  expect(probe.error).toBeNull();
  expect(probe.encrypted).toBe(true);

  const check = await page.evaluate(async ({ OPFS_DB, MARKER }) => {
    const dir = await navigator.storage.getDirectory();
    const file = await (await dir.getFileHandle(OPFS_DB)).getFile();
    const bytes = new Uint8Array(await file.arrayBuffer());
    const marker = new TextEncoder().encode(MARKER);
    let found = false;
    outer: for (let i = 0; i + marker.length <= bytes.length; i++) {
      for (let j = 0; j < marker.length; j++) if (bytes[i + j] !== marker[j]) continue outer;
      found = true; break;
    }
    const header = new TextDecoder().decode(bytes.slice(0, 16));
    return { size: bytes.length, found, header };
  }, { OPFS_DB, MARKER });

  expect(check.size).toBeGreaterThan(0);
  expect(check.found, `raw OPFS bytes of ${OPFS_DB} contain ${MARKER}`).toBe(false);
  expect(check.header).not.toBe('SQLite format 3\0');
});

test('reopen with wrong key fails', async ({ page }) => {
  await openPage(page);
  const probe = await page.evaluate(() => window.dbCrypto.probe());
  expect(probe.opfs).toBe(true);

  const wrong = await page.evaluate(() => window.dbCrypto.reopen('epiora-d6-wrong-key'));
  expect(wrong.ok).toBe(false);
  expect(typeof wrong.error).toBe('string');
  expect(wrong.error.length).toBeGreaterThan(0);

  const right = await page.evaluate((key) => window.dbCrypto.reopen(key), PROBE_KEY);
  expect(right.ok).toBe(true);
  expect(right.rows).toEqual([MARKER]);
});

test('no OPFS is reported, not thrown', async ({ page }) => {
  // Stub OPFS out before any page script runs — the deterministic stand-in
  // for a browser without OPFS (Firefox private window, older Safari).
  await page.addInitScript(() => {
    Object.defineProperty(StorageManager.prototype, 'getDirectory',
      { value: undefined, configurable: true });
  });
  const errors = await openPage(page);
  const probe = await page.evaluate(() => window.dbCrypto.probe());
  expect(probe.opfs).toBe(false);
  expect(typeof probe.error).toBe('string');
  expect(probe.error.length).toBeGreaterThan(0);
  expect(errors, 'probe threw instead of reporting').toEqual([]);
});

test('measurements for the report', async ({ page }) => {
  // ~1,000 field encrypts + ~1,000 decrypts at 600k PBKDF2 iterations each,
  // plus a db write-throughput comparison; several minutes in the worst case.
  test.setTimeout(1_200_000);
  const errors = await openPage(page, '?bench=1');
  await page.waitForFunction(() => window.__measure !== undefined, null, { timeout: 1_100_000 });
  const m = await page.evaluate(() => window.__measure);
  console.log('measurements: ' + JSON.stringify(m));
  expect(errors, 'page errors during measurements').toEqual([]);

  expect(m.field.count).toBeGreaterThanOrEqual(1000);
  expect(m.field.encrypt_total_ms).toBeGreaterThan(0);
  expect(m.field.decrypt_total_ms).toBeGreaterThan(0);
  expect(m.db.rows).toBeGreaterThan(0);
  for (const engine of ['sqlite', 'sqlcipher']) {
    const row = m.db.engines.filter((r) => r.engine === engine);
    expect(row, `${engine} benchmark row present`).toHaveLength(1);
    expect(row[0].median_ms !== null || row[0].error !== null,
      `${engine} has median_ms or error`).toBe(true);
  }
  expect(m.sqlcipher_mjs_bytes).toBeGreaterThan(0);
  expect(m.sqlcipher_mjs_gz_bytes).toBeGreaterThan(0);
  await fs.writeFile(path.join(HERE, 'crypto-results.json'), JSON.stringify(m, null, 2) + '\n');
});
