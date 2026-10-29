# D7 — Reference device and WebR on low-end Android

**Status:** go

## Question

Plan Task 8, resolving spec §17 open decision D7 ("Low-end Android
reference device") and feeding spec §9's fallback ("if [WebR download
size and start-up on low-end Android are] unacceptable, advanced methods
are hosted-only on mobile"): how large is the WebR download, how long
does it take to become ready on a low-end device, and does
`fisher.test(matrix(c(43,3,11,18), 2))` in WebR reproduce the R fixture
p-value?

## Method

- Reference device (plan Step 1, human step): **pending the owner.**
  Until the owner names a ≤ 4 GB RAM Android phone with Chrome, all
  measurements use the plan's interim proxy, recorded verbatim:
  *"Pixel emulator + Chrome DevTools 4× CPU throttle and Slow 4G"*
  (run as desktop Chromium with the throttling applied — what plan
  Step 2 specifies for the check).
- `spikes/webr-mobile/index.html` loads WebR from the npm package
  `webr@0.6.0` (its `browser` export `dist/webr.js`; `dist/webr.mjs` is
  the Node build and fails in Chrome on `import 'module'`), pinned and
  integrity-locked by `package-lock.json` rather than fetched from a
  CDN, then evaluates `fisher.test(matrix(c(43,3,11,18), 2))$p.value`
  and sets `window.__webr = { transfer_bytes, ready_ms, fisher_ms,
  fisher_p }`.
- `spikes/webr-mobile/measure.mjs` serves the spike over loopback HTTP
  with COOP/COEP headers (SharedArrayBuffer channel) and gzip content
  encoding (CDN-like wire bytes), drives Chrome (`channel: 'chrome'`)
  via Playwright, and applies `Emulation.setCPUThrottlingRate {rate:4}`
  plus `Network.emulateNetworkConditions` with the DevTools "Slow 4G"
  preset values it sends to the browser — latency 562.5 ms (150 ms
  target × 3.75), download 180,000 B/s, upload 84,375 B/s
  (devtools-frontend `Slow4GConditions`, formerly "Fast 3G",
  crbug.com/342406608). It measures a cold load (fresh context) and a
  warm load (same context, HTTP cache warm), then asserts `fisher_p`
  equals `fixtures/2x2/oswego_vanilla.json` →
  `expected.fisher_exact_test.p_value` within the fixture's `tolerance`
  (1e-9) and exits non-zero otherwise. `--cdp <endpoint>` attaches to a
  device over Chrome remote debugging for the (human) real-device run,
  with no synthetic throttling; there the cold load first issues
  `Network.clearBrowserCache`.
- `transfer_bytes` is counted server-side: the page's own
  PerformanceResourceTiming never sees the worker-side fetches (R.wasm,
  vfs data) — recorded in `__webr_meta.transfer_bytes_page_scope`
  (~60 KB vs ~13 MB wire). The server counts response bytes under
  `/node_modules/webr/` since the last document fetch; the document is
  `no-store` so each measured load resets the counter, while assets
  carry `max-age=86400` so warm loads can cache-hit.
- Definitions: `ready_ms` = module-script start → `webR.init()`
  resolved (download + instantiate + engine init); `fisher_ms` = the
  single `evalR` call.

## Environment

- Local: Ubuntu resolute x86-64 (kernel 7.0.0-1015-oem), Node.js
  v24.22.0, Google Chrome 151.0.7922.137 (headless, `channel: 'chrome'`),
  Playwright 1.64.1; page reports `hardwareConcurrency` 22,
  `deviceMemory` 32 — i.e. a fast desktop, with the proxy throttling
  doing the approximating.
- Under test: `webr` 0.6.0 (bundles R 4.6.0, per the page's
  `R.version.string`), SharedArrayBuffer channel
  (`crossOriginIsolated: true`).
- CI: `ubuntu-latest`, Node 22, runner `google-chrome-stable`; versions
  printed by the "Environment versions" step of
  `.github/workflows/spike-webr-mobile.yml`.
- RNG: none.

## Measurements

Local run — exact command `cd spikes/webr-mobile && npm ci && node
measure.mjs` (output written to `webr-results.json`, a gitignored
artifact):

- Cold load: `transfer_bytes` 13,309,580 (~12.7 MiB wire, gzip), of
  which `R.wasm` alone is 12,328,041 B gzipped (18,062,845 B raw);
  `ready_ms` 79,434.5; `fisher_ms` 13.6; `fisher_p`
  2.5974506918372895e-07 → parity PASS vs fixture (tolerance 1e-9).
- Warm load (same context, HTTP cache): `transfer_bytes` 12,336,585 —
  Chrome did **not** retain R.wasm (debug run showed it as the only
  re-requested asset, also under `--disk-cache-size=2147483648`), so a
  "warm" load re-downloads ~12.3 MB; `ready_ms` 70,197.3; `fisher_ms`
  14.4; `fisher_p` identical.
- CI run: see the `spike-webr-mobile` workflow run linked from this
  task's PR (the `webr-results.json` step summary / artifact).

## Decision

Applying the task's proposed rule — *"WebR acceptable on mobile if warm
`ready_ms` ≤ 10,000 on the reference device; otherwise advanced methods
are hosted-only on mobile (spec §9)"* — to the only environment
available so far (the interim proxy, warm `ready_ms` 70,197 ms ≫
10,000 ms):

**advanced methods are hosted-only on mobile (spec §9)** — *interim*,
pending the owner's reference-device run (OWNER GATE; plan Steps 1 and
5 are human steps). The proxy margin is ~7× over budget, and a real ≤
4 GB device is expected to be slower, not faster, so the device run is
confirmation rather than a likely flip; but the rule's letter and the
D7 device choice itself remain the owner's to close.

