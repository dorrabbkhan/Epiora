// Measurement harness + parity check for the D7 spike (plan v0.1 Task 8).
//
//   node measure.mjs                    interim proxy: desktop Chromium with
//                                       4x CPU throttle and Slow 4G
//   node measure.mjs --cdp <endpoint>   reference device: attach to Android
//                                       Chrome via remote debugging, e.g.
//                                       --cdp http://127.0.0.1:9222
//                                       (no synthetic throttling on device)
//
// Loads index.html over a loopback HTTP server, waits for window.__webr =
// { transfer_bytes, ready_ms, fisher_ms, fisher_p }, records cold and warm
// (HTTP-cache) loads, and asserts fisher_p matches
// fixtures/2x2/oswego_vanilla.json within its tolerance.
//
// Slow 4G = the Chrome DevTools preset of the same name (formerly "Fast 3G",
// crbug.com/342406608): target ~150 ms RTT, ~1.6 Mbps down, ~750 Kbps up,
// which DevTools sends to the browser as latency 562.5 ms (150 x 3.75),
// download 180,000 B/s and upload 84,375 B/s. CPU throttle rate 4 matches
// the plan's "4x CPU throttle".
import { chromium } from '@playwright/test';
import http from 'node:http';
import { promises as fs } from 'node:fs';
import zlib from 'node:zlib';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const HERE = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(HERE, '..', '..');
const FIXTURE = path.join(ROOT, 'fixtures', '2x2', 'oswego_vanilla.json');
const RESULTS = path.join(HERE, 'webr-results.json');

const CPU_SLOWDOWN = 4;
const SLOW_4G = { latency: 562.5, downloadThroughput: 180_000, uploadThroughput: 84_375, offline: false };
const PAGE_TIMEOUT = 900_000;

const MIME = {
  '.html': 'text/html', '.mjs': 'text/javascript', '.js': 'text/javascript',
  '.cjs': 'text/javascript', '.wasm': 'application/wasm', '.json': 'application/json',
  '.map': 'application/json', '.so': 'application/octet-stream',
  '.data': 'application/octet-stream', '.gz': 'application/gzip',
  '.ttf': 'font/ttf', '.otf': 'font/otf', '.css': 'text/css',
};

function usage(msg) {
  if (msg) console.error(msg);
  console.error('usage: node measure.mjs [--cdp <endpointURL>]');
  process.exitCode = 2;
}

const args = process.argv.slice(2);
let cdpEndpoint = null;
for (let i = 0; i < args.length; i++) {
  if (args[i] === '--cdp') cdpEndpoint = args[++i];
  else { usage(`unknown argument: ${args[i]}`); process.exit(2); }
}

// Serve the spike dir first (its node_modules after `npm ci` in CI), then the
// repo root (local root-level installs). COOP/COEP enables the
// SharedArrayBuffer channel; assets are gzip-encoded like a real CDN so
// transfer_bytes reflects wire bytes.
const ROOTS = [HERE, ROOT];
const zipCache = new Map();
// Wire bytes served under /node_modules/webr/ since the last document load.
// The page reads this via /__wire for its transfer_bytes field: worker-side
// fetches (R.wasm, vfs) never appear in the page's resource timing.
let webrWireBytes = 0;
const server = http.createServer((req, res) => {
  (async () => {
    const urlPath = decodeURIComponent(new URL(req.url, 'http://x').pathname);
    if (urlPath === '/__wire') {
      res.writeHead(200, { 'content-type': 'application/json', 'cache-control': 'no-store' });
      res.end(JSON.stringify({ bytes: webrWireBytes }));
      return;
    }
    if (urlPath === '/index.html' || urlPath === '/') webrWireBytes = 0;
    for (const root of ROOTS) {
      const file = path.join(root, urlPath);
      if (!file.startsWith(root)) continue;
      try {
        let data = await fs.readFile(file);
        const headers = {
          'content-type': MIME[path.extname(file)] ?? 'application/octet-stream',
          'cross-origin-opener-policy': 'same-origin',
          'cross-origin-embedder-policy': 'require-corp',
          // the document itself is never cached so every measured load resets
          // the counter; the heavy webr assets stay cacheable for warm loads
          'cache-control': urlPath === '/index.html' ? 'no-store' : 'public, max-age=86400',
        };
        const gzipOk = (req.headers['accept-encoding'] ?? '').includes('gzip')
          && ['.html', '.mjs', '.js', '.wasm', '.json', '.map', '.so'].includes(path.extname(file));
        if (gzipOk) {
          data = zipCache.get(file) ?? data;
          if (!zipCache.has(file)) { data = zlib.gzipSync(data); zipCache.set(file, data); }
          headers['content-encoding'] = 'gzip';
        }
        if (file.includes(`${path.sep}node_modules${path.sep}webr${path.sep}`)) webrWireBytes += data.length;
        res.writeHead(200, headers);
        res.end(data);
        return;
      } catch { /* try next root */ }
    }
    res.writeHead(404).end('not found: ' + urlPath);
  })().catch((e) => res.writeHead(500).end(String(e)));
});
await new Promise((r) => server.listen(0, '127.0.0.1', r));
const baseURL = `http://127.0.0.1:${server.address().port}`;

