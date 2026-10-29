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

Canonical run: CI workflow `spike-query-bench`, run
[40883881690](https://github.com/dorrabbkhan/Epiora/actions/runs/40883881690)
(`ubuntu-latest`, Node.js v22.23.4, Google Chrome 156.0.8078.47,
Playwright 1.64.1 — versions printed in the "Environment versions" step;
"2 passed (1.1m)" in the "Playwright benchmark" step; `results.json` in
the "Report results.json" step and the `query-bench-results` artifact).

- Median of 5 timed repetitions, 1,000,000 rows (run 40883881690):

  | query | sqlite median_ms | duckdb median_ms |
  |---|---|---|
  | weekly_epi_curve | **1060.4** | 75.3 |
  | crosstab_exposed_ill | 655.1 | 9.9 |
  | district_under5_by_sex | 123.9 | 34.8 |

- Local run for comparison, exact command:
  `cd spikes/query-bench && npx playwright test` on the dev machine
  (Ubuntu resolute, Chrome 151.0.7922.137): sqlite 789.5 / 394.5 / 67.6
  ms, duckdb 88.7 / 12.7 / 40.6 ms — the dev host is faster than the CI
  runner; the weekly-curve median sits either side of the 1000 ms budget
  depending on machine class.
- Engine download weight (`bundle_gz_bytes`, measured in-page with
  `CompressionStream('gzip')`, identical on both runs): sqlite 586,888
  bytes (`index.mjs` + `sqlite3.wasm`); duckdb 8,326,520 bytes
  (`duckdb-browser.mjs` + `duckdb-eh.wasm` + `duckdb-browser-eh.worker.js`)
  — ~8.0 MB gz, by itself over the spec §12.8 5 MB first-load budget and
  therefore usable only as a lazily loaded analytic add-on.
- OOM measurement (run 40883881690, `records OOM as error` test,
  `bench.html?rows=50000000`): all 6 rows record
  `error: "Invalid string length"` — the dataset cannot be materialized
  as text in a browser tab — and every later query still produced a
  result row; no renderer crash, no hang.
- A failed run is also a measurement: run
  [40883237792](https://github.com/dorrabbkhan/Epiora/actions/runs/40883237792)
  (the pre-chunking version) recorded `page.waitForFunction: Target
  crashed` on the OOM case — serializing row-by-row with `csv += line`
  allocated ~1 GB of cons-string nodes before V8's length limit could
  throw, and Chrome killed the renderer. Chunked serialization
  (`gen-linelist.mjs`, 250k-row slices) lowered the live allocation to
  ~the serialized bytes; run 40883881690 then recorded the error rows
  cleanly.
- Generator determinism, command:
  `node spikes/query-bench/gen-linelist.mjs | sha256sum` →
  `0ce9b3dff7f1a9e2f671790d66204962f4bc29a9a1c728fb3d12888d33edc6d1`
  (1,000,000 data rows, ~31.9 MB CSV, seed 42) — identical locally and in
  run 40883881690's determinism step.
- `spike-reports` check (run
  [40883881855](https://github.com/dorrabbkhan/Epiora/actions/runs/40883881855))
  green: `check-spikes.sh` validates this report.

## Decision

Applying the plan's Step 5 rule verbatim — *"if every SQLite median
≤ 1000 ms, D3 = SQLite alone; otherwise SQLite for storage plus
DuckDB-wasm for analytic queries, noting the added bundle size"* — to the
recorded run's medians (1060.4 / 655.1 / 123.9 ms; the weekly epi-curve
median exceeds 1000 ms):

**D3 = SQLite for storage plus DuckDB-wasm for analytic queries.**

The added bundle size is ~8.0 MB gz (8,326,520 bytes over sqlite-wasm's
586,888), so DuckDB must load lazily and is excluded from the §12.8
first-load budget only by loading on demand — a real cost: SQLite alone
was under budget on a fast dev machine but not on the CI runner, and the
D7 reference device will be slower still, so analytic queries cannot rely
on SQLite alone meeting 1 s.

## Consequences

- SQLite alone does **not** reliably meet the §12.8 budget: the weekly
  epi-curve (a per-row `strftime` + GROUP BY over 1M rows) measured
  1060 ms on the CI runner — the decision is not marginal in the safe
  direction, since the D7 reference device and field phones will be
  slower than `ubuntu-latest`, not faster.
- `epiora-app` therefore keeps SQLite as the on-device store (spec §7.1)
  and lazy-loads DuckDB-wasm (~8.0 MB gz, fetched once and cached by the
  PWA) for analytic queries: 66× faster on the 2×2 crosstab, 14× on the
  weekly curve. The first-load bundle is unaffected if the load is
  deferred until the Analysis workspace opens.
- A 50M-row dataset exceeds what a browser tab can materialize as
  serialized text; per-query error recording keeps the harness alive and
  reporting (the pattern the app needs for low-end devices, Review
  Focus 5 — demonstrated by failed run 40883237792 before and green run
  40883881690 after chunking the serialization).
- The benchmark harness pattern (seeded CSV + per-engine try/catch +
  `results.json`) is reusable for Task 8's device run if wanted.

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
