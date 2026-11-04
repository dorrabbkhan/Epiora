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
Interface deviation: `mdb_version` returns `ACE12`–`ACE18` (mdb-ver's names)
for `.accdb` inputs, outside the plan's union `"JET3"|"JET4"|"ERROR:<message>"`.
Kept because folding ACE formats into `JET4` would misreport the format and
lose the distinction mdb-ver itself makes; `.accdb` import is a later task and
can widen the union then. `mdb_tables` returns user tables only (the
`mdb-tables` default); system (`MSys*`) catalog entries are filtered via
libmdb's `mdb_is_system_table`, matching the native tool and the report's
93-table count.
Files outside the Files block:
- spikes/mdb-wasm/test/data/Sample.mdb — Epi Info sample DB copied under D2 (Apache-2.0); the task's Consumes item permits copying.
- spikes/mdb-wasm/test/data/NOTICE — Apache §4 attribution for the copied data files (D2 consequence).
- spikes/mdb-wasm/test/data/testV1997.mdb — JET3/Access 97 test file (jackcess @ be0a213, Apache-2.0) needed by the "JET3 file reports its format" test.
- spikes/mdb-wasm/test/data/Oswego12.native.csv — native `mdb-export` output the parity test compares against (regenerated and diffed in CI).
- spikes/mdb-wasm/mdb.mjs — thin JS wrapper realising the produced interface (host-path staging + string[]/string return types over the C exports).
- spikes/mdb-wasm/dist/mdb.js, dist/mdb.wasm — built module committed so `node --test` passes without Emscripten; CI rebuilds it and re-runs the tests and measurements.
- spikes/mdb-wasm/.gitignore — ignores the build/ scratch dir (downloaded tarball + extracted source).
- .github/workflows/spike-mdb-wasm.yml — CI run producing the linked measurements.
- scripts/check.sh — bug fix: `node --test` with a bare directory arg fails with MODULE_NOT_FOUND on both Node 24 (v24.22.0) and Node 22 (verified on v22.21.0); changed to the `*.test.mjs` glob, which is correct on both. Required for the iteration gate to run this spike's tests.
- docs/decisions.md — plan Architecture requires a decisions entry per spike.
Supersedes: —

### v0.1 Task 5 — R reference fixture pipeline (2026-10-28)

Decision: go — R fixtures regenerate byte-identically under renv 1.3.1 (R
4.5.2, epitools 0.5-10.1, jsonlite 2.0.0, all lockfile-pinned); pinning
choice is "renv" over "Docker image". CI validates each fixture against
fixtures/schema/fixture.schema.json and fails on any drift after
regeneration (proven in run 40783827413).
Evidence: docs/spikes/r-fixtures.md
Files outside the Files block:
- fixtures/.Rprofile — renv activation entry point (sources renv/activate.R) written by renv::init.
- fixtures/renv/activate.R — renv bootstrap; pins renv itself to 1.3.1, written by renv::init.
- fixtures/renv/settings.json — renv project settings written by renv::init.
- fixtures/renv/.gitignore — excludes the project library (renv/library/) from git, written by renv::init.
- fixtures/tools/package.json, fixtures/tools/package-lock.json — committed npm manifest + lockfile (ajv-cli 5.0.0 with integrity-pinned transitive deps) installed by `npm ci` in CI; added in fix round 1 so the schema validator's dependencies are locked.
- fixtures/tools/.gitignore — ignores fixtures/tools/node_modules so `npm ci` output cannot trip the drift guard; added in fix round 1.
- docs/decisions.md — plan Architecture requires a decisions entry per spike.
Supersedes: —

### v0.1 Task 6 — D3 query engine benchmark (2026-10-29)

Decision: SQLite for storage plus DuckDB-wasm for analytic queries —
by owner decision on PR #8 (2026-10-29), the plan's rule (every SQLite
median ≤ 1000 ms → SQLite alone; otherwise SQLite + DuckDB-wasm) is
applied to the worst observed CI median across all recorded runs on the
measured code. The worst SQLite median is the weekly epi-curve at
1065.7 ms (CI run 40884469336), and 3 of the 7 recorded CI runs exceed
1000 ms on it (40883881690, 40884469336, 40890984797); every other
SQLite median and all duckdb-wasm medians are under budget in every
run. DuckDB-wasm adds ~8.0 MB gz (8,326,520 bytes vs sqlite-wasm's
586,888), so it must lazy-load outside the §12.8 first-load budget. On
the 50M-row case the sqlite rows record
`error: "SQLITE_NOMEM: sqlite3 result code 7: out of memory"` while
duckdb ingests all 50M rows and answers its queries (Review Focus 5);
a pre-chunking run 40883237792 crashed the renderer instead — recorded
as a measurement.
Evidence: docs/spikes/d3-query-engine.md
Files outside the Files block:
- spikes/query-bench/package.json, spikes/query-bench/package-lock.json — committed npm manifest + integrity-pinned lockfile so CI installs the exact measured versions via `npm ci` (kept inside the spike folder per conductor note; npm cache lives outside the repo).
- spikes/query-bench/.gitignore — ignores node_modules/ and the generated results.json (a run artifact, never committed).
- .github/workflows/spike-query-bench.yml — CI run producing the linked measurements (npm ci, generator SHA-256, playwright bench + OOM case, results.json artifact).
- docs/decisions.md — plan Architecture requires a decisions entry per spike.
Supersedes: —

### v0.1 Task 7 — D6 browser encryption (2026-10-29)