const fixture = JSON.parse(await fs.readFile(FIXTURE, 'utf8'));
const expectedP = fixture.expected.fisher_exact_test.p_value;
const tolerance = fixture.tolerance;

const pageErrors = [];
async function measuredLoad(page, url) {
  const response = await page.goto(url, { waitUntil: 'domcontentloaded', timeout: PAGE_TIMEOUT });
  if (!response || response.status() !== 200)
    throw new Error(`index.html did not load (HTTP ${response?.status() ?? 'none'}) — page missing?`);
  await page.waitForFunction(
    () => window.__webr !== undefined || window.__webr_error !== undefined,
    null, { timeout: PAGE_TIMEOUT });
  const err = await page.evaluate(() => window.__webr_error ?? null);
  if (err) throw new Error(`page reported __webr_error: ${JSON.stringify(err)}`);
  return page.evaluate(() => window.__webr);
}

let browser = null;
let page = null;
const results = {
  mode: cdpEndpoint ? 'device-cdp' : 'proxy',
  url: `${baseURL}/index.html`,
  env: {},
  cold: null,
  warm: null,
  fisher_expected_p: expectedP,
  fisher_tolerance: tolerance,
  parity: null,
  error: null,
};

try {
  if (cdpEndpoint) {
    browser = await chromium.connectOverCDP(cdpEndpoint);
    const context = browser.contexts()[0] ?? await browser.newContext();
    page = context.pages()[0] ?? await context.newPage();
    page.on('pageerror', (e) => pageErrors.push(String(e)));
    const cdp = await context.newCDPSession(page);
    results.env.userAgent = await page.evaluate(() => navigator.userAgent);
    // Cold: clear the browser HTTP cache so the first load is a real download.
    await cdp.send('Network.clearBrowserCache');
    results.cold = await measuredLoad(page, results.url);
    results.warm = await measuredLoad(page, results.url);
    results.env.cdpEndpoint = cdpEndpoint;
  } else {
    browser = await chromium.launch({ channel: 'chrome' });
    const context = await browser.newContext({ viewport: { width: 800, height: 600 } });
    page = await context.newPage();
    page.on('pageerror', (e) => pageErrors.push(String(e)));
    const cdp = await context.newCDPSession(page);
    await cdp.send('Emulation.setCPUThrottlingRate', { rate: CPU_SLOWDOWN });
    await cdp.send('Network.enable');
    await cdp.send('Network.emulateNetworkConditions', SLOW_4G);
    results.env = {
      throttle: { cpuSlowdown: CPU_SLOWDOWN, ...SLOW_4G, preset: 'Chrome DevTools Slow 4G' },
      userAgent: await page.evaluate(() => navigator.userAgent),
    };
    results.cold = await measuredLoad(page, results.url);
    results.warm = await measuredLoad(page, results.url);
  }

  results.env.meta = await page.evaluate(() => window.__webr_meta ?? null);
  const p = results.cold.fisher_p;
  results.parity = Math.abs(p - expectedP) <= tolerance;
  results.pageErrors = pageErrors;
} catch (e) {
  results.error = String(e);
} finally {
  await browser?.close().catch(() => {});
  server.close();
}

await fs.writeFile(RESULTS, JSON.stringify(results, null, 2) + '\n');
console.log(JSON.stringify(results, null, 2));

if (results.error) { console.error('FAIL: ' + results.error); process.exit(1); }
if (results.parity !== true) {
  console.error(`FAIL: fisher_p ${results.cold?.fisher_p} != ${expectedP} within ${tolerance}`);
  process.exit(1);
}
if (pageErrors.length) { console.error('FAIL: page errors: ' + pageErrors.join('; ')); process.exit(1); }
console.log('PASS: fisher_p parity within ' + tolerance);