## Consequences

- Advanced methods on mobile route to hosted R (the §9 fallback); the
  Rust engine still covers §8.3 everyday procedures offline on phones.
- WebR must lazy-load outside the §12.8 first-load budget regardless of
  platform: its ~12.7 MiB wire payload alone is ~2.5× the 5 MB budget.
- Plain HTTP caching does not protect the payload — every navigation
  re-downloads R.wasm (~12.3 MB) because Chrome does not retain the
  streamed wasm response, so "warm" saves only ~9 s of ~79 s. If WebR
  on mobile is ever revisited, measure Cache Storage / service-worker
  pinning of the payload instead of relying on the HTTP cache.
- `fisher.test` parity with the R 4.5.2 fixture holds under WebR's
  R 4.6.0 (Δ = 0 within 1e-9) — R-version drift did not move this
  statistic, though fixture-version pinning stays the guard (Review
  Focus 4).
- Harness lessons for v0.2+ probes: in-page resource timing misses
  worker fetches — measure wire bytes at the server or via CDP; the
  npm `webr` package's `browser` export (`dist/webr.js`) is the
  browser load, not `dist/webr.mjs`.
- Pending human steps (owner): choose the D7 reference device (Step 1)
  and run `node measure.mjs --cdp http://127.0.0.1:9222` on it via
  Chrome remote debugging (Step 5; `adb forward tcp:9222
  localabstract:chrome_devtools_remote` plus `adb reverse tcp:<port>
  tcp:<port>` so the phone reaches the harness server).

## Licences

- `webr` 0.6.0: `package.json` declares `"SEE LICENSE IN LICENCE.md"`;
  the packaged `LICENSE.md` states the webR **binaries are GNU GPL v3**
  (they contain R — GPL v2 & v3 — plus PCRE2 BSD, liblzma public
  domain, libgfortran GPL v2/v3, LLVM Flang Apache-2.0, Noto fonts SIL
  OFL, jsTree and xterm.js MIT); non-binary repo material is MIT.
  GPL-3.0 ↔ AGPL-3.0 combine under their §13; the spike loads WebR as
  a fetched runtime payload, and this licence fact stays attached to
  D7 for the v0.2+ call on shipping it.
- `@playwright/test` 1.64.1: Apache-2.0 (devDependency, harness only).
- `esbuild` 0.28.2 (transitive dep of `webr`, dev tooling): MIT.
- Rejected licences respected: no GADM data, no SQLite Encryption
  Extension (SEE), no PowerSync (spec A4, A6, A7).
