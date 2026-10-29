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
  `exposed` 0/1 (p=0.5), `ill` 0/1 (p=0.2). `generateRows()` yields the
  rows one at a time; `generateLinelistCSV()` still materializes the
  whole dataset as one CSV string for the CLI determinism check only.
- `spikes/query-bench/bench.html` runs, per engine, 5 timed repetitions
  of three queries: `weekly_epi_curve` (group counts by week of
  `onset_date`), `crosstab_exposed_ill` (the §12.8 2×2), and
  `district_under5_by_sex` (`district IN (…10 values) AND age < 5`,
  grouped by sex). Each query runs in its own try/catch; engine load is
  lazy and its failure is re-thrown to every subsequent query of that
  engine, so an engine failure is recorded per row and the harness
  continues with the next query and the next engine.
- Engines ingest the same generated rows in bounded chunks — the harness
  never holds the dataset as one JS value, so an out-of-memory failure
  can only come from inside an engine:
  - `@sqlite.org/sqlite-wasm` (`oo1.DB`, in-memory database) ingests
    8,192-row `INSERT … VALUES` batches in one transaction under
    `PRAGMA hard_heap_limit=268435456` (256 MiB). Past the cap SQLite's
    own allocator returns `SQLITE_NOMEM`, so `?rows=50000000` exhausts
    the engine's heap mid-ingest on any machine — the mechanism a
    low-end device's memory ceiling imposes, made deterministic. The
    1,000,000-row table is ~50 MB, far below the cap. Week via
    `strftime('%W', onset_date)` on the ISO text column.
  - `@duckdb/duckdb-wasm` (browser `eh` bundle on Chrome, Web Worker)
    ingests 1,000,000-row chunks via `registerFileText` + `COPY` +
    `dropFile`; `onset_date` loaded as `DATE`, week via
    `strftime(onset_date, '%W')`.
  - `bundle_gz_bytes` is measured in-page: every asset the engine
    fetched, gzipped via `CompressionStream`.
- `spikes/query-bench/bench.spec.mjs` (Playwright, `channel: 'chrome'`)
  serves the repo over loopback HTTP, asserts 2 engines × 3 queries =
  6 rows where each row has `median_ms` or `error`, writes
  `spikes/query-bench/results.json`, and a second test loads
  `bench.html?rows=50000000`, asserting the affected rows carry an
  `error` naming the engine's out-of-memory failure and that at least
  one row after the first failed row has `median_ms` — a later query
  actually executed, not just a row re-throwing a cached error.
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

Every successful CI run of workflow `spike-query-bench` on the measured
code is listed below (`ubuntu-latest`; versions printed in each run's
"Environment versions" step; `results.json` in the "Report results.json"
step and the `query-bench-results` artifact). Runs 40883881690,
40884469336 and 40884720733 executed identical spike code — commits
30120cb..eddc900 changed docs only; runs 40889431190 and 40889903879
are the fix-round heads (chunked ingest — same queries, seed and reps;
ingest is not timed). Each number is the median of 5 timed repetitions
on 1,000,000 rows, in ms.