Decision: field-level only — WebCrypto AES-GCM 256 field encryption
(§11.1 interface produced by this spike) is the browser path; whole-DB
encryption via the pinned open SQLCipher wasm build works end to end on
OPFS (marker absent from raw bytes, wrong key rejected, ~0 ms write
overhead at 10k rows) but its MEMFS+whole-file-export persistence model
has no page-level OPFS VFS, and it adds 1.66 MB gz to first load.
Evidence: docs/spikes/d6-browser-encryption.md
Files outside the Files block:
- spikes/browser-crypto/package.json, spikes/browser-crypto/package-lock.json — committed npm manifest + integrity-pinned lockfile so CI installs the exact measured versions via `npm ci` (kept inside the spike folder per Task-6 convention).
- spikes/browser-crypto/.gitignore — ignores node_modules/, the vendored sqlcipher-wasm assets (downloaded by fetch-vendor.sh) and the generated crypto-results.json (a run artifact, never committed).
- spikes/browser-crypto/vendor/fetch-vendor.sh — downloads the pinned 7mind/sqlcipher-wasm v1.4.0 release zip and tagged wrapper/LICENSE, each verified by SHA-256; needed because the build is not published on npm.
- .github/workflows/spike-browser-crypto.yml — CI run producing the linked measurements (npm ci, SHA-256-verified vendor fetch, playwright checks + measurements, crypto-results.json artifact).
- docs/decisions.md — plan Architecture requires a decisions entry per spike.
Supersedes: —

### v0.1 Task 8 — D7 reference device and WebR on low-end Android (2026-10-29)

Decision: The plan's proxy ("Pixel emulator + Chrome DevTools 4× CPU
throttle and Slow 4G", run as desktop Chromium per Step 2) is the
recorded D7 reference device, by owner choice (2026-10-29) — no
physical ≤ 4 GB device is available. On it, WebR 0.6.0 loads ~13.3 MB
gz on the wire (12.3 MB of it re-downloaded even on a warm load —
Chrome does not retain the streamed R.wasm) and reaches ready in
~70–80 s, far over the 10,000 ms warm budget, so the task's rule
decides "advanced methods are hosted-only on mobile (spec §9)". A
real-device confirmation (plan Step 5 on a physical ≤ 4 GB Android
phone) is deferred to a later plan.
Evidence: docs/spikes/d7-webr-mobile.md
Files outside the Files block:
- spikes/webr-mobile/package.json, spikes/webr-mobile/package-lock.json — committed npm manifest + integrity-pinned lockfile so CI installs the exact measured versions via `npm ci` (Task-6/7 convention).
- spikes/webr-mobile/.gitignore — ignores node_modules/ and the generated webr-results.json (a run artifact, never committed).
- .github/workflows/spike-webr-mobile.yml — CI run producing the linked measurements (npm ci, `node measure.mjs`, webr-results.json artifact).
- docs/decisions.md — plan Architecture requires a decisions entry per spike.
Supersedes: —

### v0.1 Task 9 — Sync protocol prototype (2026-10-29)

Decision: go — the spec §7.3 sync outline (Lamport clocks, server-assigned
per-field versions, base-version conflicts keeping both writes) holds its
invariants under simulation: no lost write, every conflict surfaced, all
devices converge, and ordering is deterministic under 1-hour wall-clock skew
and identical timestamps. The v2.0 sync spec must add tombstones,
form-version migration and persistence; Electric remains a fallback for D8.
Evidence: docs/spikes/sync-proto.md
Files outside the Files block:
- spikes/sync-proto/Cargo.lock — lockfile pinning rand 0.9.5 / rand_chacha 0.9.0 and transitive deps for reproducible runs (cargo-generated).
- docs/decisions.md — plan Architecture requires a decisions entry per spike.
Supersedes: —

### v0.1 Task 10 — Tauri 2 shell on Android and iOS (2026-10-29)

