# D3 — SQLite alone vs SQLite + DuckDB-wasm for analytic queries

**Status:** go

## Question

Plan Task 6, resolving spec §17 open decision D3 ("DuckDB-wasm alongside
SQLite, or SQLite alone") against the §12.8 performance budget ("2×2 table
on one million rows under 1 s in the browser"), and exercising Review
Focus 5: on a million-row line list, do the three everyday analytic query
shapes stay under the budget on SQLite alone — and does an
out-of-memory-scale dataset (50M rows) get recorded as an error
measurement while later queries still run?

## Method

- `spikes/query-bench/gen-linelist.mjs` generates a deterministic line
  list (seed 42, ChaCha20 keystream RNG implemented in-file — no
  `Math.random`): `onset_date` uniform over 26 weeks starting Monday
  2026-01-05, `age` 0–89, `sex` M/F, `district` one of 50 values,
  `exposed` 0/1 (p=0.5), `ill` 0/1 (p=0.2). It materializes the whole
  dataset as one CSV string — the same text both engines ingest —
  so `?rows=50000000` overflows V8's maximum string length
  (~536,870,888 chars) and throws a catchable `RangeError` rather than
  stalling or crashing the renderer.
- `spikes/query-bench/bench.html` runs, per engine, 5 timed repetitions
  of three queries: `weekly_epi_curve` (group counts by week of
  `onset_date`), `crosstab_exposed_ill` (the §12.8 2×2), and
  `district_under5_by_sex` (`district IN (…10 values) AND age < 5`,
  grouped by sex). Each query runs in its own try/catch; engine load is
  lazy and its failure is re-thrown to every subsequent query of that
  engine, so an OOM is recorded per row and the harness continues.
- Engines: `@sqlite.org/sqlite-wasm` (`oo1.DB`, in-memory database,
  parameterized inserts in one transaction; week via
  `strftime('%W', onset_date)` on the ISO text column) and
  `@duckdb/duckdb-wasm` (browser `eh` bundle on Chrome, Web Worker +
  `registerFileText` + `COPY`; `onset_date` loaded as `DATE`, week via
  `strftime(onset_date, '%W')`). `bundle_gz_bytes` is measured in-page:
  every asset the engine fetched, gzipped via `CompressionStream`.
- `spikes/query-bench/bench.spec.mjs` (Playwright, `channel: 'chrome'`)
  serves the repo over loopback HTTP, asserts 2 engines × 3 queries =
  6 rows where each row has `median_ms` or `error`, writes
  `spikes/query-bench/results.json`, and a second test loads
  `bench.html?rows=50000000` asserting the affected rows carry `error`
  while every later query still produced a row.
- Dependencies install from the committed `package-lock.json` with
  `npm ci` inside `spikes/query-bench` (all 51 tarballs integrity-pinned).

## Environment

- Local: Ubuntu resolute x86-64 (kernel 7.0.0-1015-oem), Node.js
  v24.22.0, Google Chrome 151.0.7922.137 (stable channel — no Playwright
  browser download), Playwright 1.64.1.
- CI: `ubuntu-latest`, `actions/setup-node` `node-version: '22'` and the
  runner image's preinstalled `google-chrome-stable`; versions printed by
  the "Environment versions" step of `.github/workflows/spike-query-bench.yml`.
- Packages (from `spikes/query-bench/package-lock.json`):
  `@sqlite.org/sqlite-wasm` 3.54.1-build1 (SQLite 3.54.1),
  `@duckdb/duckdb-wasm` 1.33.1-dev57.0, `apache-arrow` 17.0.0,
  `flatbuffers` 24.12.23, `tslib` 2.8.1, `@playwright/test` 1.64.1.
- RNG: ChaCha20, seed 42 (spec §7.4), recorded here and reprinted by the
  workflow's determinism step.

## Measurements

Local run, exact command: `cd spikes/query-bench && npx playwright test`
(2 tests, both passing: `six rows` 19.7 s, `records OOM as error` 56.7 s);
numbers from the `results.json` it writes. CI run link is added to this
section once the `spike-query-bench` workflow runs on this PR.

- Median of 5 timed repetitions, 1,000,000 rows (local Chrome 151):

  | query | sqlite median_ms | duckdb median_ms |
  |---|---|---|
  | weekly_epi_curve | 789.5 | 88.7 |
  | crosstab_exposed_ill | 394.5 | 12.7 |
  | district_under5_by_sex | 67.6 | 40.6 |

- Engine download weight (`bundle_gz_bytes`, measured in-page with
  `CompressionStream('gzip')`, same run): sqlite 586,888 bytes
  (`index.mjs` + `sqlite3.wasm`); duckdb 8,326,520 bytes
  (`duckdb-browser.mjs` + `duckdb-eh.wasm` + `duckdb-browser-eh.worker.js`)
  — ~8.0 MB gz, by itself over the spec §12.8 5 MB first-load budget and
  therefore usable only as a lazily loaded analytic add-on.
- OOM measurement (same run, `bench.html?rows=50000000`): all 6 rows
  record `error: "Invalid string length"` — the requested dataset cannot
  even be materialized as text in a browser tab — and every later query
  still produced a result row (the spec test asserts each post-failure
  row exists with `median_ms` or `error` populated; run time 56.7 s, no
  renderer crash, no hang).
- Generator determinism, command:
  `node spikes/query-bench/gen-linelist.mjs | sha256sum` →
  `0ce9b3dff7f1a9e2f671790d66204962f4bc29a9a1c728fb3d12888d33edc6d1`
  (1,000,000 data rows, ~31.9 MB CSV, seed 42; identical on every run —
  the CI determinism step reprints this value).

## Decision

Applying the plan's Step 5 rule verbatim — *"if every SQLite median
≤ 1000 ms, D3 = SQLite alone; otherwise SQLite for storage plus
DuckDB-wasm for analytic queries, noting the added bundle size"* — to the
measured medians (789.5 / 394.5 / 67.6 ms, all ≤ 1000 ms):

**D3 = SQLite alone.**

## Consequences

- The spec §12.8 budget is met by SQLite alone on the three everyday
  analytic shapes, with the worst query (weekly epi-curve via per-row
  `strftime`) at ~0.79 s of the 1 s budget — headroom exists but is not
  large; a slower reference device (D7, Task 8) may not meet it, which is
  why that task measures on the real device.
- DuckDB-wasm is ~8.9× faster on the crosstab and ~8.8× on the weekly
  curve, but adds ~7.7 MB gz over sqlite-wasm — dead weight under the 5 MB
  first-load budget unless lazy-loaded, and unnecessary while SQLite
  meets the budget.
- A 50M-row dataset exceeds what a browser tab can hold as serialized
  text; per-query error recording keeps the harness alive and reporting
  (the pattern the app needs for low-end devices, Review Focus 5).
- The query benchmark harness pattern (seeded CSV + per-engine try/catch
  + `results.json`) is reusable for Task 8's device run if wanted.

## Licences

Read from each package's `package.json`/`LICENSE` (all installed via the
integrity-pinned `package-lock.json`):

- `@duckdb/duckdb-wasm` 1.33.1-dev57.0 — MIT.
- `@sqlite.org/sqlite-wasm` 3.54.1-build1 — Apache-2.0 (the npm wrapper);
  the bundled `sqlite3` C library is public-domain/blessing and the
  Emscripten glue is MIT / UIUC-NCSA, per the bundle's own header.
- `apache-arrow` 17.0.0 — Apache-2.0; `flatbuffers` 24.12.23 —
  Apache-2.0; `tslib` 2.8.1 — 0BSD (DuckDB's transitive deps, loaded via
  the import map for the Arrow result table).
- `@playwright/test` + `playwright`/`playwright-core` 1.64.1 —
  Apache-2.0 (test tooling only; remaining transitive dev deps are
  MIT/ISC per the lockfile `license` fields).
- No GADM data, SQLite SEE, or PowerSync used (spec decisions A4/A6/A7).