| CI run | head | sqlite weekly | sqlite 2×2 | sqlite district | duckdb weekly | duckdb 2×2 | duckdb district |
|---|---|---|---|---|---|---|---|
| [40883881690](https://github.com/dorrabbkhan/Epiora/actions/runs/40883881690) | 30120cb | **1060.4** | 655.1 | 123.9 | 75.3 | 9.9 | 34.8 |
| [40884469336](https://github.com/dorrabbkhan/Epiora/actions/runs/40884469336) | 58461ee | **1065.7** | 535.2 | 98.0 | 57.2 | 8.7 | 29.0 |
| [40884720733](https://github.com/dorrabbkhan/Epiora/actions/runs/40884720733) | eddc900 | 741.6 | 511.5 | 100.3 | 60.4 | 7.5 | 27.1 |
| [40889431190](https://github.com/dorrabbkhan/Epiora/actions/runs/40889431190) | bc634f4 | 926.0 | 370.4 | 69.9 | 43.8 | 7.1 | 23.4 |
| [40889903879](https://github.com/dorrabbkhan/Epiora/actions/runs/40889903879) | 079f731 | 981.0 | 627.6 | 120.1 | 77.4 | 10.5 | 34.0 |

(The workflow's `pull_request` path filter evaluates the whole PR diff,
so each push — including the docs-only commit carrying this table —
re-triggers a run on identical spike code; the newest run is always in
the PR checks list.)

- Local runs on the dev machine (Ubuntu resolute, Chrome 151.0.7922.137),
  exact command `cd spikes/query-bench && npx playwright test`:
  sqlite 789.5 / 394.5 / 67.6 ms, duckdb 88.7 / 12.7 / 40.6 ms; and a
  second run by the review session via `./scripts/check.sh`, recorded in
  [this PR comment](https://github.com/dorrabbkhan/Epiora/pull/8#issuecomment-6498567363):
  sqlite 855.5 / 383.9 / 65.6 ms (duckdb medians not recorded there).
  The fix-round code (chunked ingest, same queries/seed/reps), command
  `cd spikes/query-bench && npx playwright test -g "six rows"`: sqlite
  1042.0 / 369.1 / 65.6 ms, duckdb 43.7 / 9.9 / 39.3 ms.
- **The sqlite weekly-epi-curve median falls on both sides of the
  1000 ms budget across runs, including on identical spike code**:
  above it in CI runs 40883881690 (1060.4) and 40884469336 (1065.7)
  and in the fix-round local run (1042.0), below it in CI runs
  40884720733 (741.6), 40889431190 (926.0) and 40889903879 (981.0) and
  the earlier local runs (789.5, 855.5). The other five medians are
  under budget in
  every recorded run. Which run(s) the plan's Step-5 rule applies to
  is escalated to the owner by the conductor; the Decision section
  below is unchanged pending that decision.
- Engine download weight (`bundle_gz_bytes`, measured in-page with
  `CompressionStream('gzip')`, identical on every run): sqlite 586,888
  bytes (`index.mjs` + `sqlite3.wasm`); duckdb 8,326,520 bytes
  (`duckdb-browser.mjs` + `duckdb-eh.wasm` + `duckdb-browser-eh.worker.js`)
  — ~8.0 MB gz, by itself over the spec §12.8 5 MB first-load budget and
  therefore usable only as a lazily loaded analytic add-on.
- OOM measurement (`records OOM as error` test,
  `bench.html?rows=50000000`, chunked ingest): all three sqlite rows
  record `error: "SQLITE_NOMEM: sqlite3 result code 7: out of memory"` —
  the engine's own allocator failing under its 256 MiB hard heap limit
  while ingesting row batches — while duckdb ingested the same
  50,000,000 rows and executed all three queries (`OOM rows:` line in
  the "Playwright benchmark" step). CI runs
  [40889431190](https://github.com/dorrabbkhan/Epiora/actions/runs/40889431190)
  and [40889903879](https://github.com/dorrabbkhan/Epiora/actions/runs/40889903879):
  duckdb medians on the 50M-row table 1900.7 / 213.8 / 811.5 ms and
  3366.7 / 323.2 / 1380.8 ms respectively. Local run, exact command
  `cd spikes/query-bench && npx playwright test -g "records OOM"`
  (1.6 min, `OOM rows:` line in the test output): duckdb medians
  2005.7 / 216.3 / 796.8 ms; no renderer crash, no hang.
- A failed run is also a measurement: run
  [40883237792](https://github.com/dorrabbkhan/Epiora/actions/runs/40883237792)
  (an earlier harness that materialized the dataset as one growing
  string) recorded `page.waitForFunction: Target crashed` on the OOM
  case — serializing ~50M rows with `csv += line` allocated ~1 GB of
  cons-string nodes before V8's length limit could throw, and Chrome
  killed the renderer.
- Generator determinism, command:
  `node spikes/query-bench/gen-linelist.mjs | sha256sum` →
  `0ce9b3dff7f1a9e2f671790d66204962f4bc29a9a1c728fb3d12888d33edc6d1`
  (1,000,000 data rows, ~31.9 MB CSV, seed 42) — identical locally and
  in every CI run's determinism step.
- `spike-reports` check green on every push of this branch (e.g. run
  [40889431192](https://github.com/dorrabbkhan/Epiora/actions/runs/40889431192)):
  `check-spikes.sh` validates this report.

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

- The sqlite weekly epi-curve (a per-row `strftime` + GROUP BY over 1M
  rows) measured on **both** sides of the 1000 ms budget across runs,
  including on identical spike code — above it in two CI runs and one
  local run, below it in three CI runs and two local runs — so whether
  SQLite alone meets §12.8 depends on which run(s) the plan's rule
  applies to (escalated to the owner). The D7 reference device and
  field phones will be slower than `ubuntu-latest`, not faster.
- `epiora-app` therefore keeps SQLite as the on-device store (spec §7.1)
  and lazy-loads DuckDB-wasm (~8.0 MB gz, fetched once and cached by the
  PWA) for analytic queries: across the recorded CI runs, ~12–19× faster
  than sqlite on the weekly curve and ~61–68× on the 2×2 crosstab. The
  first-load bundle is
  unaffected if the load is deferred until the Analysis workspace opens.
- An engine hitting its memory ceiling is recorded, not fatal: under
  its 256 MiB heap cap sqlite-wasm fails mid-ingest with
  `SQLITE_NOMEM`, every affected row records the engine's own error
  message, and the harness continues — duckdb then ingests the same
  50M rows and answers its queries (the pattern the app needs for
  low-end devices, Review Focus 5 — demonstrated by crashed run
  40883237792 before and green runs after).
- The benchmark harness pattern (seeded generator + per-engine
  try/catch + `results.json`) is reusable for Task 8's device run if
  wanted.

## Licences

Read from each package's `package.json`/`LICENSE` in the installed tree
(all installed via the integrity-pinned `package-lock.json`):

- `@duckdb/duckdb-wasm` 1.33.1-dev57.0 — MIT.
- `@sqlite.org/sqlite-wasm` 3.54.1-build1 — Apache-2.0 (the npm wrapper);
  the bundled `sqlite3` C library is public-domain/blessing and the
  Emscripten glue is MIT / UIUC-NCSA, per the bundle's own header.
- `apache-arrow` 17.0.0 — Apache-2.0; `flatbuffers` 24.12.23 —
  Apache-2.0; `tslib` 2.8.1 — 0BSD (loaded via the import map for the
  Arrow result table).
- Non-MIT transitive packages in the lockfile: `qs` 6.17.1 —
  BSD-3-Clause (declared dependency of `@duckdb/duckdb-wasm`, installed
  by `npm ci`; no dist bundle imports it, so it is not loaded at runtime
  by the benchmark); `@swc/helpers` 0.5.23 — Apache-2.0 (declared
  dependency of `apache-arrow`; not imported by the `Arrow.dom.mjs`
  tree, not loaded at runtime).
- `json-bignum` 0.0.3 (declared dependency of `apache-arrow`): its
  `package.json` has no `license` field — only a legacy `licenses`
  array declaring MIT — and the tarball ships an MIT `LICENSE` file;
  the README states no licence. It is imported only by arrow's
  `arrow2csv` CLI, so the benchmark does not need it at runtime.
- `@playwright/test` + `playwright`/`playwright-core` 1.64.1 —
  Apache-2.0 (test tooling only). Every remaining transitive package in
  the lockfile is MIT per its `package.json` `license` field; no
  ISC-licensed package exists in the tree.
- No GADM data, SQLite SEE, or PowerSync used (spec decisions A4/A6/A7).
