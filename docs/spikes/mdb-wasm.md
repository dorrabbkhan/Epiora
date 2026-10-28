# mdbtools compiled to WebAssembly — can a wasm libmdb read Epi Info `.mdb` files?

**Status:** go

## Question

Plan Task 4, exercising spec §7.1 (`epiora-mdb`: "mdbtools' LGPL libraries
compiled to wasm and native; loaded only for legacy import") and Review Focus 1:
can `libmdb` be compiled to WebAssembly and read the Epi Info sample `.mdb`
files in Node — including a legacy Access 97/JET3 file — while reporting a
truncated or corrupt file as a clear error naming the format, never a crash or
a silent empty table?

## Method

`spikes/mdb-wasm/build.sh` compiles mdbtools 1.0.1's `src/libmdb` (only the
library, not the GPL utilities) to wasm with Emscripten, plus
`spikes/mdb-wasm/harness.c` which exports three functions:
`mdb_version(path)`, `mdb_tables(path)` (JSON array of user-table names), and
`mdb_export_csv(path, table)` — the latter replicating `mdb-export`'s CSV
defaults column-for-column via libmdb's own `mdb_print_col`, so output is
byte-comparable to the native tool. `spikes/mdb-wasm/mdb.mjs` wraps the
module: it stages a host file into the Emscripten FS and returns
`"JET3"|"JET4"|"ACE<n>"|"ERROR:<msg>"`, `string[]`, or a CSV string.
`node --test` runs `test/read.test.mjs` against the committed module in
`dist/` (built by `build.sh`; rebuilt and re-measured in CI).

Build flags that made it work: `--disable-glib` (mdbtools' own `fakeglib.c`
implements the glib subset it needs — no glib-to-wasm port is required),
`--disable-iconv` (emscripten's iconv does not know the `UCS-2LE` alias JET4
needs; the fallback `wcstombs`/`C.UTF-8` path is deterministic and correct),
`--disable-man`. Link flags: `-sMODULARIZE -sEXPORT_ES6 -sFORCE_FILESYSTEM
-sALLOW_MEMORY_GROWTH`, exporting `ccall`/`FS` and `_mdb_version`,
`_mdb_tables`, `_mdb_export_csv`, `_malloc`, `_free`.

Truncation detection: `mdb_open` only reads page 0, so a file truncated after
the header still opens. `harness.c` therefore verifies readability by also
reading the table catalog; when either step fails it sniffs the version byte
itself and returns `ERROR:<format> ...` (e.g. `ERROR:JET4 file truncated or
corrupt: cannot read table catalog`).

## Environment

- Host: Linux x86-64, Node.js v24.22.0, GCC system toolchain.
- Emscripten SDK 6.0.13 (`emsdk` @ `d7b4130688bfa348392b921ab04015aa15c4f154`,
  SDK `sdk-releases-eb132757325583b787fe9414b09afea9442b72ad-64bit`).
- mdbtools 1.0.1 release tarball, SHA-256
  `ff9c425a88bc20bf9318a332eec50b17e77896eef65a0e69415ccb4e396d1812`.
- Test data (see `spikes/mdb-wasm/test/data/NOTICE`): `Sample.mdb` from
  `Epi-Info/Epi-Info-Community-Edition` @ `4cd353c` (Apache-2.0, SHA-256
  `c3f70af4…`); `testV1997.mdb` (JET3) from `jahlborn/jackcess` @ `be0a213`
  (Apache-2.0, SHA-256 `27788b47…`); `Oswego12.native.csv` produced by native
  `mdb-export` (SHA-256 `ada20497…` — regenerated and diffed in CI).
- CI: `.github/workflows/spike-mdb-wasm.yml` on `ubuntu-latest`.
- No RNG used.

## Measurements

Native (commands run on the host):

