# Epiora decision log

One entry per resolved decision. Append new entries; never rewrite history —
superseded entries stay and point at their replacement.

## Entry format

    ### <ID> — <title> (<YYYY-MM-DD>)
    Decision: <what was decided>
    Evidence: <link to the spike report or run that backs it>
    Supersedes: <ID of the entry this replaces, or "—">

### D2 — Epi Info 7 source licence (2026-10-28)

Decision: The Epi Info 7 source (`Epi-Info/Epi-Info-Community-Edition`, master @
`4cd353c`) is Apache-2.0, not public domain. Epiora may reuse its code, GOLD
grammar files (`.grm`/`.cgt`) and sample data (`Epi.Core/Projects/Sample/`,
including `Sample.mdb`) under Apache §4 notice obligations; the "Epi Info"
trademark remains CDC's and may be referenced descriptively only. Nested
third-party assets (Modern UI icons, CC BY-ND 3.0) are excluded from reuse.
Evidence: docs/spikes/d2-epi-info-licence.md
Owner sign-off: Dorrabb Khan, 2026-10-28 — accepted Apache-2.0 with notice (PR #2, Option 1).
Supersedes: —

### v0.1 Task 3 — Epi Info 7 on Windows CI (2026-10-28)

Decision: Epi Info 7 usable as a CI reference — both a Release MSBuild of
`Epi Info 7.sln` @ `4cd353c` and the official 7.2.7.0 release ZIP run a
`.pgm` headless on `windows-latest` and emit routed HTML. Reference `.pgm`s
must `READ` `.mdb` data tables directly; shipped `.prj` connection strings
do not decrypt on either binary.
Evidence: docs/spikes/ei7-windows.md
Files outside the Files block: docs/decisions.md — plan Architecture requires a decisions entry per spike.
Supersedes: —

### v0.1 Task 4 — mdbtools compiled to WebAssembly (2026-10-28)

Decision: go — libmdb (mdbtools 1.0.1, `--disable-glib --disable-iconv`)
compiles to wasm with Emscripten, reads Epi Info's Sample.mdb with CSV output
byte-identical to native `mdb-export`, handles JET3 files, reports truncated
input as an `ERROR:` naming the format, at ~61 KB gzipped total.
Evidence: docs/spikes/mdb-wasm.md
Files outside the Files block:
- spikes/mdb-wasm/test/data/Sample.mdb — Epi Info sample DB copied under D2 (Apache-2.0); the task's Consumes item permits copying.
- spikes/mdb-wasm/test/data/NOTICE — Apache §4 attribution for the copied data files (D2 consequence).
- spikes/mdb-wasm/test/data/testV1997.mdb — JET3/Access 97 test file (jackcess @ be0a213, Apache-2.0) needed by the "JET3 file reports its format" test.
- spikes/mdb-wasm/test/data/Oswego12.native.csv — native `mdb-export` output the parity test compares against (regenerated and diffed in CI).
- spikes/mdb-wasm/mdb.mjs — thin JS wrapper realising the produced interface (host-path staging + string[]/string return types over the C exports).
- spikes/mdb-wasm/dist/mdb.js, dist/mdb.wasm — built module committed so `node --test` passes without Emscripten; CI rebuilds it and re-runs the tests and measurements.
- spikes/mdb-wasm/.gitignore — ignores the build/ scratch dir (downloaded tarball + extracted source).
- .github/workflows/spike-mdb-wasm.yml — CI run producing the linked measurements.
- scripts/check.sh — bug fix: `node --test` on Node 24 cannot run a bare directory arg (MODULE_NOT_FOUND); changed to the `*.test.mjs` glob. Required for the iteration gate to run this spike's tests.
- docs/decisions.md — plan Architecture requires a decisions entry per spike.
Supersedes: —