Decision: go-with-changes — Tauri for desktop and mobile. A Tauri 2 app
(vanilla template, com.epiora.spike) builds an Android debug APK that
shows `2x2 RR = 5.57` from a wasm `risk_ratio` on an emulator, and
builds + launches on the iOS simulator; the SQL plugin opens
`epiora.db` keyed via `PRAGMA key` on SQLite3MultipleCiphers (vendored
libsqlite3-sys compiling sqlite3mc-src 2.5.1, chacha20), verified
on-device in CI (row write + read-back, ciphertext header). Changes:
keying needs connect-time pragma support or a fork before v0.3 (the
plugin's pool and auto-create defeat a raw `PRAGMA key` execute —
measured on-device); the vendored libsqlite3-sys patch must track
upstream bumps.
APK size and cold start on the D7 reference: measured on the D7 proxy
(CI emulator) per the owner's Task 8 decision and owner option A for
Step 4 — aarch64 release APK (unsigned, arm64-v8a only) 8,560,415
bytes; cold start `am start -W` TotalTime 1716/1354/740 ms on the
x86_64 debug APK (the aarch64 APK cannot run on the x86_64 emulator);
real-device confirmation deferred to a later plan.
Evidence: docs/spikes/tauri-mobile.md
Files outside the Files block:
- spikes/tauri-shell/package.json, package-lock.json — npm manifest + lockfile for `npm ci` (create-tauri-app generated + edited).
- spikes/tauri-shell/src-tauri/Cargo.lock — resolved crate versions incl. the patched libsqlite3-sys (cargo-generated).
- spikes/tauri-shell/stats-wasm/ (Cargo.toml, Cargo.lock, src/lib.rs, .gitignore) — the wasm crate behind the produced `risk_ratio` interface.
- spikes/tauri-shell/build-wasm.sh — builds src/stats_wasm.wasm from the crate (the plan's npm-script hook).
- spikes/tauri-shell/src/stats_wasm.wasm — built module committed so the app works without a wasm toolchain; CI rebuilds it.
- spikes/tauri-shell/src-tauri/vendor/libsqlite3-sys/ — vendored 0.30.1 with the bundled amalgamation swapped for sqlite3mc-src (MIT); implements "SQL plugin built with SQLite3MultipleCiphers".
- docs/decisions.md — plan Architecture requires a decisions entry per spike.
Supersedes: —

### v0.1 Task 11 — v0.1 closed: §17 rows D2, D3, D6, D7 resolved (2026-10-30)

Decision: spec §17 rows D2, D3, D6 and D7 are marked resolved, each quoting
its merged report's Decision line verbatim and linking the report and this
log. `docs/spikes/README.md` indexes the nine reports (status, verbatim
decision, link) and lists the inputs v0.2 consumes. The sentences the
decisions change in §8.3 (data wrangling engine), §9 (WebR mobile fallback)
and §11.1 (browser whole-DB encryption), plus the stale references in §2
(D2 licence) and §12.8 (D7 reference proxy), were updated on the owner's
decision in fix round 1 with the wording proposed in the PR.
Evidence: docs/spikes/README.md; spec §17; the nine reports under docs/spikes/
Files outside the Files block:
- scripts/check-spikes.sh — README.md added to the exempt list alongside
  TEMPLATE.md so the new spike index is not linted as a report (the task's
  Files block did not anticipate the index tripping the checker).
- scripts/tests/check-spikes.bats — "ignores README.md" case covering the
  README.md exemption.
Supersedes: —

### v0.2 Task 1 — engine workspace (2026-11-02)

Decision: The v0.2 engine workspace is scaffolded as planned: `epiora-core`
(error type, tolerance classes, E7 comparison, corpus comparator) and the
empty `epiora-wasm` cdylib, with the float ban enforced by
`crates/epiora-core/clippy.toml` and the licence/advisory gate by
`deny.toml`. Pinned tooling: Rust 1.99.0 (`rust-toolchain.toml`, clippy +
rustfmt components), cargo-deny 0.20.2 (CI downloads the musl tarball and
verifies SHA-256 `9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f`),
bats-core 1.14.0 (CI downloads the source archive and verifies SHA-256
`bb537b70b15b732f6d8827dd6578e3d8ce166636ce1f18ea9a074184fcce9177`).
E8 is applied: `spikes/webr-mobile/measure.mjs` reads the Fisher p-value
and tolerance from both fixture format v1 (`fisher_exact_test.p_value` +
top-level `tolerance`) and v2 (`fisher_exact.p_two_sided` leaf +
`tolerance_classes[class]`); its threshold and decision rule are
untouched.
Evidence: `cargo test --workspace --all-features` (8 compare + 2 error
tests) and `bats scripts/tests/float-ban.bats` locally; the `engine` and
`spike-webr-mobile` workflow runs linked in the PR.
Files outside the Files block:
- spikes/webr-mobile/measure.mjs — in the task's Files block but outside
  v0.2's usual scope: the only `spikes/` edit in v0.2 (E8), so the frozen
  WebR spike reads fixture format v2 before Task 2 changes it.
- Cargo.lock — cargo-generated lockfile; committed so cargo-deny and CI
  resolve the same pinned dependency graph.
- .github/workflows/spike-tauri.yml — the new root rust-toolchain.toml
  overrides the `stable` toolchain this workflow installed, so it now
  installs the pinned 1.99.0 itself, keeping the Android/iOS/wasm targets
  its builds need (CI failure seen in run 41436582500). Not a spikes/
  edit; the spike code is untouched.
- docs/decisions.md — this entry.
Supersedes: —

### v0.2 Task 2 — corpus v2 in R (2026-11-02)

Decision: The corpus moves to fixture format v2 (spec §6.2): every expected
leaf is `{value, class, r_default?}` under `tolerance_classes`
`{closed_form: 1e-9, iterative: 1e-6}`; root-found values are solved with
`uniroot(tol = 1e-14)` inside verbatim ports of R 4.6.2's `fisher.test`
internals and `epitools` 0.5-10.1's `or.midp`/`ormidp.test`, with a
generation-time check that each tight root agrees with R's
default-tolerance result within 1e-2 relative. The corpus covers
`special`, `2x2` (incl. a regenerated `oswego_vanilla`), `stratified`,
`rxc`, `freq` and `data/*.dataset.json` (spec §3.3). The
`spike-fixtures` workflow is renamed `fixtures.yml`; it validates all
five fixture folders plus the dataset JSONs and keeps the
regenerate-then-diff drift guard.
Package versions and licences (reference generation only, never
distributed), as of the owner-approved upgrade round below:
epitools 0.5-10.1 GPL (>= 2); epiR 3.0.0 GPL (>= 2); DescTools 0.99.60
GPL (>= 2); fs 2.1.0 MIT + file LICENSE; sf 1.1-3 GPL-2 | MIT + file
LICENSE; s2 1.1.13 Apache License (== 2.0); units 1.0-1 GPL-2;
flextable 0.10.3 GPL-3; officer 0.7.7 MIT + file LICENSE; gdtools 0.5.2
GPL-3 | file LICENSE; DFBA 0.1.0 GPL-2; BiasedUrn 2.0.12 GPL-3;
pander 0.6.6 AGPL-3; remaining transitive deps MIT/GPL/LGPL/BSD per
`fixtures/renv.lock` (107 packages).
Interface deviation (recorded for the owner): the plan's "MH (N−1) from
`mantelhaen.test` on the single-stratum array" cannot run literally —
R 4.6.2's `mantelhaen.test` requires every dimension ≥ 2, i.e. at least
two strata. `fixtures/R/common.R` therefore carries a verbatim port of
the non-exact 2×2×K branch of `mantelhaen.test` with only the dimension
guard removed; on every fixture where the stock function can run (≥ 2
non-empty strata) generation asserts the port reproduces it bit for bit.
This supplies `chi_square.mantel_haenszel` in the `two_by_two` fixtures
and the `mantel_haenszel` block of `stratified` `one_stratum`. For the
same reason `one_stratum`'s `mantel_haenszel.risk_ratio` uses epiR's
per-stratum `RR.strata.wald` (identical to the pooled MH value for one
stratum), and `breslow_day_tarone` is undefined there (df = K−1 = 0).
`freq` `inputs.labels` mirrors `expected.categories[]` positionally and
includes `"(missing)"` last only when `include_missing` is true and the
column has missing values; percentage and CI denominators include the
missing category only in that case. `midp_exact.p_two_sided` is classed
`closed_form` because it is a direct pnhyper sum, not a root-found value.
Evidence: local `Rscript fixtures/generate.R` runs byte-identical twice;
`fixtures.yml` workflow run linked in the PR; `oswego_vanilla`
`odds_ratio.ci_95_fisher_exact.upper` moved from 138.86503356049562
(v1 default tolerance) to 138.39347928707681 (tight) — both expected
values named in the plan.
Files outside the Files block: none.
Supersedes: —

#### Task 2 upgrade round (2026-11-02)

Owner decision (PR #17 conversation, verbatim): "i don't care about
slower CI or whatever as long as quality is high and we have parity
with latest equivalent tools" and "R can also update while we're at it".
On that authority the following deviations from the plan's "R 4.5.2"
wording and from the earlier pins were made:

- R 4.5.2 → 4.6.2 (2026-10-29). `fixtures/renv.lock` `R.Version` and
  `fixtures.yml` `R_VERSION` now say 4.6.2; `r-lib/actions/setup-r`
  installs it in CI. The lockfile's pins of R 4.5.2's recommended
  packages could not all be kept: Matrix 1.7-4 fails to build on R 4.6.2
  (`error: implicit declaration of function 'Rf_findVarInFrame'` /
  `assignment to 'SEXP' from 'int'` in `cholmod-common.c` — R 4.6 removed
  that non-API entry point). The recommended packages in the lockfile
  moved to the versions shipped with R 4.6.2 (which are also current
  CRAN): Matrix 1.7-6, MASS 7.3-66, class 7.3-24, lattice 0.23-1,
  survival 3.8-12 (boot stayed 1.3-32; KernSmooth 2.23-27 came in as a
  new transitive dep).
- Ports re-verified against R 4.6.2 sources before the "R 4.6.2"
  citations were kept: `deparse(body(stats::fisher.test))` 2×2 branch is
  unchanged vs the ported `dnhyper`/`mnhyper`/`pnhyper`/`mle`/`ncp.L`/
  `ncp.U`; `deparse(body(stats::mantelhaen.test))` non-exact 2×2×K
  branch is unchanged vs `mh_2x2xk` (dimension guard still present
  upstream, so the port deviation stands); `epitools` is still 0.5-10.1,
  so the mid-p ports are unchanged.
- epiR 2.0.38 → 3.0.0 (latest CRAN): the sf-avoidance pin is dropped;
  epiR 3.0.0 hard-imports `sf` (plus `flextable`, `officer`, `DFBA`,
  `zoo`), so `sf` and its deps (s2, units, wk, classInt, DBI, …) are now
  in the lockfile. fs 1.6.6 → 2.1.0 (latest CRAN): the libuv/cmake
  avoidance pin is dropped; `libuv1-dev` and `cmake` are installed in CI.
- `fixtures.yml` apt step extended beyond the task's named list
  (libgdal/geos/proj/udunits2/uv/cmake): epiR 3.0.0's tree needs
  `libsqlite3-dev` (sf), `libabsl-dev` (s2), `libcairo2-dev` (gdtools),
  `libharfbuzz-dev` + `libfribidi-dev` (textshaping), `libfreetype-dev` +
  `libfontconfig-dev` (systemfonts/gdtools), `libpng-dev` + `libtiff-dev`
  + `libjpeg-dev` + `libwebp-dev` (ragg), plus `libicu-dev` (stringi),
  `libcurl4-openssl-dev`/`libssl-dev`/`libxml2-dev`/`zlib1g-dev` and
  `gfortran` for the rest of the source builds.
- Local regeneration note: this host lacks the cairo/harfbuzz/fribidi
  headers, so packages were installed locally from Posit Package
  Manager's Ubuntu 26.04 (resolute) R-4.6 binaries; the lockfile records
  only `Repository: CRAN` (P3M "RSPM" source fields normalised), so CI
  restores the identical versions from source.
- Regeneration under R 4.6.2: two consecutive
  `Rscript fixtures/generate.R` runs are byte-identical (sha256 over all
  39 fixture JSONs). Compared leaf-by-leaf against the previous commit
  (every `value`/`r_default` under `expected`, excluding
  `source.r_version`/`source.packages`): **no fixture value changed**.
  The only diffs are `source.r_version` → "R version 4.6.2 (2026-10-29)"
  and `source.packages` (stats 4.5.2 → 4.6.2, epiR 2.0.38 → 3.0.0) on
  every fixture. All generation-time checks pass unchanged: tight-root
  1e-2, `check_mh_port` bitwise vs stock `mantelhaen.test`,
  `check_mh_single_table` ((N−1)/N identity), `check_rr_mh` (epiR MH risk
  ratio vs port), the VANILLA sanity table, Breslow-Day p < 0.01. The 16
  `chisq.test` warnings are unchanged degenerate-table notices.
  Command used for the value diff: recursive JSON leaf walk of
  `git show HEAD:<file>` vs each regenerated file (script in the PR
  body).

### v0.2 Task 3 — Epi Info 7 reference output for Oswego (2026-11-02)

Decision: `fixtures/ei7/` now holds the Epi Info 7.2.7.0 reference output
for the Oswego program (`FREQ SEX`, `TABLES VANILLA ILL`, `TABLES VANILLA
ILL STRATAVAR=SEX`): `oswego.html` byte-for-byte from the ei7.yml artifact
and `oswego.json` parsed from it, with `labels.json` mapping all 180
emitted labels — 56 to corpus key paths (`freq.`, `two_by_two.`,
`stratified_two_by_two.`), 124 explicitly `null`.

- Release ZIP SHA-256: `8440a628a28fd1c1993ebc8211fe0104fdb2572ae8bcb3427e07baceb08df609`
  (verified in the producing run's log).
- Sample.mdb SHA-256: `c3f70af4aef2de572aadeb127e74661667642b325d9f0f2792f1e60e00f9078e`
  — matches the fixture's expected hash, so the data Epi Info read is the
  same `Oswego12` table behind `fixtures/data/oswego.csv`.
- Producing run: `https://github.com/dorrabbkhan/Epiora/actions/runs/41507625047`
  (recorded as `source.run_url`; ei7.yml preserves the committed value on
  reruns so the `git diff --exit-code fixtures/ei7` drift check can go
  green, and a run that produces different output fails the diff).
- Output licence: the output is produced by the official Epi Info 7
  build; the source it is built from is Apache-2.0 at `4cd353c` (D2).
  The signed release ZIP itself states no licence, EULA or notice in the
  archive (verified on the checksum-pinned file, docs/spikes/ei7-windows.md)
  and is used only as a downloaded CI oracle, never redistributed.
- Unmapped labels (`key: null`) are enumerated in `labels.json`: every
  crosstab cell and Row%/Col% (table cells are procedure inputs, not
  estimates), `Total Percent`/`Total Cum. Percent`, `Mid-p exact 1-tailed
  p` (the corpus stores only the two-sided mid-p), all per-stratum
  statistics under `SEX=Female`/`SEX=Male`, and `Adjusted OR (MLE)` and
  its CI (Epi Info reports an adjusted MLE odds ratio the corpus does not
  cover). Cells printed as `Undefined` (Male stratum OR/RR — a zero cell)
  or `NaN` (Breslow-Day for Odds/Risk Ratio variants) emit no value —
  they carry no number.
- Mapping notes for Task 9: Epi Info's `(T)` confidence limits are Taylor
  series — mapped to the corpus `ci_95_wald` paths, but for the odds
  ratio Epi Info prints 5.8410–94.1811 where the corpus leaf is
  5.8412–94.1787, a difference beyond display precision to be reconciled
  at Task 9, not a relabel. `(R)` is the RGB interval (adjusted OR:
  7.1914–197.0062 vs corpus 7.1916–197.0002). `Risk Difference (RD%)` and
  FREQ/table percentages are on the percent scale (risk leaves are 0–1).
  `Fisher exact 1-tailed p` maps to `fisher_exact.p_upper` — Epi Info
  prints the tail in the direction of the association (upper for Oswego).

Files outside the Files block:
- .github/workflows/fixtures.yml — its parser-test step said
  `node --test fixtures/ei7`; a bare directory argument fails with
  MODULE_NOT_FOUND on Node ≥21 including the pinned 22.23.4, so the step
  now uses the `fixtures/ei7/*.test.mjs` glob (same test file).
- docs/decisions.md — this entry.

Notes: `.github/workflows/ei7.yml` carried a temporary `push` trigger on
this branch during development because a workflow file that does not yet
exist on the default branch cannot be `workflow_dispatch`ed; it was
removed before the PR, leaving the plan's dispatch + weekly triggers.
Supersedes: —

### v0.2 Task 4 — `dataset`, `project` and the corpus dataset loader (2026-11-03)

Decision: The spec §3.3 dataset model is implemented as planned:
`Dataset`/`Column`/`ColumnData`/`Value`/`ValueType` with `Dataset::new`
validation (`EmptyDataset` on zero columns; `InvalidColumn` on unequal
lengths, duplicate or empty names, non-finite reals), canonical bytes
and a `-0.0`-normalised SHA-256 content hash, the `Project`/`DatasetRef`
types (type only, spec §3.2), and `corpus::load_dataset` /
`load_dataset_dir` for `fixtures/data/*.dataset.json` (malformed JSON or
schema violations → `CorpusFormat`; dataset rules keep their own
errors). Dates parse strict `YYYY-MM-DD` via a proleptic-Gregorian
`days_from_civil`; anything else is `CorpusFormat`. The `golden_hash`
constant was computed with the `xxd -r -p | sha256sum` command recorded
in the PR body.
Evidence: `cargo test -p epiora-core --all-features` (11 dataset + 4
corpus_datasets tests) and `./scripts/check.sh` locally; the `engine`
workflow run linked in the PR.
Files outside the Files block:
- Cargo.lock — cargo-generated lockfile updated by the new `sha2`
  dependency so cargo-deny and CI resolve the same pinned graph.
- docs/decisions.md — this entry.
Supersedes: —

### v0.2 Task 5 — `special` maths on `libm` (2026-11-03)

Decision: `special` is implemented as planned, with two method choices
the fixtures forced beyond the §3.4 "Method" column's shorthand, and one
manifest change:

- `normal_cdf`/`normal_sf` port R's `pnorm_both` (Cody rational
  approximations, `src/nmath/pnorm.c`) instead of composing `erfc`: the
  `normal_cdf` fixture's `x = −38` leaf is the denormal
  `2.88542835e-316`, whose `closed_form` band is narrower than one
  denormal ulp — only R's own `exp(-xsq²/2)·exp(-del/2)·temp` path is
  bit-identical. `normal_quantile` ports `qnorm`'s AS 241.
- `hypergeom_pmf` ports R's `dhyper`/`dbinom_raw` (`stirlerr` + `bd0`):
  a plain `ln_gamma` difference fails `closed_form` at the fixture's
  `m = n = 2e6` point by 6.9e-9 > 1e-9 (measured during development).
  `binomial_tail` stays "summed in log space" via `dbinom_raw`'s log
  density; `NcHypergeom` is the `fisher.test` construction as specified.
- `crates/epiora-core/Cargo.toml` (in the Files block, though the named
  change was `libm`): `serde_json` gained `features =
  ["float_roundtrip"]`. serde_json's default float parser is not
  correctly rounded — it reads jsonlite's 17-digit `1.0000000000000001e-09`
  (the v2 `tolerance_classes.closed_form`) as `9.999999999999999e-10`, so
  `compare`'s exact-class check rejected every real fixture and each leaf
  could carry a 1-ulp parse error. `float_roundtrip` is serde_json's
  correctly rounded parser; unlike `arbitrary_precision` it does not
  change `serde_json::Number`'s representation for other crates in the
  workspace via feature unification.
Evidence: `cargo test -p epiora-core --all-features` (12 special tests,
all fixture points in tolerance) and `./scripts/check.sh` locally; the
`engine` workflow run linked in the PR.
Files outside the Files block:
- docs/decisions.md — this entry.
Supersedes: —

### v0.2 Task 6 — `stats`: 2×2, R×C, proportions (2026-11-03)

Decision: `stats` is implemented as planned. Points the fixtures or
the interface list fixed beyond the plan's prose:

- `risk_ratio.small_sample` is `epitools::riskratio.small` as generated:
  the estimate is `(a/(a+b)) / ((c+1)/(c+d+1))` — only the reference
  group is adjusted — while its bounds use the *unadjusted* Wald
  standard error. Verified against all four `zero_*` fixtures, Oswego
  and `large`.
- `odds_ratio.ci_95_midp_exact`: when the observed cell sits at either
  end of the conditional support (`x == lo` or `x == hi`), both bounds
  are `Undefined { Unbounded }`. The `zero_*` fixtures carry null for
  both bounds in exactly these cases, even where the mid-p equation
  would still bracket a root — that is the reference behaviour
  (`epitools::oddsratio(method = "midp")` returns NA there).
- `freq_proportion_fixtures` is scoped to the `categories` subtree:
  `proportions` produces only `CategoryStats`; `total`/`missing` come
  from tabulation in Task 8.
- `chi_square.df` (and every other output leaf) is an `Estimate`, so a
  serialized result never contains `null`: `Value(x) → x`,
  `Undefined → {"undefined": "<code>"}`.
- `fisher_exact.p_two_sided` is built as
  `min(p_lower + qualifying mass above x, p_upper + qualifying mass
  below x)`: each candidate anchors on the identical one-sided-tail
  `f64`, so `p_two_sided ≥ min(one-sided)` holds in floating point by
  construction, and by pmf unimodality the smaller candidate is
  `fisher.test`'s `relErr` sum (fix-round change in `exact.rs`).
Files outside the Files block:
- `crates/epiora-core/Cargo.toml` — `proptest` 1.12 dev-dependency (in
  the plan's Tech Stack; needed by `tests/stats_props.rs`). Licence:
  MIT OR Apache-2.0, both on the `deny.toml` allow list.
- `Cargo.lock` — cargo-generated lockfile for the same addition.
- docs/decisions.md — this entry.
Supersedes: —

### v0.2 Task 7 — `stats`: stratified 2×2 (2026-11-03)

Decision: `stratified` is implemented as planned — MH OR with the
Robins–Breslow–Greenland CI, MH RR with the Greenland–Robins CI, the
uncorrected and Yates-corrected MH chi-squares, and Breslow–Day–Tarone
with df = kept strata − 1. Points resolved beyond the plan's prose:

- Breslow–Day's p-value is `1 − pchisq(X², df)` exactly as
  `DescTools::BreslowDayTest` computes it — the complement of the
  *lower-tail* CDF, not the upper tail `pchisq(..., lower.tail =
  FALSE)`. `special` therefore gained a port of R's lower-tail
  `pgamma_raw` (`inc_gamma_lower`, exposed as `chisq_cdf`): the
  "x large" region is verbatim so `1 − chisq_cdf` reproduces
  `1 − pchisq` including its underflow-to-0; `pgamma_smallx` maps to
  the equivalent A&S 6.5.29 convergent series and the `ppois_asymp`
  middle region (x within ~20% of a, reachable only for a ≳ 200 —
  outside the reference grid) falls back to `1 − inc_gamma_upper`.
  Checked against a 231-row R 4.6.2 `pchisq` grid written by the
  conductor (`target/conductor/chisq_cdf_ref.csv`, gitignored): max
  relative error 6.2e-14 on `pchisq`, 1.4e-15 on `1 − pchisq`.
Files outside the Files block:
- crates/epiora-core/src/special/gamma.rs — `inc_gamma_lower` plus the
  `dpois_raw`/`dpois_wrap`/`pd_upper_series`/`pd_lower_cf`/
  `pd_lower_series` helpers it needs; `breslow_day` must reproduce
  `1 − pchisq`, which `chisq_sf` cannot (it computes the upper tail
  directly).
- crates/epiora-core/src/special/chisq.rs — `chisq_cdf` wrapper.
- crates/epiora-core/src/special/mod.rs — re-export of `chisq_cdf`.
- crates/epiora-core/tests/special.rs — committed `chisq_cdf` unit
  tests (literals copied from the conductor's R grid, one per ported
  branch; the near-diagonal else point via the exact integer-shape
  Poisson-tail identity).
- docs/decisions.md — this entry.
Supersedes: —

### v0.2 Task 8 — `tabulate`, `procedures`, receipts, registry (2026-11-03)

Decision: `tabulate`, `procedures`, receipt v1, the validation registry
and `corpus-runner` are implemented as planned. Points resolved beyond
the plan's prose:

- `VarSpec.levels` is an orientation aid, not a filter: it must equal
  the variable's non-missing category set, else `InvalidOptions` —
  categories are never silently dropped.
- `AmbiguousLevels` fires only when the result would be a 2×2;
  orientation is a 2×2 concern (spec §3.3). A 2-level Text variable
  inside an R×C table keeps its category order, no error.
- `stratify_by` on a non-2×2 `tables` call is `InvalidOptions`
  ("stratify_by requires a two-by-two table") rather than a silently
  pooled R×C; the result type has no place for stratified R×C.
- `FreqTable.missing` still reports the missing count when
  `include_missing` adds the `"(missing)"` category (confirmed by
  `freq/missing_group_incl_missing`); `total` is the in-scope record
  count. `"(missing)"` is appended only when missing values exist.
- `FreqResult.exclusions` counts records excluded for a missing
  stratify variable; a record missing the analysed variable is reported
  in `missing`, not excluded. Excluded records are counted under every
  variable they miss.
- Combined stratum labels join per-variable labels with `,`;
  `FreqTable.stratum` is the stratum label. `Labels2x2` is
  `{exposure, outcome}` and `StratifiedResult` is
  `{strata_labels, stats}` (the plan left both unexpanded).
- Receipt `validation` entries carry the family-prefixed registry
  statistic id (the spec example shows the leaf path only); options
  serialize with explicit `null` for unset `Option` fields ("every
  default filled in").
- `serde_json` is now a required dependency (receipts are product
  code); the `corpus` feature only gates the corpus module. `ryu`
  added per Tech Stack (Apache-2.0/BSL-1.0, both on the allow list).
- `#[allow(clippy::large_enum_variant)]` on `TablesResult`: the
  plan's interface embeds two full stats structs in `TwoByTwo`, so
  boxing is declined to keep the produced signature exact.
- `corpus-runner` iterates the five family dirs in sorted path order;
  a fixture's declared `inputs.table`/`inputs.strata` is verified
  against tabulation (`CorpusFormat` on disagreement).
- `check-engine.sh` full mode gained the `corpus-runner` smoke run
  writing `target/corpus-native.jsonl` (gitignored).

Files outside the Files block:
- `crates/epiora-core/src/tabulate/categories.rs` — the category
  machinery (`Key`, `Cats`, `collect`, `resolved`, `orient`) split
  out of `tabulate.rs` to keep both files under ~400 lines (rule 8).
- `crates/epiora-core/src/corpus/mod.rs` — registers `run_case`
  (`pub mod run_case; pub use run_case::{run_case, CaseOutput};`) so
  `corpus-runner` and the corpus tests can drive fixtures.
- `crates/epiora-core/src/validation/registry/{mod,freq,rxc,
  stratified_two_by_two,two_by_two}.rs` — the registry entries moved
  into per-family submodules (each family's repeated fixture list is
  one shared `const`); required by the ~400-line constraint now that
  `validation.rs` held 1142 lines. `REGISTRY` is unchanged: same 86
  entries, ids, fixture lists, classes and order (verified by dumping
  every `statistic|fixtures|class` row before and after — the diff is
  empty — and by the unchanged golden receipt).
- `Cargo.lock` — cargo-generated lockfile for the `ryu` addition.
- `docs/decisions.md` — this entry.
Public items not in the plan's Interfaces block:
- `tabulate::frequency_stratified` — the shared per-stratum counting
  machinery behind `procedures::freq`'s `stratify_by`.
- `tabulate::StratifiedFreq` — `frequency_stratified`'s return type
  (strata labels + one `FreqCounts` per stratum + exclusions).
- `validation::statistic_ids` — enumerates the statistic leaf ids a
  serialized stats object emits; `tests/registry.rs` and the receipt
  `validation` block need it to assert every emitted statistic is
  registered.
Supersedes: —
### v0.2 Task 9 — Epi Info 7 method choices (2026-11-04)

Decision: `corpus::ei7::compare_ei7` compares all 56 mapped values of
`fixtures/ei7/oswego.json` against the engine's FREQ SEX, TABLES
VANILLA × ILL and TABLES VANILLA × ILL STRATAVAR=SEX output at the
`ei7_display` rule (`|engine − ei7| ≤ 0.5 × 10^−d`); every one passes
(`tests/ei7.rs::oswego_matches_epi_info_7`, `checked = 56`).

- **FREQ proportion CI is Clopper–Pearson.** Epi Info 7 prints "Exact
  95% Conf Limits" for FREQ; the mapped `ci_95_clopper_pearson` bounds
  (46.70–69.92 / 30.08–53.30) match the engine at display precision,
  while the Wilson bounds (47.37–69.12 / 30.88–52.63) would not.
  `ProportionCi`'s default stays `ClopperPearson`; `freq.rs` is
  unchanged.
- **z-based 95% intervals: engine keeps qnorm(0.975); the comparison
  rescales to z = 1.96** (owner decision, PR #18). Epi Info builds its
  z-based intervals with z = 1.96 exactly; rescaling the engine bounds
  reproduces every printed digit:

  | Interval | Corpus (qnorm) | Rescaled to 1.96 | Epi Info 7 |
  |---|---|---|---|
  | OR Taylor (`two_by_two.odds_ratio.ci_95_wald`) | 5.841191–94.178692 | 5.8410–94.1811 | 5.8410–94.1811 |
  | RR Taylor (`two_by_two.risk_ratio.ci_95_wald`) | 1.938340–16.029337 | 1.9383–16.0296 | 1.9383–16.0296 |
  | MH RR (`stratified_two_by_two.mantel_haenszel.risk_ratio.ci_95`) | 1.986792–16.512399 | 1.9868–16.5127 | 1.9868–16.5127 |
  | MH OR RGB (`stratified_two_by_two.mantel_haenszel.odds_ratio.ci_95`) | 7.191633–197.000244 | 7.1914–197.0062 | 7.1914–197.0062 |

  The conversion lives only in `corpus/ei7.rs` (`rescale_z`), never in
  `stats` or `procedures`; the engine has no z/confidence-level option.
  Ratio intervals rescale on the log scale (`c = √(lo·hi)`,
  `se = (ln hi − ln lo)/(2·qnorm(0.975))`, `c·e^∓1.96·se`); the risk
  difference interval rescales additively (`m ∓ 1.96·se`). It applies
  to the bounds of: `two_by_two.odds_ratio.ci_95_wald`,
  `two_by_two.risk_ratio.ci_95_wald`,
  `two_by_two.risk.difference_ci_95_wald`,
  `stratified_two_by_two.crude.odds_ratio.ci_95_wald`,
  `stratified_two_by_two.crude.risk_ratio.ci_95_wald`,
  `stratified_two_by_two.mantel_haenszel.odds_ratio.ci_95` and
  `stratified_two_by_two.mantel_haenszel.risk_ratio.ci_95` — i.e. keys
  ending `ci_95_wald.lower|upper` plus `mantel_haenszel.*.ci_95.*`. No
  Wilson proportion CI is mapped (FREQ prints exact limits), so none is
  converted. Point estimates, p-values, chi-squares and Fisher/mid-p
  exact limits are compared unconverted; the display rule itself is
  unchanged.
- **Percent scale:** Epi Info prints `Risk Difference (RD%)` and its
  limits as percents while the corpus `risk.difference` leaves are
  0–1, so `two_by_two.risk.difference` and
  `two_by_two.risk.difference_ci_95_wald.{lower,upper}` are compared as
  `engine × 100`. FREQ percents and the Clopper–Pearson limits are
  already 0–100 in the engine; no other label needed scaling.
- **`Fisher exact 1-tailed p` → `fisher_exact.p_upper`:** Epi Info
  prints the tail in the direction of the association (upper for
  Oswego, OR > 1). Unverified for a negative association.
- **Unmapped statistics are "not in v0.2"** (from Task 3's
  `labels.json`): every crosstab cell count and Row%/Col%, `Total
  Percent`/`Total Cum. Percent`, `Mid-p exact 1-tailed p`, all
  per-stratum single-table statistics under `SEX=Female`/`SEX=Male`,
  and `Adjusted OR (MLE)` with its CI. Values Epi Info printed as
  `Undefined` or `NaN` emit no number and are never compared.
Supersedes: —

### v0.2 Task 10 — epiora-wasm binding, size gate, parity (2026-11-04)

Decision: The spec §5.1 binding is implemented in `epiora-wasm` as type
conversion only: `datasetFromColumns` decodes the §5.1 typed-array
encoding (Integer `BigInt64Array` + `Uint8Array` presence, Real
`Float64Array` + mask, Date `Int32Array` + mask, Text dictionary +
`Uint32Array` codes with `0xFFFFFFFF` = missing, Boolean `Uint8Array`
0/1/255) into `Column`s and `Dataset::new` does the validation;
`datasetHash`, `freq`, `tables`, `engineVersion` and the `parity`
feature's `runCorpusCase` wrap the core calls. `EngineError` crosses
as a JS `Error` carrying `code` and `message`; results serialize via
`serde_wasm_bindgen::Serializer::json_compatible` (u64 → number).
Native/wasm parity (spec §5.2) is bit-identical: 29 `parity_line`s,
`parity/diff.sh` clean (measured: `diff` of
`target/corpus-native.jsonl` vs `target/corpus-wasm.jsonl`, no
differences). The size gate passes at **130,445 bytes** after
`gzip -9` (`scripts/wasm-size.sh crates/epiora-wasm/pkg/
epiora_wasm_bg.wasm`, `wasm-opt -Oz`; budget 409,600).
Tooling pins (each download SHA-256-verified in `engine.yml`; licences
as the downloads state): wasm-pack 0.15.1 (MIT/Apache-2.0 —
`LICENSE-MIT` + `LICENSE-APACHE` in the tarball; 0.15.1 is the latest
release, sha
`410101009739b9d01c7d5affaaad6e9c9a24b4cba6791a4805db8e89b9bf7af2`),
binaryen `version_133` wasm-opt (Apache-2.0 —
`binaryen-version_133/include/binaryen-c.h` line 4 of the downloaded
tarball states "Licensed under the Apache License, Version 2.0";
version_133 is the latest release, sha
`2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489`,
matching upstream's published `.sha256`), wasm-bindgen CLI 0.2.129
(MIT/Apache-2.0 in the tarball, sha
`82d12bb940e2d4e72e0d5605387fc1b8ca179044e012b620f0ce4e7440e8320e`;
0.2.130 was only 5 days old, so the crate and CLI pin to the older
`0.2.129`, found by wasm-pack's `which` check under `--mode
no-install`). `rustup target add wasm32-unknown-unknown` supplies the
target in CI (no `rust-toolchain.toml` change). `actions/setup-node`
v7.1.0 and `actions/upload-artifact` v7.0.2 are pinned by commit SHA;
Node 22 per the Tech Stack. Direct crate dependencies this task adds
(`license` field from `cargo metadata --format-version 1 --locked
--all-features`): ts-rs 12.0.1 (MIT), wasm-bindgen 0.2.129 (MIT OR
Apache-2.0), js-sys 0.3.106 (MIT OR Apache-2.0), serde-wasm-bindgen
0.6.5 (MIT).
Interface notes: `Deserialize` and `#[serde(default)]` were added to
the option types (`FreqOptions.include_missing`/`ci_method`,
`TablesOptions.stratify_by`/`include_missing`, and `#[serde(default)]`
on the two `skip_serializing_if` `Option` fields so ts-rs emits `?`),
since the binding takes options as JS objects — wire defaults are
type conversion, not branching on data. `datasetFromColumns`'s
`version: number` is validated as a non-negative integer
(`InvalidOptions`). `runCorpusCase(fixtureJson, datasetsJson)` takes
the datasets as one JSON object keyed by id holding the
`*.dataset.json` documents. `Estimate`'s exported TS type is the
literal union its custom serializer emits (`number | { "undefined":
<reason codes> }`), not the derive's enum shape.
Evidence: `./scripts/check-engine.sh full` locally — all Rust tests
(37 export_bindings included), 7/7 node binding tests, 29-line parity
diff clean, size 130,445; `bats scripts/tests` (incl. wasm-size and
parity-diff, 4/4).
Files outside the Files block:
- .cargo/config.toml — `[env]` pins `TS_RS_EXPORT_DIR` to
  crates/epiora-wasm/bindings and `TS_RS_LARGE_INT=number`, so every
  `cargo test` run regenerates into the committed directory instead of
  ts-rs's default `crates/epiora-core/bindings`, and u64 fields map to
  `number` as the JSON emits.
- Cargo.lock — cargo-generated lockfile for the new dependencies.
- crates/epiora-core/src/{dataset/mod.rs, procedures/mod.rs,
  receipt/mod.rs, validation.rs, stats/{breslow_day, estimate,
  proportion, rxc, stratified, two_by_two}.rs} — the Files block names
  only Cargo.toml, but its "(feature `ts` → `ts-rs` derives on the
  option, result and receipt types)" change lands the `#[cfg_attr(
  feature = "ts", derive(ts_rs::TS), ts(export))]` on those
  type-definition files, plus `Deserialize`/`#[serde(default)]` on the
  option types named above.
- docs/decisions.md — this entry.
Supersedes: —

### v0.2 Task 11b — window every exact-method pass (2026-11-04)

Decision: `NcHypergeom::weights` now stores only the mass window —
`max` is found over the full support without storing, the window is
the `[first, last]` support-index range whose `v − max ≥ UNDERFLOW`
(scanned from both ends, not assumed contiguous), and only
`first..end` is stored, summed (left to right from `0.0`) and
divided, Task 11a's per-term rule unchanged. `pmf`, `mean` and
`tail_sum` read the window and treat out-of-window terms as `+0.0`;
an empty selected range keeps the `f64::sum` seed `-0.0`. `midp` and
`fisher_p_two_sided` compute both tails from one window through the
private `pub(crate)` helper `tails` — each tail the identical `f64`
`lower_tail`/`upper_tail` returns. `weights_probe` is now a 3-tuple
(pre-change full-support reference, `libm::exp` evals, stored
count); no public signature changed. Bit-identity verified:
`crates/epiora-wasm/parity/diff.sh target/baseline.jsonl
target/after.jsonl` — no difference (29 lines).
Files outside the Files block:
- crates/epiora-core/src/special/hypergeom/probe.rs — new private
  submodule holding `weights_full` (the bit-identity reference) and
  `weights_probe`, moved out of `hypergeom.rs` to keep it under the
  ~400-line rule, per the task's explicit escape hatch.
- docs/decisions.md — this entry.
Supersedes: —