- `mdb-ver test/data/Sample.mdb` → `JET4`; `mdb-tables -1` lists 93 user
  tables including `Oswego` and `Oswego12` (Task 3's Oswego data table).
- `mdb-ver test/data/testV1997.mdb` → `JET3`; `mdb-tables -1` → `Table1`–`Table4`.
- `mdb-ver` on the first-4 KB truncation of `Sample.mdb` still prints `JET4`
  (header-only check) — confirming that the produced `mdb_version` must open
  the catalog, not just the header.
- `mdb-export Sample.mdb Oswego12` → 76 lines (header + 75 rows), SHA-256
  `ada2049737fcb31412692a979b1017ce11583a4c03d41d66b2b1ea7113d5de67`,
  committed as `test/data/Oswego12.native.csv`. Largest table in `Sample.mdb`
  by `mdb-export | wc -l`: `Epi105`, 2,152 data rows.

Wasm (commands run on the host; CI links below):

- `node --test spikes/mdb-wasm/test/*.test.mjs` → 4/4 pass:
  table list contains `Oswego`+`Oswego12`; `mdb_export_csv(..., "Oswego12")`
  is **byte-identical** to the native reference CSV; the 4 KB truncation
  returns `ERROR:JET4 file truncated or corrupt: cannot read table catalog`
  with no throw; the JET3 file returns `JET3` and tables `Table1`–`Table4`.
- `node -e` timing script (see workflow "Measure" step for the exact script):
  `mdb_export_csv(Sample.mdb, "Epi105")` → 131,198 bytes, identical to native;
  6 runs: 50.8 ms cold then 26.9 / 23.2 / 20.4 / 20.5 / 19.2 ms — median of
  the 5 warm runs ≈ 20.5 ms.
- Module size: `dist/mdb.wasm` 92,280 bytes (43,320 gzipped), `dist/mdb.js`
  62,753 bytes (17,556 gzipped) — ~60.9 KB gzipped total.

CI run [40766020082](https://github.com/dorrabbkhan/Epiora/actions/runs/40766020082)
(`spike-mdb-wasm`, green in 1 m 23 s, job log + `mdb-wasm-build` artifact):

- Pinned-checksum step verified all three committed data files
  (`Sample.mdb`, `testV1997.mdb`, `Oswego12.native.csv`) — `OK`.
- Native `mdb-ver` → `JET4` / `JET3`; native `mdb-export` regenerated
  `Oswego12.native.csv` and `diff` against the committed copy passed.
- `node --test` → `pass 4`.
- Measure step (`export_ms_runs=19.6,21.7,22.6,23.7,30.7,63.7`,
  `export_ms_median=23.7`, `csv_bytes=131198`): Epi105 export ≈ 24 ms median.
- The CI-built `mdb.wasm` (artifact `mdb-wasm-build`) is **byte-identical**
  to the committed local build — SHA-256
  `3d245ed45f80bb0b0cec3e68f3f9c99c83ef3e64b81059856de4f72d9c7372ab`
  (`sha256sum` on the downloaded artifact vs `dist/mdb.wasm`): the pinned
  toolchain makes the build bit-reproducible.
- `spikes` workflow run 40765905982 (`spike-reports` job: bats +
  `check-spikes.sh`) also green.

## Decision

**go** — libmdb compiles cleanly to wasm (no glib port needed thanks to
`--disable-glib`), reads Epi Info's own `Sample.mdb` with byte-identical CSV
parity to the native `mdb-export`, handles JET3 (Access 97) files, reports
truncated input as an `ERROR:` naming the format instead of crashing, and the
whole reader fits in ~61 KB gzipped — trivially inside the spec §12.8
first-load budget even bundled in the main load (it is meant to load lazily
anyway).

## Consequences

- `epiora-mdb` (spec §7.1) is viable in the browser: keep the same
  `--disable-glib --disable-iconv` build recipe; the harness API already
  matches the task's produced interface.
- LGPL-2.0 constraint to carry forward: libmdb is statically linked into the
  wasm module here. For a product unit, LGPL §6(b)-style compliance means
  shipping libmdb as a separately replaceable wasm artifact (or offering
  relinkable objects) — record this when `epiora-mdb` is designed.
- `mdb_version` semantics adopted for the reader: "format of a *readable*
  database"; a file whose header parses but whose catalog does not is an
  `ERROR:` naming the detected format (satisfies Review Focus 1).
- The interface's literal return union omits ACE (`.accdb`) formats; the
  module returns `ACE12|ACE14|ACE15|ACE16|ACE18` for them (mdb-ver's names)
  rather than folding them into `JET4` — noted so the interface can be
  updated when `.accdb` import lands.
- JET3 catalog reads work; writing or editing `.mdb` remains untouched
  (spec non-goal).
- `.prj` files remain unusable (Task 3 finding); import must target `.mdb`
  tables directly.

## Licences

- `libmdb` (mdbtools 1.0.1): LGPL-2.0 (`COPYING.LIB` in the tarball; source
  headers grant "Lesser GPL" terms). Statically linked into the spike module —
  acceptable for a throwaway spike; product use needs the linking arrangement
  noted in Consequences. The GPL-2.0 utilities (`COPYING`) are **not** linked;
  `harness.c` replicates `mdb-export`'s output through the public libmdb API.
- Emscripten SDK 6.0.13 (compiler toolchain, not linked into Epiora source):
  MIT/University of Illinois-NCSA.
- `test/data/Sample.mdb`: Apache-2.0 (Epi Info 7 source @ `4cd353c`, D2);
  `test/data/testV1997.mdb`: Apache-2.0 (jackcess @ `be0a213`). Both unmodified;
  attribution in `test/data/NOTICE`.
- Node.js (test runner): MIT. `actions/checkout@v4`, `actions/upload-artifact@v4`:
  MIT. No other dependencies beyond the plan's tech stack.
