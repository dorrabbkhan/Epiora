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
  SDK `sdk-releases-eb132757325583b787fe9414b09afea9442b72ad-64bit`). The two
  archives `emsdk install` downloads (kept via `EMSDK_KEEP_DOWNLOADS=1`,
  verified in the workflow's "Verify emsdk-downloaded archive checksums" step):
  `wasm-binaries.tar.xz` SHA-256
  `cda6801012ff3319a539111826ed02a9abd7eff135c2ef261c8941caf5a3763e` and
  `node-v24.19.0-linux-x64.tar.xz` SHA-256
  `14b342e71204f811bde6153be8e04b62aef63c236fef92b55f9c83154b409647`.
- mdbtools 1.0.1 release tarball, SHA-256
  `ff9c425a88bc20bf9318a332eec50b17e77896eef65a0e69415ccb4e396d1812`.
- Test data (see `spikes/mdb-wasm/test/data/NOTICE`): `Sample.mdb` from
  `Epi-Info/Epi-Info-Community-Edition` @ `4cd353c` (Apache-2.0, SHA-256
  `c3f70af4…`); `testV1997.mdb` (JET3) from `jahlborn/jackcess` @ `be0a213`
  (Apache-2.0, SHA-256 `27788b47…`); `Oswego12.native.csv` produced by native
  `mdb-export` (SHA-256 `ada20497…` — regenerated and diffed in CI).
- CI: `.github/workflows/spike-mdb-wasm.yml` on `ubuntu-latest`, Node.js
  v22.23.4 via `actions/setup-node` (`node-version: '22'`).
- No RNG used.

## Measurements

All numbers below come from CI run
[40769040858](https://github.com/dorrabbkhan/Epiora/actions/runs/40769040858)
(`spike-mdb-wasm`, green in 1 m 7 s, job log + `mdb-wasm-build` artifact)
unless an exact command is given.

- Native `mdb-ver` → `JET4` on `Sample.mdb`, `JET3` on `testV1997.mdb`;
  native `mdb-tables -1` on `Sample.mdb` lists 93 user tables including
  `Oswego` and `Oswego12` (Task 3's Oswego data table). The wasm
  `mdb_tables` returns the same 93 user tables — `MSys*` system tables are
  filtered via `mdb_is_system_table`, the same rule native `mdb-tables`
  applies by default. Wasm count:
  `node -e 'import("./spikes/mdb-wasm/mdb.mjs").then(m => m.mdb_tables("spikes/mdb-wasm/test/data/Sample.mdb")).then(t => console.log(t.length))'`
  → `93`, and the "no name starts with `MSys`" assertion in
  `test/read.test.mjs` passes.
- Largest user table by rows (the "Largest user table by rows" step:
  native `mdb-export | wc -l` over every `mdb-tables -1` entry):
  `largest_table=Epi105 rows=2152`.
- Pinned-checksum step verified all three committed data files
  (`Sample.mdb`, `testV1997.mdb`, `Oswego12.native.csv`) — `OK`; the two
  emsdk-downloaded archives matched their pinned SHA-256s; native
  `mdb-export` regenerated `Oswego12.native.csv` (76 lines: header + 75
  rows) and `diff` against the committed copy passed.
- `node --test` on Node v22.23.4 → 4/4 pass: table list contains
  `Oswego`+`Oswego12` and no `MSys*`; `mdb_export_csv(..., "Oswego12")` is
  byte-identical to the native reference CSV; the 4 KB truncation returns
  `ERROR:JET4 file truncated or corrupt: cannot read table catalog` with no
  throw; the JET3 file returns `JET3` and tables `Table1`–`Table4`.
- Epi105 parity step: native `mdb-export` vs the wasm module — `cmp` on the
  two CSVs passed (byte-identical).
- The CI-built `dist/mdb.wasm` and `dist/mdb.js` are **byte-identical** to
  the committed copies (`cmp` against `git show HEAD:…` passed): the pinned
  toolchain keeps the build bit-reproducible.
- Measure step (`Buffer.byteLength(csv)` for the byte count):
  `csv_bytes=131198`, `export_ms_runs=12.3,12.6,13.7,15.1,16.3,32.3`,
  `export_ms_median=15.1` — Epi105 export ≈ 15 ms median.
- Module size (build step `ls -l`: 92,310 / 62,753 bytes; gzip sizes from
  `gzip -c dist/mdb.wasm | wc -c` etc. on the byte-identical committed
  files): `dist/mdb.wasm` 92,310 bytes (43,334 gzipped), `dist/mdb.js`
  62,753 bytes (17,556 gzipped) — ~60.9 KB gzipped total.
- `spikes` workflow run
  [40769041436](https://github.com/dorrabbkhan/Epiora/actions/runs/40769041436)
  (`spike-reports` job: `check-spikes.sh`) also green.

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
  attribution in `test/data/NOTICE`. `test/data/Oswego12.native.csv`: a changed
  form of `Sample.mdb` generated by native `mdb-export` 1.0.1 — Apache-2.0,
  notice per §4(b) in `test/data/NOTICE`.
- Node.js (test runner): MIT. `actions/checkout@v4`, `actions/setup-node@v4`,
  `actions/upload-artifact@v4`: MIT. No other dependencies beyond the plan's
  tech stack.
