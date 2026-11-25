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
### v0.2 Task 11 — Chromium performance gate (2026-11-04)

Decision: — (gate task; no plan decision rule). Adds the spec §5.3
Chromium benchmark under `crates/epiora-wasm/bench/` (Playwright
1.64.1 exact, committed `package-lock.json`; Chromium pinned by the
Playwright version), the native `criterion` report
`crates/epiora-core/benches/tables_1m.rs` (same SplitMix32, seed
20261031 and draw order as `gen.mjs`; first-3 EXPOSED pin [1, 1, 1]
re-derived by running `node -e 'import("./gen.mjs").then(({generate})
=> console.log([...generate(1000, 20261031)[0].values.slice(0,3)]))'`
→ [ 1, 1, 1 ]), and the `bench` job in `engine.yml` (ubuntu-latest;
median of 5 runs must be < 1000 ms; `bench-results.json` uploaded as
an artifact and the median written to the job summary; the native
criterion run is reported, not gated).
Interface notes: `scripts/check-engine.sh` already had the `E2E=1`
hook, so it is unchanged. The timed region is exactly
`datasetFromColumns` + `tables(EXPOSED × ILL by REGION)`; 1M-row page
generation is untimed. `.gitignore` already covered
`bench/node_modules` via `node_modules/`; the edit adds only the
generated `bench-results.json`.
Dependency licences: `@playwright/test` 1.64.1 (Apache-2.0, from
package-lock.json `license` fields), criterion 0.8.2 (MIT OR
Apache-2.0, `cargo info`).
Evidence: `E2E=1 ./scripts/check-engine.sh full` locally — see the
task's PR for the CI bench run and its `bench-results.json` artifact;
native `cargo bench -p epiora-core --bench tables_1m` reported in the
PR body.
Files outside the Files block:
- scripts/bench.sh — check-engine.sh's E2E hook invokes
  `scripts/bench.sh`; the Files block names the bench/ contents but no
  runner script, so the E2E entry point lives here (builds the release
  pkg, `npm ci`, `playwright install --with-deps chromium` under CI,
  `npx playwright test`).
- crates/epiora-wasm/bench/serve.mjs — the static file server
  Playwright's `webServer` runs; Node has no built-in static-server
  CLI and no extra dependency was wanted, so the bench serves
  crates/epiora-wasm itself (/pkg + /bench) with fixed MIME types.
- Cargo.lock — cargo-generated lockfile for the new criterion dev
  dependency.
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

### v0.2 Task 12 — fuzzing (2026-11-04)

Decision: `fuzz_harness` (feature `fuzzing` = `corpus` + optional
`arbitrary`, MIT OR Apache-2.0) holds the four `pub fn(&[u8])` entry
points — `dataset_json` (corpus `load_dataset`), `dataset_columns`
(arbitrary typed columns into `Dataset::new`), `freq_any` and
`tables_any` (arbitrary dataset + options). Inputs decode with
`arbitrary` 1.5.0 through hand-written `Arbitrary` impls (the
`derive` feature's `derive_arbitrary` was not required); decode
failures return early. Variable names are picked by index from the
generated columns; every selector divisible by 10 names
`__fuzz_unknown_<sel>` instead (≈10 %, from the input bytes — no RNG,
no clock). The only permitted panic is `assert_no_null`'s scoped
`#[allow(clippy::panic)]` — the crate's panic deny stays in force
everywhere else. A serialized `null` would mean a non-finite output
(serde_json writes non-finite `f64` as `null`) or a broken Task 8
no-`null` contract; the check walks `out.result` only — the receipt
legitimately serialises `levels: null`. Dataset targets instead check
that no `Real` column cell serialises as `null` (a leaked non-finite
`f64` past `Dataset::new`).
Files outside the Files block:
- crates/epiora-core/fuzz/rust-toolchain.toml — Step 3 requires the
  dated-nightly pin (`nightly-2026-10-15`); both fuzz workflows install
  exactly this channel.
- crates/epiora-core/fuzz/.gitignore — keeps cargo-fuzz's `target/`,
  `corpus/` and `artifacts/` output out of git; `regressions/` stays
  tracked.
- crates/epiora-core/fuzz/Cargo.lock — locks libfuzzer-sys 0.4.13, cc
  and arbitrary for reproducible CI fuzz builds.
- Cargo.lock — cargo-generated lockfile for the new optional
  `arbitrary` dependency.
- docs/decisions.md — this entry.
Supersedes: —

### v0.2 Task 13 — v0.2 closed (2026-11-05)

Decision: v0.2 is closed. `docs/validation/v0.2.md` is the byte-identical
`v0.2-validation-md` artifact of `engine.yml` run
https://github.com/dorrabbkhan/Epiora/actions/runs/42013745091
(head `56c430d`, sha256 `2cce885175dda1267612622379faeadbf4417aebb9a4d50606ffc29b37b21a97`):
86 rows, one per `validation::REGISTRY` statistic, every worst observed
relative error within its class tolerance, measured by running the corpus
through `run_case`; plus the Epi Info 7 check — 56 mapped values, 0
mismatches at `ei7_display`. Its header names that commit and run URL and
carries the measured optimised `.wasm` size **132,158 bytes** after
`gzip -9` (E3 budget 409,600) and the Chromium bench median **551.6 ms**
on 1,000,000 rows (E4 budget < 1000 ms), produced by the `engine`/`bench`
jobs of the same run and handed to the bin as job outputs via
`EPIORA_WASM_GZIP_BYTES` / `EPIORA_BENCH_MEDIAN_MS`.

The owner decisions E1–E8 (v0.2 spec §2) are realised: E1 by `special`
hand-written on `libm` (and the platform A8/§7.4 edits applied here), E2 by
`corpus::ei7::compare_ei7` over `fixtures/ei7/oswego.json`, E3 by
`scripts/wasm-size.sh` and the size gate, E4 by `crates/epiora-wasm/bench`
and the `bench` job, E5 by the tight `uniroot(tol = 1e-14)` roots in
`fixtures/R/nchyper.R` with `r_default` cross-checks, E6 by the
`tabulate`/`stats`/`procedures` split, E7 by `corpus::compare` +
`ToleranceClass::passes`, E8 by `spikes/webr-mobile/measure.mjs` reading
v1 and v2 fixture shapes.

The four platform-spec edits of v0.2 spec §11 are applied verbatim in this
PR (OWNER GATE — the owner approves the spec edits): §6 row A8, §7.4,
§12 item 1 and §12 item 3; every changed sentence is listed in the
`docs: close v0.2 engine core` commit message. The `E1`/`E5` tags are the
v0.2 spec's decision citations, not platform-spec prose, so the added
sentences carry them only where §11's quoted wording does.

Files outside the Files block:
- crates/epiora-core/Cargo.toml — registers the `validation-report` bin
  (`required-features = ["corpus"]`, same pattern as `corpus-runner`).
- crates/epiora-core/src/corpus/mod.rs — `pub mod report;` registration.
- crates/epiora-core/src/corpus/report/mod.rs and report/ei7.rs — the
  report machinery the bin and the test share (`build_rows` must be
  library-visible so `validation_report_has_every_registry_row` can assert
  on rows measured by actually running the corpus); split in two for the
  ~400-line rule. The Files block names only the bin.
- crates/epiora-core/tests/validation_report.rs — the Step-1 test's home;
  the Files block names no test file.
- docs/decisions.md — this entry.
Supersedes: —
### v0.3 Task 1 — Snapshot codec and receipt project stamp (2026-11-05)

Decision: `epiora-core` gains `snapshot.rs` — the columnar snapshot codec
of v0.3 spec §5.1 (`EPSN` magic, format 1, strict canonical decoding so
every accepted input re-encodes byte-for-byte) — plus
`EngineError::SnapshotDecode` (`snapshot_decode`). `RunContext` gains
`project: Option<ProjectStamp>`; a run naming a project emits
`receipt_version` 2 (`RECEIPT_VERSION_WITH_PROJECT`) with the
`"project": {"id", "name"}` key inside `receipt_sha256`, while
project-less runs keep byte-identical receipt v1 (v0.3 spec §8.2, F12).
`epiora-wasm` gains `datasetFromSnapshot`/`datasetToSnapshot`.
Evidence: crates/epiora-core/tests/snapshot.rs, tests/receipt_project.rs,
crates/epiora-wasm/tests/node/binding.test.mjs
Files outside the Files block:
- crates/epiora-core/benches/tables_1m.rs, src/corpus/report/ei7.rs,
  src/corpus/run_case.rs, src/fuzz_harness.rs — `project: None` added to
  `RunContext` struct literals so they still compile.
- crates/epiora-core/tests/{ei7,procedures,receipt,registry,tables_reuse,tabulate}.rs
  — same `project: None` literal fix in test helpers.
- crates/epiora-core/src/bin/corpus-runner.rs — Step 5's
  `--snapshot-hex <dataset-id>` flag printing the native
  `encode_snapshot` hex the Node binding test compares against.
- scripts/check-engine.sh — exports `NATIVE_OSWEGO_SNAPSHOT_HEX` from
  that flag into the node binding test step.
- docs/decisions.md — this entry.
Supersedes: —
### v0.3 Task 2 — import fixture corpus (2026-11-06)

Decision: `fixtures/R/import.R` generates the 35-case import corpus of
v0.3 spec §6.5 under `fixtures/import/<case>/` (input file plus
`expected.json` per `fixtures/schema/import.schema.json`); `generate.R`
sources it last. Case semantics follow spec §6.1–6.2 literally:

- `missing_codes` replaces the whole default list: with `["-99"]`, "NA"
  and "." stay literal values (only "" and "-99" are missing).
- A column with zero non-missing values (header_only, ALLMISS in
  missing_codes_default) fits every candidate, so the first candidate
  wins: `boolean`.
- Duplicate/empty header renames are positional-occurrence:
  `x,x,,x` -> `x`, `x_2`, `column_3`, `x_3`.
- In xlsx fixtures a missing cell is a genuinely empty cell
  (openxlsx2's NA -> t="e" #N/A is patched out); the real error cell is
  reserved for xlsx_cell_error, which counts it in coercions.
- `coercions` names every column, including zeros.

Byte-stability (plan Step 2): openxlsx2 is called with
`creator = "epiora"` (otherwise the local username leaks into
`dc:creator`/`lastModifiedBy`) and `datetime_created = 2026-01-01
UTC`. Every xlsx/ods container is rebuilt by `repack_zip()` —
unzip, patch if needed, rewrite through `write_stored_zip()`:
every member stored uncompressed (deflate implementations could
differ between machines), no extra fields, DOS time/date fixed at
2026-01-01 00:00, UTF-8-name flag, mode-0644 attributes, canonical
member order (`[Content_Types].xml`/`mimetype` first, the rest
sorted). After that only member contents can differ.

LibreOffice output differs between releases and builds, so the two
LO-produced files (xls_basic's input.xls; ods_dates' input.ods) are
made by the pinned TDF build both locally and in CI:
`LibreOffice_26.2.6.3_Linux_x86-64_deb.tar.gz` from the permanent
TDF archive (`downloadarchive.documentfoundation.org/libreoffice/old/`),
SHA-256 `fd0e8f8f2408dd2e5b90286e60f3f97cf566ba441cd48cfc5bcc68067303e0bc`,
downloaded and hash-verified by fixtures.yml (the distro
`libreoffice-calc-nogui` install was dropped) and extracted with
`dpkg -x` — `soffice --version` fingerprint `LibreOffice 26.2.6.3
8221e31b3ac356a1623c672912a3d2b492f7e3d1`, checked by `lo_soffice()`
against `EPIORA_SOFFICE` so no other build can generate. Conversions
run under `SOURCE_DATE_EPOCH=0` and `TZ=UTC` (doc metadata and OLE
timestamps otherwise follow wall clock and local timezone), a
private UserInstallation profile seeded with `givenname = "epiora"`
(the OS username otherwise lands in dc:creator /
meta:initial-creator and in the .xls Author and WriteAccess
records), `env -u LD_LIBRARY_PATH` (the LD_LIBRARY_PATH R exports
for its own libs makes soffice.bin fail to find libreglo.so), and a
`FONTCONFIG_FILE` confined to the build's bundled fonts (document
font tables, default column widths and the rendered thumbnail
otherwise follow the host's installed fonts — verified to change
content.xml/styles.xml between hosts). For the LO-produced ods,
`normalize_ods_meta()` rewrites meta.xml generator/creators/dates to
fixed values so the file carries no build or user identity.
`xlsx_1904_dates` writes serials via openxlsx2's 1904-epoch
converter and patches `workbookPr date1904="1"` into workbook.xml.
`xlsx_formula_cached` and `xlsx_cell_error` patch cached `<v>`
results / a t="e" #N/A cell into the sheet XML (openxlsx2 writes
neither). `ods_dates` goes through LibreOffice because `write_ods`
stores dates and booleans as strings; `ods_basic` uses `write_ods`
directly. Every case directory is unlinked before the case is
written and each `file.copy` is asserted with `overwrite = TRUE`, so
a stale artifact can never survive regeneration (the previous
version copied without `overwrite`, leaving pre-repack LO output
committed).

Versions and licences (AGENTS.md rule 6): R 4.6.2; openxlsx2 1.30
(MIT), readODS 2.3.6 (GPL-3, fixture-generation only — never linked
into product code), minty 0.0.6 (MIT, readODS dependency) — all CRAN,
pinned in fixtures/renv.lock; LibreOffice 26.2.6.3 TDF build locally
and in CI (MPL-2.0, per the `LICENSE` file in the downloaded
tarball).
Evidence: fixtures/import/ (35 cases) regenerated from
`rm -rf fixtures/import`; `Rscript fixtures/generate.R` run twice —
`git status --porcelain fixtures/` showed no regeneration drift
after the second run; the committed input.xls and every
ods_dates/input.ods member verified byte-identical against a
conversion of the same inputs inside a minimal ubuntu:24.04
container (different user, timezone and font environment).
Files outside the Files block: fixtures/R/import_zip.R — the zip
container writers and the LibreOffice bridge, split out of
fixtures/R/import.R to keep both files under the ~400-line global
constraint; generated output is unchanged by the split.
Supersedes: —
### v0.3 Task 3 — epiora-import: text formats, detection, inference, coercion (2026-11-06)

Decision: `crates/epiora-import` implements spec §6.1–§6.3 for text
containers: container detection by file magic, encoding by BOM/UTF-8
validity (Windows-1252 fallback), delimiter by quote-aware modal field
consistency over the first 64 KiB of decoded text (ties `,` `;` tab
`|`, no qualifier → `,`), decimal `comma` iff delimiter `;` plus the
spec's two fixed-decimal patterns, `encoding_rs` decoding (UTF-8
borrowed, one leading U+FEFF stripped, strict under an explicit
`settings.encoding` → `encoding_invalid`), `csv` `flexible(false)`
reading (`RaggedRow` at the record's 1-based physical line, blank
lines skipped, `MAX_COLUMNS`/`MAX_ROWS` enforced), §6.2 inference
(Boolean → Integer → Real → Date → Text, first fit wins) shared with
the coercion predicates, `column_types` overrides counted in
`coercions`, and `source_sha256`. Interpretations where the spec/plan
leave room:

- `Detection` and the returned `settings` report the *effective*
  reading: user overrides fill `encoding`/`delimiter`/`decimal`, so a
  saved `Imported.settings` re-imports deterministically.
- The `.tsv` extension changes nothing observable: the plan's fallback
  rule ("no candidate qualifies → `,`") is absolute, so `file_name` is
  currently unused beyond the API signature.
- An all-missing column infers `boolean` (spec §6.2's first-fit rule;
  the owner-reviewed fixtures `missing_codes_default`/`header_only`
  agree). The plan's test list named `all_missing_is_text` — written
  before Task 2 settled the corpus semantics — so the test ships as
  `all_missing_is_boolean`.
- `""` (after ASCII trim) is always missing in addition to
  `missing_codes`; a custom `missing_codes` list replaces the default
  (Task 2's recorded semantics).
- `dmy`/`mdy` require one consistent `/`, `-` or `.` separator and do
  not also accept ISO dates; `iso` accepts only `YYYY-MM-DD`.
- A real literal whose `f64` parse is non-finite (`1e999`) does not fit
  Real → Text (or missing under an override), matching §6.2's
  "inf/nan are not reals" intent and `Dataset::new`'s finite rule.
- `settings.sheet` on a text file is `sheet_not_found`; `key:
  {column: …}` and `column_types` keys naming an absent column are
  `unknown_column`.
- Zip containers are reported `xlsx` provisionally; any non-text
  container is a `spreadsheet` error until Task 4.
- `sha2` is `0.11` (latest at task time; `epiora-core` remains 0.10 —
  a duplicate-version warning only, `bans ok`).

Dependencies and licences (AGENTS.md rule 6): csv 1.4.0 (Unlicense OR
MIT), csv-core 0.1.13 (MIT OR Apache-2.0), encoding_rs 0.8.44
((Apache-2.0 OR MIT) AND BSD-3-Clause — BSD-3-Clause added to
`deny.toml`, the only licence `cargo deny` reported), sha2 0.11.1 (MIT
OR Apache-2.0), serde 1.0.229 + serde_json 1.0.154 (dev; MIT OR
Apache-2.0), thiserror 2.0.21 (MIT OR Apache-2.0), ts-rs 12.0.1 (MIT),
arbitrary 1.5.0 + derive_arbitrary 1.5.0 (MIT OR Apache-2.0),
libfuzzer-sys 0.4.13 ((MIT OR Apache-2.0) AND NCSA; fuzz crate only,
outside the deny-checked workspace).
Evidence: `cargo test -p epiora-import` — 19 tests green including
`every_text_fixture_matches` over all 26 non-spreadsheet fixtures;
`./scripts/check.sh` full PASS; 60 s local fuzz: `cd
crates/epiora-import/fuzz && cargo +nightly-2026-10-15 fuzz run
text_import -- -max_total_time=60` → "Done 685581 runs in 61
second(s)", no crash.
Files outside the Files block:
- crates/epiora-core/src/dataset/mod.rs — `ValueType` gained
  `Serialize`/`Deserialize` (`snake_case`, the corpus spellings), a
  `ts_rs::TS` derive (feature `ts`, not `ts(export)` so the wasm
  bindings are unchanged) and an `arbitrary::Arbitrary` impl (feature
  `fuzzing`): `ImportSettings.column_types` is
  `BTreeMap<String, ValueType>` and must be serde+ts+fuzz-able. No
  behaviour change.
- Cargo.lock — workspace lock for the new member.
- .github/workflows/engine.yml — the PR fuzz job also runs
  `text_import` from `crates/epiora-import/fuzz` (60 s) and uploads its
  artifacts path (conductor note for Task 3).
- .github/workflows/fuzz-nightly.yml — new `fuzz-import` matrix job on
  the same pinned nightly (1800 s), mirroring the existing job.
- crates/epiora-import/fuzz/{.gitignore,rust-toolchain.toml,
  regressions/text_import/.gitkeep,Cargo.lock} — the fuzz crate's
  housekeeping files, mirroring `crates/epiora-core/fuzz`.
- docs/decisions.md — this entry.
Supersedes: —
### v0.3 Task 4 — epiora-import: spreadsheets via calamine (2026-11-06)

Decision: `epiora-import` reads xlsx, xls and ods through `calamine`
(spec §6.1.5, §6.2, F13). `inspect`/`import` route non-text containers
to `sheet.rs`, which turns the chosen sheet's used range into the same
`Table` of candidate strings that text files produce, plus a
`forced_coerced` cell set; §6.2 inference and coercion then run
unchanged. `excel_date.rs` carries `serial_to_days` and the inverse
days → `YYYY-MM-DD` formatter. Interpretations where the spec/plan
leave room:

- Container detection: an ODF zip is recognised by its first member —
  `mimetype` stored with content
  `application/vnd.oasis.opendocument.spreadsheet` (the ODF-required
  form); every other zip is `xlsx`, and a zip that is neither fails
  inside calamine as `spreadsheet` (`[Content_Types].xml` is not
  separately verified — calamine is the arbiter).
- Cell mapping: integral numerics within `]-2^63, 2^63[` emit integer
  candidate text, other finite numerics the shortest round-trip
  `f64` text; `Data::Bool` emits `true`/`false`; `Data::DateTime`
  (`is_datetime`) goes through `serial_to_days` and emits ISO text —
  a `None` (serial 60, non-zero time part) is `Forced`; non-`datetime`
  `ExcelDateTime` (durations) and `DateTimeIso`/`DurationIso` emit
  their value as text; `Data::Error` and non-finite floats are
  `Forced`. `Forced` cells skip inference, coerce to missing under any
  chosen type and count in `coercions`, never in `missing`.
- Spreadsheet headers: an empty or absent header cell takes the Excel
  column letter (`A`, `B`, …, `AA`) — the column's own name — and is
  not recorded in `header_renames`; `xlsx_formula_cached` (no `C1`
  cell) fixes this reading. Non-empty duplicates uniquify and report
  exactly like text. With `header: false` the columns are the letters.
- ODS normalisation: calamine's ODS `read_row` rejects whitespace-only
  text between cells, which readODS's pretty-printed `content.xml`
  carries, so ODS input is repacked (`ods_bytes`) with whitespace-only
  text dropped at row/cell depth before calamine sees it. The repack
  streams: members other than `content.xml` are copied compressed via
  `ZipWriter::raw_copy_file` (bytes and method preserved, so `mimetype`
  stays first and stored), and `content.xml` is read with
  `quick_xml::Reader::from_reader` + `read_event_into` and written
  event by event into a Deflated entry — no member is decompressed
  into a whole-file buffer, so a highly compressible archive cannot
  OOM the process (review fix, PR #39). The repack feeds calamine
  only; `source_sha256` covers the raw input bytes.
- `Detection` for sheets reports `encoding`/`delimiter` `None`,
  `decimal` `dot` (or an override, which still applies to string
  cells), `sheets` in workbook order; the effective `settings` record
  `decimal` and `sheet: Some(<used>)`, so a saved re-import is
  deterministic.
- calamine's `dates`/`chrono` features are NOT enabled — serials are
  converted by `serial_to_days`, so no clock/locale reads enter the
  crate (spec §6.3, conductor note).
- `worksheet_range` materialises calamine's `Range` before
  `MAX_COLUMNS`/`MAX_ROWS` are checked; a hostile declared span could
  over-allocate inside calamine — noted as a follow-up, not solvable
  at our layer today.

Dependencies and licences (AGENTS.md rule 6): calamine 0.38.0 (MIT);
new transitives atoi_simd 0.18.1 (MIT OR Apache-2.0), byteorder 1.5.0
(Unlicense OR MIT), codepage 0.1.3 (Apache-2.0 OR MIT), crc32fast
1.5.4 (MIT OR Apache-2.0), debug_unsafe 0.1.4 (MIT OR Apache-2.0),
fast-float2 0.2.4 (MIT OR Apache-2.0), flate2 1.1.11 (MIT OR
Apache-2.0), log 0.4.35 (MIT OR Apache-2.0), nix 0.31.3 (MIT), nix's
cfg_aliases/bitflags already allowed, simd-adler32 0.3.10 (MIT),
typed-path 0.13.0 (MIT OR Apache-2.0), zlib-rs 0.6.8 (**Zlib** — added
to `deny.toml`, the only licence `cargo deny` reported), zopfli 0.8.4
(Apache-2.0); direct deps quick-xml 0.42.0 (MIT) and zip 9.4.1 (MIT)
for `ods_bytes` — both already in the tree via calamine at the same
versions.
Evidence: `cargo test -p epiora-import` — 27 tests green including
`every_spreadsheet_fixture_matches` over all 9 `xlsx_*`/`xls_*`/`ods_*`
fixtures; `./scripts/check.sh quick` PASS (`cargo deny` run as
`target/bin/cargo-deny --offline check` → "advisories ok, bans ok,
licenses ok, sources ok"); 60 s local fuzz:
`cd crates/epiora-import/fuzz && CARGO_NET_OFFLINE=true
cargo +nightly-2026-10-15 fuzz run sheet_import --
-max_total_time=60` → "Done 589446 runs in 61 second(s)", no crash.
Files outside the Files block:
- crates/epiora-import/src/{table,coerce,text}.rs — the `forced_coerced`
  cell set lives on `Table` (spreadsheet error cells and unconvertible
  date serials must count as coercions, which the string pipeline
  cannot express), `coerce_column` takes the `Table` to honour it, and
  `text::parse` initialises it empty; `table.rs` also gains
  `sheet_names`/`column_letter` for the spreadsheet header rule.
- crates/epiora-import/fuzz/Cargo.toml — `[[bin]]` registration of the
  new `sheet_import` target.
- Cargo.lock — workspace lock for the new dependencies.
- crates/epiora-import/fuzz/Cargo.lock — fuzz crate lock, same reason.
- .github/workflows/engine.yml — the PR fuzz job also runs
  `sheet_import` (60 s), per the conductor's Task 4 note.
- .github/workflows/fuzz-nightly.yml — `sheet_import` added to the
  `fuzz-import` matrix (1800 s), per the conductor's Task 4 note.
- docs/decisions.md — this entry.
Supersedes: —
### v0.3 Task 5 — record keys, record JSON and re-import diff (2026-11-07)

Decision: `epiora-import` gains `keys.rs` (`record_ids`,
`key_candidates`, `KeyCandidate`), `records.rs` (`record_json`,
`dataset_from_records`) and `diff.rs` (`diff`, `summarize`, `Diff`,
`FieldChange`, `SchemaChanges`, `TypeChange`, `DiffSummary`) per spec
§4.2/§4.4/§6.4. Interpretations where the spec/plan leave room:

- `record_json` emits `serde_json::Map` output — key order is UTF-8
  byte order — and `-0.0` is normalised to `0.0` (same rule as the
  snapshot codec), so stored JSON and rebuilt datasets hash alike.
  A row index past the end serialises as `{}` (no input may panic).
- `record_ids` on `RecordKey::Column` names an absent column →
  `unknown_column`; a missing cell → `missing_key` and a repeated id →
  `duplicate_key`, both with 1-based data lines as the plan's
  `first_line`/`line`. `RowPosition` renders `row:<1-based>`.
- `key_candidates` still counts `duplicates`/`missing` over rendered
  ids for `Real`/`Boolean` columns; `usable` additionally requires a
  keyable type and zero counts.
- `dataset_from_records` treats a field absent from a row's JSON as
  missing (schema additions rebuild over old rows); a value whose
  variant does not match the schema type — or any malformed row — is
  `invalid_settings` (the closest existing code; no dedicated variant
  exists) naming the 1-based record. Matching is strict: no
  integer→real widening.
- `diff` emits `FieldChange`s for shared columns whose `Value`s differ
  (variant-aware, so a type-changed column reports its populated rows
  even when the rendered value looks the same — the stored JSON must be
  rewritten, spec §4.2 `update` entries) and for added columns' non-
  missing cells (`old: None`); removed columns are reported once in
  `schema.removed_columns` instead of per row. `FieldChange.row` is the
  row index in `next`; `added` indices and `changed` order follow
  `next`'s row order, `removed` ids `previous`'s. `unchanged` is the
  records-model notion — re-ordered rows under a key column are not a
  change.
- `diff` relies on `Imported`'s unique-column-name invariant (the
  header-renaming step guarantees it); the `diff_any` harness enforces
  it when it builds arbitrary `Imported`s. The harness lives in the
  private submodule `src/diff/fuzz.rs` so `diff.rs` stays under the
  ~400-line file rule.
- The apply-property checkers (the proptest's `apply_diff` and the
  fuzzer's `check_apply`) rebuild `next`'s records map from
  `previous`'s `record_id → record JSON` map via the diff alone:
  `removed` ids must exist before deletion, `removed_columns` and
  `changed` apply to the survivors (a `FieldChange` naming a
  non-survivor fails), each `added` index's id — read from `next` —
  must be new before its record is inserted, columns `next` still
  lacks get nulls, and the whole map then equals `next`'s records map.
- `Diff`/`FieldChange`/`SchemaChanges`/`TypeChange`/`DiffSummary`/
  `KeyCandidate` derive `Serialize`/`Deserialize` and `ts_rs::TS`
  (feature `ts`) for the Task 6 wasm surface; no `ts(export)` yet.

Dependencies and licences (AGENTS.md rule 6): serde_json 1.0.154 moved
from dev-dependency to dependency (MIT OR Apache-2.0; already in the
tree via epiora-core — needed by `record_json`/`dataset_from_records`
in `src/`; `float_roundtrip` keeps f64 reads correctly rounded);
proptest 1.12.0 added as dev-dependency for
`proptest_diff_applied_to_previous_gives_next` (MIT OR Apache-2.0;
already in the tree via epiora-core's tests; plan Tech Stack lists
`proptest`). No new transitive crates.
Evidence: `cargo test -p epiora-import --all-features` — 44 tests green
including `reimport_with_new_duplicate_key_is_refused` (Review Focus 1)
and the apply-property proptest; `./scripts/check.sh quick` PASS;
60 s local fuzz: `cd crates/epiora-import/fuzz && CARGO_NET_OFFLINE=
true cargo +nightly-2026-10-15 fuzz run diff_any -- -max_total_time=60`
→ "Done 569856 runs in 61 second(s)", no crash. One 120 k-exec fuzz
crash during development was the harness decoder producing duplicate
column names, not product code — fixed in `fuzz_decode`. Fix round 1
(the split above, `record_json`'s `{}` contract, the stricter apply
checkers): `cargo test -p epiora-import --all-features` — 45 tests
green; `./scripts/check.sh quick` PASS; same 60 s fuzz command →
"Done 230244 runs in 61 second(s)", no crash.
Files outside the Files block:
- crates/epiora-import/Cargo.toml — serde_json moved to
  `[dependencies]`, proptest added to `[dev-dependencies]` (above).
- Cargo.lock — workspace lock for the two dependency changes.
- crates/epiora-import/fuzz/Cargo.toml — `[[bin]]` registration of the
  new `diff_any` target.
- crates/epiora-import/fuzz/Cargo.lock — fuzz crate lock, same reason.
- crates/epiora-import/src/diff/fuzz.rs — the `diff_any` decode/apply
  harness, moved out of `diff.rs` to keep every file under the
  ~400-line rule (fix round 1).
- crates/epiora-import/tests/common/mod.rs — shared `col`/`dataset`/
  `imported`/`key` test builders, split out of `tests/diff.rs` for the
  same rule (fix round 1).
- crates/epiora-import/tests/diff_property.rs — the
  `proptest_diff_applied_to_previous_gives_next` property and its
  strategies/apply helper, split out of `tests/diff.rs` (fix round 1).
- .github/workflows/engine.yml — the PR fuzz job also runs `diff_any`
  (60 s), mirroring the existing target loop.
- .github/workflows/fuzz-nightly.yml — `diff_any` added to the
  `fuzz-import` matrix (1800 s).
- docs/decisions.md — this entry.
Supersedes: —
### v0.3 Task 6 — epiora-import-wasm binding, parity and import workflow (2026-11-07)

Decision: `epiora-import-wasm` exports `inspect`, `inspectWith`,
`importFile` (`ImportedHandle`: `rowCount`, `summary`, `toSnapshot`,
`contentHash`, `recordBatch`, `keyCandidates`), `diffAgainst`
(`DiffHandle`: `summary`, `addedBatch`, `changedBatch`, `removed`) and
`datasetFromRecords` per the plan's Interfaces, with `import-runner`
and `parity/run.mjs` printing identical `parity_line`s for the
native/wasm diff and `import.yml` gating it all. Interpretations where
the plan leaves room:

- `diffAgainst(previousSnapshot, imported)` takes no key argument, so
  the diff key is the re-import's own `imported.settings.key` (the key
  the wizard picked for the new file). The previous snapshot is decoded
  with placeholder id `"previous"`/version `0` — the snapshot format
  stores neither and `diff` never reads them.
- `summary()` uses the plan's literal camelCase (`rowCount`,
  `sourceSha256`); `Inspection`/`PreviewColumn` keep snake_case field
  names matching the fixture spellings (`preview_rows`,
  `coerced_to_missing`).
- `contentHash()` builds the dataset with id `""`/version `0`; id and
  version are not hashed (v0.2 spec §3.3).
- `datasetFromRecords` returns the snapshot bytes of the rebuilt
  dataset (the plan's `→ Uint8Array`), not a handle.
- Errors cross as JS `Error` with `code` from `ImportError::code()`
  (engine codes pass through the `Engine` variant) and `message` from
  `Display`; `RaggedRow`/`MissingKey`/`DuplicateKey` also set `line` as
  a JS number.
- Bindings are regenerated by a programmatic `export_bindings` test in
  `epiora-import-wasm` (feature `ts`) that pins
  `ts_rs::Config::with_out_dir` to `crates/epiora-import-wasm/bindings`
  — not `#[ts(export)]` attributes, because
  `cargo test --workspace --all-features` runs every export test and
  `#[ts(export)]` would resolve the global `TS_RS_EXPORT_DIR`
  (`.cargo/config.toml` → `crates/epiora-wasm/bindings`), polluting the
  core bindings diff. `Value.ts`/`ValueType.ts` are emitted into the
  import bindings as `FieldChange`/`TypeChange` dependencies.
- `parity_line(case, outcome)` lives in `epiora-import` so native and
  wasm print byte-identical lines: `{case, summary, snapshot_hash,
  snapshot_sha256}` on success (`snapshot_hash` = dataset content hash,
  `snapshot_sha256` = SHA-256 of the snapshot bytes) and
  `{case, error: {code, message}}` on failure, so error cases stay
  deterministic on both sides.
- Conductor note honoured: `crates/epiora-import/tests/
  fuzz_regressions.rs` replays `fuzz/regressions/{text_import,
  sheet_import, diff_any}/` through `epiora_import::{text_import,
  sheet_import, diff_any}` behind the `fuzzing` feature (missing or
  empty folder is a pass, `.gitkeep` ignored), plus an empty/0xff
  smoke test; `.gitkeep`s added for the two missing regression dirs.
  `import.yml` runs `cargo test -p epiora-import --all-features` so the
  replay executes in CI.
- `import.yml` pins Node 22, matching `engine.yml` (the plan's Node 24
  is the app stack, not the wasm CI).
- `scripts/wasm-build.sh` takes an optional crate argument
  (`epiora-wasm` default, behaviour unchanged); `epiora-import-wasm`
  builds into `pkg/` (release, `-Oz` wasm-opt) and `pkg-parity/`
  (nodejs, `parity` feature).
- `import.yml`'s `on.pull_request.paths` are a superset of the plan's
  Step 4 list — the plan's list plus `rust-toolchain.toml`,
  `scripts/check*.sh`, `scripts/wasm-build.sh` and
  `crates/epiora-core/**` (`epiora-import` and `epiora-import-wasm`
  depend on `epiora-core` for the snapshot codec and content hash, so a
  core-only change can break import parity; owner-requested on PR #42).

Dependencies and licences (AGENTS.md rule 6): no new crates —
`wasm-bindgen`/`js-sys` (MIT) and `serde-wasm-bindgen` (MIT) were
already in the tree via `epiora-wasm`; `serde`, `serde_json`, `ts-rs`
(MIT) already via `epiora-import`; `Cargo.lock` changes only for the
new workspace member. `cargo deny check` clean.
Evidence: `./scripts/check.sh` PASS locally (run as
`PATH="$PWD/target/bin:$PATH" ./scripts/check.sh` — cargo-deny is
installed under `target/bin`, not as a cargo subcommand);
`import-runner fixtures/import` → 35 lines, `parity/run.mjs
fixtures/import` → 35 lines, `parity/diff.sh` identical (bit for bit);
`node --test crates/epiora-import-wasm/tests/node` → 6/6;
`epiora_import_wasm_bg.wasm` gzip -9 = 519,170 bytes (reported only —
the gate is Task 17); `cargo test -p epiora-import --all-features` →
fuzz_regressions green.
Files outside the Files block:
- crates/epiora-import/src/summary.rs — `ImportSummary`/`summary`/
  `RecordBatch`/`parity_line`; keeps `lib.rs` under the ~400-line rule
  (it was already at 362 lines).
- crates/epiora-import/src/lib.rs — `mod summary` + re-exports,
  `Serialize`/`Deserialize`/`TS` derives on `Inspection`/`PreviewColumn`
  (boundary types, spec §5.3), `sha256_hex` → `pub(crate)` for
  `parity_line`.
- crates/epiora-import-wasm/src/parity.rs — the `runImportCase` export
  behind `parity` (mirrors `epiora-wasm`'s `src/parity.rs` split).
- crates/epiora-import-wasm/pkg/.gitignore and
  crates/epiora-import-wasm/pkg-parity/.gitignore — ignore build
  output, same convention as `epiora-wasm`.
- crates/epiora-wasm/parity/diff.sh — reused for the import parity
  diff; its diagnostic grep now also extracts `"case"` names (behaviour
  on engine lines unchanged; parity-diff.bats still covers it).
- crates/epiora-import/tests/fuzz_regressions.rs and
  crates/epiora-import/fuzz/regressions/{sheet_import,diff_any}/.gitkeep
  — the replay test + dirs added per the conductor note for this task
  (Task 5 follow-up, PR #40).
- Cargo.lock — new workspace member.
- docs/decisions.md — this entry.
Supersedes: —

### v0.3 Task 7 — app scaffold, design tokens, component set, a11y harness, `app.yml`

- Token colours: nine roles (`bg`, `surface`, `text`, `textMuted`, `accent`, `accentText`, `border`,
  `focus`, `danger`) × four themes; every declared `pairs` entry passes its WCAG ratio in all four
  themes (4.5 text / 3 large-text,ui / 7 hc) — enforced by `contrast.test.ts`, not picked ad hoc.
- TypeScript is split (owner decision option 1 on PR #43): `typescript` is `^7.0.2` at the app
  root and runs `tsc --noEmit` (typecheck); `typescript-eslint@8.72.1` keeps `typescript ~6.0.3`
  for linting only. Mechanism: the lint stack lives inside a local `file:` provider package,
  `app/eslint-ts6/` (`dependencies`: `typescript ~6.0.3`, `typescript-eslint`), so npm nests it
  under `eslint-ts6/node_modules/` where its `typescript` peer resolves to 6.0.3 natively — no
  `--force`, no peer warnings; `eslint.config.js` and `lint-rules.test.ts` import `eslint-ts6`,
  which re-exports `typescript-eslint`. `app/.npmrc` sets `install-strategy=nested` so
  `ts-api-utils` (unbounded `typescript` peer, would otherwise hoist to the TS 7 root and crash
  on its missing API) stays inside the provider subtree. Plain `overrides` cannot do this: npm
  resolves a root-level package's `typescript` peer only at `node_modules/typescript`, which the
  root `typescript` dep already occupies → ERESOLVE without `--force`/`--legacy-peer-deps`.
  Reason: typescript-eslint 8.72.1 peers `typescript >=4.8.4 <6.1.0` and TS 7 exports only
  `./unstable/*` APIs (its entry points throw on import under TS 7). Remove the provider once
  typescript-eslint supports TS 7.
- `Table` mounts its `caption` through a portal (RAC's collection drops non-collection children) and
  points `aria-labelledby` at it, so the caption is a real first child of `<table>` and provides the
  accessible name.
- `ToastRegion` passes `aria-live="polite"` explicitly — RAC's `UNSTABLE_ToastRegion` is a landmark
  region without a live-region attribute.
- `FileTrigger` has no `FileTrigger.module.css` on purpose: RAC's FileTrigger renders no element of
  its own — it wires a hidden file input to its child — so the `Button` inside it carries the styles.
- Lint rules and the lint-fixture test share sources: `eslint.restrictions.json` and
  `stylelint.rules.json` are read by `eslint.config.js`/`stylelint.config.js` and imported by
  `lint-rules.test.ts`, so the test checks the exact rules the gate runs. `lint-fixtures/` is in both
  linters' ignore lists (its violations are intentional) and linted only inside the test.
- `licence check` in `check-app.sh` passes `--excludePackages 'epiora-app@0.3.0'` —
  license-checker-rseidelsohn reports the private root package itself as UNLICENSED; the gate is the
  production dependency tree (all MIT/Apache-2.0/Unlicense/0BSD at task time).
- Dependencies and licences (AGENTS.md rule 6): react + react-dom (MIT), react-aria-components
  (Apache-2.0), @tanstack/react-router (MIT). Dev: vite, @vitejs/plugin-react, vitest (MIT),
  typescript (Apache-2.0), eslint + typescript-eslint (MIT), stylelint (MIT), jsdom (MIT),
  @testing-library/* (MIT), @playwright/test (Apache-2.0), @axe-core/playwright (MPL-2.0, dev only),
  license-checker-rseidelsohn (MIT), globals (MIT), @types/* (MIT). Licence gate passes with
  `--production --onlyAllow`.
Evidence: `./scripts/check.sh` PASS locally (`PLAYWRIGHT_BROWSERS_PATH=target/ms-playwright`) —
  tsc --noEmit clean, ESLint clean, Stylelint clean, tokens regen diff clean, Vitest 8/8,
  `vite build` ok, Playwright 72/72 across chromium/firefox/webkit (12 axe runs per engine, light
  and dark; 6 viewport + 6 zoom checks per engine).
Files outside the Files block:
- `app/src/ui/components/classes.ts` — shared `cx`/`renderClass`/`RenderChildren` helpers matching
  RAC's `ClassNameOrFunction`/`ChildrenOrFunction` contracts; avoids duplicating the merge in 14
  components.
- `app/src/ui/routes/routes.module.css` and `app/src/ui/shell.module.css` — styles for the route
  placeholders and the nav shell (the Files block lists only `routes/*.tsx` and `router.tsx`).
- `app/e2e/routes.ts` — the §8.1 route list shared by `a11y.spec.ts` and `viewport.spec.ts`;
  Playwright forbids one spec importing another spec.
- `app/eslint.restrictions.json`, `app/stylelint.rules.json`, `app/lint-rules.test.ts`,
  `app/lint-fixtures/component-violations.tsx`, `app/lint-fixtures/style-violations.module.css` —
  Step 5's shared rule sources, the `lint_rules_catch_violations` test and its fixtures.
- `app/eslint-ts6/` (fix round 2, PR #43) — the `file:` provider package holding the lint
  toolchain's own `typescript ~6.0.3` + `typescript-eslint` (see the TypeScript bullet);
  `index.mjs`/`index.d.ts` re-export `typescript-eslint`.
- `app/.npmrc` (fix round 2) — `install-strategy=nested`, keeps `ts-api-utils` nested inside
  `eslint-ts6` so it cannot resolve the root `typescript` 7.
Supersedes: —

### v0.3 Task 8 — store: migrations, `Store`, repositories, OPFS backend, conformance suite

- Dependencies and licences (AGENTS.md rule 6): `@sqlite.org/sqlite-wasm` 3.54.1-build1
  (Apache-2.0, production), `comlink` 4.4.2 (Apache-2.0, production). Dev: `@vitest/browser`
  5.1.0 (MIT), `@vitest/browser-playwright` 5.1.0 (MIT), `playwright` 1.65.0 (Apache-2.0 —
  pinned to the same version `@playwright/test` already resolves, so the installed browser
  builds match). All are the latest release at task time and inside the plan's Tech Stack.
- Schema column names are `record_key` and `record_values`, not `key`/`values`: `values`
  unquoted is a syntax error in SQLite DDL (verified against sqlite-wasm 3.54.1), and `key`
  is a keyword. The spec §4.1 "Holds" column describes content, not column names.
- Conformance `open()` contract: one OPFS database per suite run (a fresh random name per
  file load), every `open()` call returns a new connection to it. That is what makes
  `uncommitted_transaction_is_invisible_after_reopen` a real reopen instead of a fresh
  empty store, and it is what Task 14's `openStdioStore(tmpPath(), …)` must mirror
  (one tmp path per suite run).
- WebKit + OPFS (the §14 risk the plan flags): Playwright's WebKit (27.2 here) has
  `createSyncAccessHandle` but `navigator.storage.getDirectory()` rejects with
  `UnknownError` in ephemeral (non-persistent) contexts — even on the main thread. A
  persistent context gives WebKit a real on-disk profile where OPFS works, so the webkit
  instance gets `provider: playwright({ persistentContext: <fresh dir per run> })` and the
  project sets `fileParallelism: false` (vitest ignores `persistentContext` under parallel
  sessions). A fresh profile dir per config load avoids stale-profile launch timeouts.
  Profiles live under `node_modules/.cache/vitest-webkit/<uuid>`; the config rmSyncs and
  recreates that parent at load so they cannot accumulate, and Playwright creates the
  `<uuid>` leaf itself on launch (the config is evaluated more than once per run, so
  mkdir'ing the leaf at load left an empty sibling dir — verified: one dir remains after
  two consecutive `--project store-opfs` runs).
- Comlink error boundary: thrown Errors cross with only name/message/stack — `StoreError`'s
  `code` would be lost. `harness.worker.ts` re-throws StoreErrors as a marked plain object
  (`{ __epioraStoreError, code, message }`, which travels by structured clone) and the test
  side revives them into real StoreError instances; the tx object handed to a proxied
  `transaction` callback gets the same treatment because those errors cross the boundary
  the other way. Task 14's relay store must implement the same wire shape.
- `Store.transaction` re-throws the callback's error unchanged after ROLLBACK (the plan's
  "re-throwing"); BEGIN and COMMIT failures are mapped through `mapError` (a nested
  `store.transaction` therefore rejects with a StoreError, code `sql`).
- `mapError` order: DOMException `name` first (`QuotaExceededError` → `quota_exceeded`;
  `NoModificationAllowedError` → `locked_elsewhere`), then the sqlite result code
  (`SQLITE_FULL` → `quota_exceeded`, `SQLITE_BUSY` → `locked_elsewhere`), then the message.
  `InvalidStateError` → `locked_elsewhere` only in `mapError`'s `'acquire'` context — the
  `getPool()` + `new pool.OpfsSAHPoolDb(name)` step in `openOpfsStore` (fix round 2: it is
  lock-significant only there; `'run'` context — exec, query, transaction, migrate — maps
  it to `sql`). A held sahpool reaches `openOpfsStore` as the raw DOMException from
  `createSyncAccessHandle` — no resultCode — so name matching is what maps it: Chromium
  `NoModificationAllowedError` ("Access Handles cannot be created …"), Firefox
  `NoModificationAllowedError` ("No modification allowed"), WebKit 27.2 `InvalidStateError`
  ("The object is in an invalid state."). The name is prefixed onto the StoreError message
  because Comlink drops it at the wire boundary. `mapError` lives in `map-error.ts` (pure)
  so `opfs.test.ts` can run in the unit project without the wasm / `?raw` module graph —
  `opfs.ts` cannot be imported under the unit project's `server.fs.allow`.
- `getPool` clears its cached promise on rejection and passes
  `forceReinitIfPreviouslyFailed` (present in sqlite-wasm 3.54.1, absent from the bundled
  .d.mts — cast like `locateFile`) so a later `openOpfsStore` retries instead of re-throwing
  sqlite-wasm's internally cached init failure. Proved by
  `held_pool_maps_to_locked_elsewhere_and_recovers`: a second worker's open rejects
  `locked_elsewhere` while the first holds the pool, and succeeds after the first is
  terminated.
- sqlite-wasm specifics: the bundled `index.d.mts` declares `init(): Promise<Sqlite3Static>`
  with no parameters but the implementation accepts `{ locateFile }` — a typed cast is used
  so the `.wasm` resolves through `?url` under Vite. `optimizeDeps.exclude` keeps the
  package out of pre-bundling (vite.config.ts and the vitest project config; vitest does
  not read vite.config.ts). oo1 `exec` returns `bigint` for i64 values beyond 2^53 and
  `Uint8Array` for blobs, matching the `SqlValue` contract directly.
- `check-app.sh` runs the two Vitest projects separately (`--project unit`,
  `--project store-opfs`) so the browser suite is visible in the gate output; `vitest run`
  alone runs both anyway.
- `server.fs.allow` is `[searchForWorkspaceRoot(process.cwd()), ../store/migrations]`
  (fix round 2: was `['..']`, which let the dev server serve the whole repo root). Vite
  requires the workspace root to stay in the list once `allow` is set; the migrations path
  is resolved from the config file's location, not cwd. Same list in vite.config.ts and
  the store-opfs vitest project.
Evidence: `npx vitest run --project store-opfs` → 15 tests × chromium/firefox/webkit = 45
  passed (`PLAYWRIGHT_BROWSERS_PATH=target/ms-playwright`); `./scripts/check.sh quick` PASS
  (npm ci, tsc --noEmit, ESLint, Stylelint, tokens diff clean, unit 9/9, store-opfs 45/45,
  licence check). Fix round 2 (PR #44): `./scripts/check.sh quick` PASS (unit 11/11,
  store-opfs 45/45), `npm run build` ok, Playwright e2e 72/72 across the three engines;
  `ls node_modules/.cache/vitest-webkit/` shows exactly one dir after two consecutive
  store-opfs runs. WebKit OPFS probes: `getDirectory()` fails with UnknownError in an
  ephemeral context, succeeds in `launchPersistentContext` (probe commands in session log).
Files outside the Files block:
- `app/package.json`, `app/package-lock.json` — the new dependencies above.
- `docs/decisions.md` — this entry.
- `.gitignore` — added `app/.vitest/` (vitest failure-attachment output, same class as the
  already-ignored `test-results/` and `playwright-report/`).
- `app/src/store/map-error.ts`, `app/src/store/opfs.test.ts` (fix round 2, PR #44) — the
  pure error mapper extracted from `opfs.ts` and its unit test (InvalidStateError context);
  extraction is what lets the test run in the unit project.
Supersedes: —

### v0.3 Task 9 — Data worker and typed RPC (2026-11-08)

Decision:
- `records` is fully rewritten in the new file order on every re-import (delete +
  upsert, not diffed in place). `content_hash_hex` is row-order sensitive, so
  `records.allJson` — the rebuild source of spec §4.3 — must reproduce the
  version's canonical order, which minimal upserts cannot do once a key-column
  re-import inserts rows mid-file. `changes` still receives only the minimal
  insert/update/delete entries the diff reported, and `records.version` carries
  forward "last changed" per record id.
- The schema `datasetFromRecords` needs is persisted inside `datasets.settings`
  as `{"settings": <effective ImportSettings>, "schema": [[name, type]…]}` — the
  column is opaque text and no schema column exists anywhere; a corrupt snapshot
  must not be the rebuild's own schema source. `DatasetInfo.settings` unwraps to
  the `ImportSettings`.
- `imports.insert` gains an optional caller `id` and is issued before the
  record/change batches inside the commit transaction, so every change entry
  carries `import_id`; plan op order otherwise preserved.
- Cancel is only reachable because `checkCancel` yields a macrotask: oo1 `exec`
  resolves on microtasks, so a commit would otherwise never let `cancelImport`
  run. sahpool's fixed capacity (default 6, persistent) is grown via
  `reserveMinimumCapacity(64)`/`addCapacity` in `openOpfsStore`, and worker store
  opens retry transient lock errors for 10 s (dying workers' handles).
- `RunContext.project` is always passed by the worker (receipt v2 names the
  project) — matches the ts-rs required-key binding.
- The bindings export no code lists, so `error_code_lists_match_rust`
  (contract.test.ts, fix round 1, PR #45) reads `crates/epiora-import/src/
  error.rs` and `crates/epiora-core/src/error.rs` and extracts the string
  literals each enum's `fn code()` returns (`=> "…"` match arms; the
  `ImportError::Engine` transparent wrap yields none) and asserts both sets
  equal the hand-copied lists in errors.ts — a new Rust `code()` fails the
  contract test instead of reaching the wire unmapped.
Files outside the Files block:
- `app/vitest.config.ts` — vitest does not read vite.config.ts; same alias and
  fs.allow additions mirrored (Task 8 precedent).
- `app/tsconfig.json` — `paths` so `tsc --noEmit` resolves the four aliases.
- `app/src/ui/strings.ts` — plan fixes `userMessage` to live in strings.ts.
- `app/src/store/repo/{runs,records,changes,versions,imports}.ts` — `runs.get`,
  `records.clearAll`/`idVersions`, `versions.markUnavailable`, kind filter on
  `changes.countByDataset` (the plan's "counts by kind"), optional `id` on
  `imports.insert`.
- `app/src/store/opfs.ts` — sahpool capacity growth and open retry (above).
- `app/src/worker/contract.types.ts` — the type file Step 2 names.
- `app/src/worker/commit.ts`, `app/src/worker/integration-helpers.ts` (fix
  round 1, PR #45) — the ~400-line rule split: the single-transaction commit
  out of importer.ts, and the shared test harness (worker spawn, fixture
  fetch, store-open retry) out of integration.browser.test.ts. Pure moves.
- `.github/workflows/app.yml` — check-app.sh now builds both wasm packages, so
  the app job installs the same pinned toolchain as engine.yml (Rust 1.99.0 +
  wasm32, wasm-bindgen 0.2.129, wasm-pack 0.15.1, binaryen wasm-opt version_133,
  all SHA-256 verified). First CI run failed `wasm-pack: command not found`.
Supersedes: —

### v0.3 Task 10 — Projects, datasets, versions and data grid screens (2026-11-09)

Decision:
- The dataset screen's version selector is a `Menu` with `selectionMode="single"`
  (menuitemradio), not a RAC `Select`. On Firefox, axe's `button-name` rule crashed
  the content process reproducibly on the post-open Select's `aria-labelledby`
  chain — bisected rule-by-rule on the live page (the same DOM replays clean in a
  fresh page, and removing `.react-aria-Select` stops the crash). A Menu trigger
  carries its name from content, so there is no labelledby chain to trip on; the
  role change keeps single-select semantics.
- `DataGrid` re-asserts `aria-rowcount` in a `useLayoutEffect`: RAC's `useTable`
  overwrites it with `collection.size + headerRows.length` when virtualised, but
  the plan fixes the attribute at the data row count (`GridPage.total`).
- `window.dataApi` is set in `main.tsx` so e2e specs seed datasets through the
  app's own typed RPC in `page.evaluate` (the plan's seeding mechanism); it is
  also a devtools handle.
- `@tanstack/react-query@5.107.1` added (in the plan's Tech Stack; MIT; the
  latest release at task time, per `npm view @tanstack/react-query version`).
- Screens read dataset settings only through `dataApi` (`DatasetInfo.settings`
  is already unwrapped by the worker); the `datasets.settings` composite column
  is never parsed in the UI (carry-over from v0.3 Task 9).
Files outside the Files block:
- `app/src/main.tsx` — `QueryClientProvider` and the `window.dataApi` handle
  above; the task's own files have nowhere else to wire them.
- `app/package.json`, `app/package-lock.json` — the `@tanstack/react-query`
  dependency.
- `app/src/ui/components/components.test.tsx` — the caption-stability test the
  owner asked for in the PR #43 carry-over (`table_caption_stays_first_child_
  across_rerenders`: rows added/removed, caption text changed, unmounted; the
  portal approach held — no replacement needed).
- `app/src/ui/components/Dialog.tsx`, `app/src/ui/components/Table.tsx`,
  `app/src/ui/components/index.ts` — `Heading` is re-exported next to `Dialog`
  and `TableLayout`/`Virtualizer` next to `Table` so screens and routes import
  only from the component set (fix round 1).
Supersedes: —
### v0.3 Task 11 — Import wizard (2026-11-09)

Decision:
- Opened as a draft PR pending an owner decision: on Firefox, axe's
  `button-name` rule crashes the content process after a RAC `Select` is
  opened (the crash bisected in Task 10). The plan fixes both "type
  `Select` per column" on the preview and `expectNoA11yViolations` on
  every wizard step, so no component swap, axe scoping or
  Firefox-only conditional was made; the options are in the PR body.
- `DataApi.__simulateQuotaOnNextWrite` exposes the Task 8 store test hook
  through the worker RPC for the `import shows quota error` e2e (Review
  Focus 3). It rejects `not_found` unless the bundle was built with
  `VITE_TEST_HOOKS=1`; only the `check-app.sh full` e2e build lane sets
  the flag — no production build does.
- On `done | error` the wizard calls `discardSession`; a
  `session_not_found` there is a no-op (Task 9's interface — the commit
  destroys the session itself), never shown as an error.
- `app/package.json` pins `@playwright/test` and `playwright` to
  exactly `1.64.1`, plus an `overrides` entry forcing
  `playwright-core` to `1.64.1` — npm's `install-strategy=nested`
  keeps `@axe-core/playwright`'s `playwright-core >=1.0.0` peer on
  the previously resolved 1.65.0 otherwise, splitting the tree into
  two playwright-core versions whose `Page` types clash under `tsc`.
  Together they are a temporary exception to "dependencies at the
  latest release", chosen by the owner (option 1 on PR #47). Reason:
  Playwright 1.65.0's bundled Firefox segfaults when
  `element.ariaLabelledByElements` is read in two JS realms
  (microsoft/playwright#43967, Mozilla bug 2090055); fix round 1's
  bisect reproduced the crash on the wizard's preview step with a
  named `getByRole` followed by a main-world
  `ariaLabelledByElements` read, and RAC's `Select` always emits
  `aria-labelledby`, so no markup or test-order change can avoid it.
  Removal trigger: return `@playwright/test` and `playwright` to the
  latest release and drop the `playwright-core` override as soon as
  a Playwright release whose bundled Firefox fixes #43967 is out,
  and re-run the full e2e on all three engines.
Files outside the Files block:
- `.github/workflows/app.yml` — `npm ci` now runs before the Playwright
  browser install (fix round 3): with no `node_modules`, `npx` fetched
  the latest playwright (1.65.0) and installed browser revisions the
  pinned 1.64.1 cannot launch — `chromium_headless_shell-1248`,
  `firefox-1555` missing in CI run 42671443566.
- `app/package.json` — the temporary 1.64.1 pin above.
- `app/package-lock.json` — regenerated by npm for the pin.
- `app/src/worker/api.ts` — the `__simulateQuotaOnNextWrite` method
  above; the quota-error test can only reach the store hook through the
  real worker RPC.
- `app/src/worker/client.ts` — the wrapped client method for it.
- `scripts/check-app.sh` — `vite build` needs `VITE_TEST_HOOKS=1` for the
  e2e run; it is the only build lane the e2e bundle goes through.
Supersedes: —

### v0.3 Task 12 — Analysis workspace and output log

Decision: The level pickers for two-level Text variables start empty in every
build — a two-level Text variable without explicit levels is an engine error,
not a guess (v0.2 spec §3). Production builds keep Run disabled, with a hint
programmatically associated to the button (`aria-describedby`), until both
levels are picked for every such variable; the gate is the pure `canRunTables`
helper. Only the `VITE_TEST_HOOKS=1` e2e build leaves Run enabled so the
`ambiguous_levels` engine error stays reachable through the real dialog path —
the plan's "level pickers left empty via the test hook". Every e2e build
carries the hooks, so the production gating is covered by Vitest instead
(`tables-run.test.ts` exercises `canRunTables` with both `testHooks` values).
FREQ renders both per-category CIs v0.2 returns (the `ci_method` interval
first, the other method second, each under its own scoped, labelled column).
The project screen's empty output log offers the next action: an Analysis
workspace link (spec §8.2). Separately, the dialogs' `aria-busy` moved from
`Dialog` onto the `.form` element — RAC's `filterDOMProps` strips every
attribute outside its labelling whitelist, so the attribute never reached the
DOM when set on `Dialog`.
Files outside the Files block:
- app/src/ui/components/Dialog.module.css — two real fixes the Task 12 dialog
  axe/e2e tests exposed: the overlay's `opacity: 0.8` faded every colour inside
  the modal (RAC nests Modal inside ModalOverlay; axe measured the "Select an
  item" placeholder at 4.3:1), replaced by a translucent
  `color-mix(var(--color-bg))` backdrop; `.dialog` gained `max-height: 90vh` +
  `overflow-y: auto` because the 22-variable FREQ form pushed the Run button
  outside the fixed-position viewport.
- app/src/ui/screens/analysis/tables-run.ts — the pure `canRunTables` gate; a
  module of its own because unit tests cannot import TablesDialog.tsx (it
  pulls in `worker/client`, which spawns a Worker at module load).
- app/src/ui/screens/analysis/tables-run.test.ts — its Vitest coverage of both
  `testHooks` values.
- docs/decisions.md — this entry.
Supersedes: —

### v0.3 Task 13 — PWA, offline, tab lock, Netlify deploy

Decision: **CSP option A** — the owner's choice, given to the conductor on
PR #49. `_headers` `style-src` is `'self'` plus exactly two `'sha256-…'`
sources, the SHA-256 (base64) of the exact `textContent` of the two `<style>`
elements react-aria-components injects at runtime:
`'sha256-38RhXrc7EdReTKsOm23ZPOCUgniTUUcjky8QOOrQx6o='` for usePress's
touch-action `@layer` rule and `'sha256-gYiS/BvZvRcK27JIXTuwhZ3hs2+VJ1X+2gUlE+farlg='`
for usePreventScroll's `.trim()`ed overscroll-behavior `@layer` rule. No
`'unsafe-inline'`, no nonces, no other sources; every other directive and
header is the plan's verbatim text. The hashes were derived by evaluating
each library file's `style.textContent` template literal, not hand-copied.
`e2e/csp.spec.ts`'s `runtime_styles_are_exactly_the_csp_hashes` collects
every `<style>` element across the routes, after a press and with a dialog
open, hashes each text and asserts the set equals the two sources in
`_headers` — a React Aria upgrade that changes either text fails it, and
`no_csp_violations_on_any_route` still demands zero
`securitypolicyviolation` events across every route, an import and a run.
The usePreventScroll sheet only mounts under `isIOS() && isWebKit()`, so
the test collects it in an iPhone-realm context (Mobile-Safari UA plus
`navigator.platform`/`maxTouchPoints`/`userAgentData` spoofed before app
code runs) — that is also the only place a wrong hash would hurt, so
asserting set-equality there, not just subset, is the point.

`clientsClaim` was tried and removed. With it on, the update prompt did
NOT hold: in e2e runs (and in `vite preview` reproductions with the real
generated `sw.js`) a waiting update auto-activated mid-session — the
controller changed under a live page seconds after `registration.waiting`
appeared, and the app's reload-on-control listener then swapped versions
silently, exactly what `registerType: 'prompt'` forbids. With it off, the
waiting worker stays parked (verified against bare pages and the real
build) until `UpdatePrompt` posts `SKIP_WAITING`. The cost is that the
first-visit page is uncontrolled until its next navigation; a freshly
activated sw also takes a beat before it starts answering fetches, so the
specs reload until `navigator.serviceWorker.controller` is set.

Two more sw-prompt gaps `UpdatePrompt` covers: workbox-window's `waiting`
event can miss an update whose install the browser's own update check
began before workbox's listeners attached (a reload right after a
deploy), so `onRegisteredSW` watches `registration.installing`/`waiting`
and shows the toast itself; it prompts only when `registration.active`
is also set, because a first install lands in `waiting` momentarily
before it activates (a spurious toast on every first load otherwise).
And pwa-register only attaches its `controlling` → reload listener
inside its own prompt path, so the fallback arms a one-shot
`controllerchange` → reload listener when it fires the prompt.

Icons: `app/scripts/make-icons.mjs` renders them — a dependency-free PNG
encoder over `node:zlib` drawing the accent rounded square with the white
ascending chart mark (192/512, `any` and `maskable` purposes). Outputs are
committed so builds need no rasteriser; `npm run icons` regenerates.
`workbox-window` is a direct devDependency because `UpdatePrompt` registers
the sw through vite-plugin-pwa's `virtual:pwa-register`, which is
workbox-window (`immediate: true`, `onNeedRefresh` → the toast, Reload →
`messageSkipWaiting`). `vite-plugin-pwa` 2.0.0 and `workbox-window` 7.4.1
are the latest releases at implementation time; both MIT.

The tab lock is a Web Lock `'epiora-store'` held by the data worker for its
lifetime (the OPFS sahpool allows one connection): a second tab gets
'held-elsewhere' and shows `LockScreen`; "Use here" steals it (`steal:
true`), the old worker closes its store and posts 'lost' on the
'epiora-tab-lock' BroadcastChannel so its tab swaps to the lock screen.
`openStore` re-checks the lock state so a stale worker can never reopen
the store.

Files outside the Files block:
- app/index.html — links `/manifest.webmanifest` and the icons, sets
  `theme-color`; there is no other place for the PWA hooks.
- app/package.json — adds `vite-plugin-pwa`, `workbox-window` and
  `netlify-cli` (devDependencies) and the `icons` script
  `make-icons.mjs` refers to.
- app/package-lock.json — regenerated by npm for those devDependencies.
- app/tsconfig.json — `types` gains `vite-plugin-pwa/client` so
  `import('virtual:pwa-register')` typechecks.
- app/src/ui/strings.ts — the lock-screen and update-toast copy; spec §9.3
  keeps every user-facing string in this one module.
- app/src/ui/LockScreen.module.css — LockScreen's styles; the component
  set's convention is a CSS module per component (Stylelint enforces
  tokens).
- app/src/worker/client.ts — the worker `name` becomes `epiora:<tab id>`
  (db name + the tab identity the lock channel tags) and `commitImport`
  fires the spec §7.1 `navigator.storage.persist()` request.
- app/src/worker/store-backend.ts — `openStore` acquires/gates on the tab
  lock (`locked_elsewhere` when held) and `closeStore` releases the
  connection when the lock is stolen.
- app/scripts/make-icons.mjs — the icon generator above.
- docs/decisions.md — this entry.
Step 5 (the `.github/workflows/app.yml` deploy steps) landed after the
owner's human step 4 — the Netlify site exists with no Netlify build and
the `NETLIFY_AUTH_TOKEN`/`NETLIFY_SITE_ID` repository secrets are set. The
owner changed Step 5 via the conductor on PR #49: production deploys run
only on `release: types: [published]` — this replaces the plan's "`--prod`
on pushes to `main`", and there is no `push` trigger. `pull_request`
deploys a `--alias pr-<number>` preview (same-repo PRs only;
`head.repo.full_name == github.repository` keeps forks out) and
`workflow_dispatch` deploys a preview aliased to the sanitised branch
name, never `--prod`. Deploys run as the final steps of the `app` job so
nothing ships unless every gate passed; the deploy `dist` is a fresh
`npm run build` with `VITE_TEST_HOOKS` unset (the e2e build's
`__simulateQuotaOnNextWrite` guard is compiled out there, so a deploy
step fails unless its "test hooks are disabled" rejection message is
present and the `VITE_TEST_HOOKS` identifier absent). `netlify-cli`
27.17.0 (the latest release at implementation time; MIT) is pinned
exactly in `app/package.json` devDependencies and run via `npx netlify`
from `app/`; secrets reach it through `env:` only and the job has
`permissions: contents: read`.
The owner added a `_headers` `/manifest.webmanifest` block via the
conductor on PR #49: the live preview served it as
`application/octet-stream`, so it now sets `Content-Type:
application/manifest+json` (the `/*` security headers still apply to
that path, and nothing else in `_headers` changed).
Supersedes: —

### v0.3 Task 14 — Tauri shell

- Vendored `libsqlite3-sys` 0.39.1 — the release matching the latest
  rusqlite (0.41.1, which requires `^0.39.0`). Upstream crate checksum
  (crates.io, verified at download):
  `14a5f01b87b92705d07b433c06f96cbc764637d81a0bffb5734ce17cf0f9a2b3`.
  The vendored build compiles `sqlite3mc-src`
  `=206.1.0+sqlite3mc-2.6.1-sqlite-3.54.0`'s untouched
  `sqlite3mc_amalgamation.c` in place of the stock `sqlite3.c` (removed,
  as in the spike) and adds `-DSQLITE_ENABLE_MEMORY_MANAGEMENT`; the
  stock `sqlite3.h` is also removed, so the `buildtime_bindgen` feature
  cannot be used — we ship the prebuilt bindings.
- Copy, not import: `vendor/libsqlite3-sys`'s patch approach and
  `app/src-tauri/icons/**` are copies from `spikes/tauri-shell` (the
  spike is not referenced or imported anywhere).
- Keychain naming: `keyring` service `org.epiora.app`, user
  `database-key-v1`, value 64 lowercase hex minted from `getrandom` on
  first launch. `KeyError::Unavailable` → `keychain_unavailable`, a
  malformed stored entry (`KeyError::Malformed`) → `key_invalid`.
- Versions (latest at task time): tauri 2.12.5, tauri-build 2.7.4,
  tauri-plugin-dialog 2.8.4, rusqlite 0.41.1, keyring 4.2.0,
  getrandom 0.4.3, serde 1, serde_json 1; `@tauri-apps/api` 2.12.4 and
  `@tauri-apps/cli` 2.12.4 (npm). Licences recorded: rust tree MIT /
  Apache-2.0 except `sqlite3mc-src` which adds `Unlicense` to the
  src-tauri deny.toml allow list (its expression is
  `MIT AND CC0-1.0 AND Unlicense AND (CC0-1.0 OR Apache-2.0)`); npm
  `@tauri-apps/*` are MIT OR Apache-2.0.
- Wire: the worker's relay store posts `{ epiora: 'store-relay', id, op,
  sql, params }` on the worker message channel (Comlink ignores them —
  unknown `type`/`id`); `bridge.ts` invokes the five Rust commands.
  Params and results cross IPC as one length-prefixed binary frame
  (`Uint8Array` request body, `tauri::ipc::Response` bytes back) —
  blob cells are raw bytes both ways, never JSON arrays. The spec's
  `store_snapshot_get/put` commands are not implemented: snapshot blobs
  already cross as binary params/results through `store_exec`/
  `store_query` (ambiguity resolved toward the plan's five commands).
- App commands carry no ACL identifiers in Tauri 2.12 (`gen/schemas`
  lists only `core:*` and `dialog:*`), so `capabilities/default.json`
  cannot name the five store commands; it keeps `core:app|event|webview|
  window:default` + `dialog:allow-open` (menu/tray/path/image/resources
  dropped as clearly unused).
- `StoreErrorCode` gains `QuotaExceeded` beyond the plan's four
  variants so a real `SQLITE_FULL` surfaces `quota_exceeded`
  (review focus 3); the plan's variants are unchanged.
- `NativeStore::exec` uses `execute_batch` when no params are bound —
  `execute` runs only the first statement of a multi-statement string
  and migrations are sent as one exec.
- `store-stdio` (`required-features = ["stdio"]`, never bundled) speaks
  JSON lines; integers are decimal strings (i64 lossless), blobs base64.
  The `test_fail_next_write` op fails the next mutating exec with
  `quota_exceeded` — `stdio.ts`'s `__simulateQuotaOnNextWrite`.
- `cargo test --workspace` compiles the epiora-app bin, which links
  webkit2gtk/gtk: `check-app.sh` builds `store-stdio` in all modes and
  runs `cargo deny` + the workspace test in `full`, gating the app-crate
  build on `pkg-config webkit2gtk-4.1` (the `native-store` CI job
  installs the Tauri deps and stubs `app/dist`, which
  `generate_context!` needs at build time).
- deny.toml ignores: RUSTSEC-2024-0429 (glib `VariantStrIter`, unused by
  the app), RUSTSEC-2024-0370 (proc-macro-error via gtk3-macros),
  RUSTSEC-2026-0370/0371/0372 (zvariant, zbus_names, zvariant_derive via
  keyring's secret-service backend — all merged into zbus 6, which the
  keychain crates do not consume yet). Transitive, no safe upgrade.
Evidence: `cargo test --workspace` in the CI-equivalent container —
3 keystore + 4 native store tests pass; `npx vitest run
src/store/conformance/native.test.ts` — 14/14 pass; `cargo deny check`,
`cargo fmt --check`, `cargo clippy -D warnings` clean;
`./scripts/check.sh quick` PASS.
Files outside the Files block:
- `app/src-tauri/icons/**` — copied from the spike; `generate_context!`
  and bundling read `bundle.icon` paths at build time.
- `app/src-tauri/.gitignore` — `/target` and `/gen` (spec §7.3: gen is
  generated in CI, not committed).
- `app/src/worker/client.ts` — exports the worker handle for
  `startTauriBridge`, names it `epiora-tauri:<tab id>` when
  `__TAURI_INTERNALS__` exists (the relay marker; workers cannot see
  Tauri's injected internals), exports `inTauri` for main.tsx/settings.
- `app/eslint.config.js` — ignores `src-tauri/target/**` (generated
  build artifacts).
- `app/package-lock.json` — regenerated for the new npm deps.
- `docs/decisions.md` — this entry.
Supersedes: —

### v0.3 Task 15 — Desktop builds and smoke test

- `desktop.yml` runs a `ubuntu-24.04` / `windows-latest` / `macos-latest`
  matrix on PRs touching `app/**` or `crates/**`, on pushes to `main`
  and on `workflow_dispatch`: clean `npm run build` + `npx tauri build`
  (no signing env), bundles uploaded as `epiora-desktop-<os>-<sha>`.
- The uploaded bundles are production-shaped: the smoke test runs a
  separate `npx tauri build --no-bundle` binary over a
  `VITE_TEST_HOOKS=1` dist, so no test hook ships in the artifacts
  (the same separation app.yml keeps between its e2e and deploy builds).
- The wasm tool triplet is pinned and SHA-256-verified per OS —
  wasm-bindgen 0.2.129, wasm-pack 0.15.1, binaryen version_133 (same
  versions as `app.yml`) — installed into `target/bin/`, which
  `scripts/wasm-build.sh` already prefers over PATH tools. The binaryen
  tarball's `lib/` is extracted beside it at `target/lib/`: the macOS
  `wasm-opt` links `@rpath/libbinaryen.dylib` with rpath
  `@loader_path/../lib`, so `bin/` alone aborts at launch (run
  42924203593).
- `scripts/desktop-smoke.sh` installs `webkit2gtk-driver`, `xvfb`,
  `gnome-keyring` and `dbus`, runs the plan's
  `cargo install tauri-driver --locked` verbatim (the resolved version
  is logged), starts an unlocked gnome-keyring inside `dbus-run-session`
  and runs WebdriverIO under `xvfb`. `XDG_DATA_HOME`/`XDG_CONFIG_HOME`
  point at a mktemp dir, so first launch exercises key creation and the
  database lands at a known path (`$XDG_DATA_HOME/org.epiora.app/
  epiora.db`).
- "The dialog test hook `VITE_TEST_HOOKS=1` that accepts a path": the
  wizard's file input accepts a *path* via WebDriver element send keys
  on `<input type=file>` — the WebDriver file-dialog mechanism. Drivers
  that cannot set file inputs fall back to injecting the same fixture
  bytes into the same input; either way the wizard's own onFile path
  runs (ambiguity resolved toward the mechanism that needs no app
  change, since no path-reading IPC exists or may be added).
- `wdio.conf.mjs` sets `reporters: []` — the spec logs its own progress
  and the launcher prints the summary; `@wdio/spec-reporter` stays out
  of the dependency list (only the plan's three packages added, all
  10.2.0, MIT; transitive deps incl. `edgedriver`/`geckodriver` via
  `@wdio/utils` are MIT/Apache-2.0/BSD/ISC).
- The `webdriverio` 10.2.0 engines floor is Node 24.22.1; the local
  toolchain (24.22.0) warns EBADENGINE — install-only warning, CI's
  `node-version: 24` resolves newer. The spec uses only the injected
  `browser`/`$` globals, no package imports.
- Local `scripts/desktop-smoke.sh` cannot pass in this sandbox (no
  webkit2gtk-4.1 dev files, no display, sudo blocked): it fails at the
  expected first step. The green run is the CI job — link recorded in
  the PR.
- The smoke spec drives the real UI strings: the project dialog's
  submit is "Create" (`t.common.create`), not the trigger's "New
  project" — surfaced by run 42927777638 — and "Record key" is the
  wizard select's accessible name (Label span beside the trigger
  button), not its text — found by auditing every spec label against
  `strings.ts`. The assertions, fixture and `SQLite format 3` check
  are unchanged.
- Windows smoke attempt (plan Step 3): removed from the matrix — the
  leg kept its bundle build but lost the smoke step. msedgedriver
  cannot create a WebDriver session for the app on `windows-latest`:
  the WebView2 browser process never opens its DevTools port
  ("DevToolsActivePort file doesn't exist") — unmodified in run
  42927777638 and with
  `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS='--no-sandbox --disable-gpu'`
  in run 42930223305. The Linux leg runs the same spec green, so the
  WebView2 session failure is Windows-image-specific, not a spec bug.
  Reinstating needs a runner where msedgedriver↔WebView2 attach works.
- Files outside the Files block: `app/package-lock.json` — regenerated
  for the new dev dependencies (same convention as Task 14).

### CI speed-ups (owner decision 2026-11-11)

Decision: Add three caches and move two desktop bundle legs off PRs.
Nothing is loosened or skipped: every check that ran on a PR still runs,
except the Windows/macOS bundle legs (below).

- **Rust build cache** — `Swatinem/rust-cache` v2.9.3 (pinned by SHA) in
  every job that compiles Rust in `app.yml`, `desktop.yml`, `engine.yml`
  and `import.yml`. `workspaces` covers each Cargo workspace the job
  compiles: the root workspace (`.`), `app/src-tauri`, or both.
  `save-if: ${{ github.ref == 'refs/heads/main' }}` — only main writes;
  PRs, dispatches and release runs restore main's entry. On PR runs of
  engine.yml/import.yml `GIT_SHA` differs per commit, so `build.rs`
  (`rerun-if-env-changed`) rebuilds epiora-core and its dependents —
  correct, and the dependency cache still applies. The nightly fuzz
  jobs are left uncached: they use separate fuzz workspaces
  (`crates/*/fuzz`, excluded from the root workspace) and the win was
  not trivially safe to take.
- **Playwright browsers** — `~/.cache/ms-playwright` cached via
  `actions/cache` v6.1.0 (pinned by SHA) in app.yml's `app` job and
  engine.yml's `bench` job. Key:
  `<os>-playwright-<playwright-core version>` for the app job's
  three-browser set, the same with a `-chromium` suffix for the bench
  job's chromium-only set — the version read from the job's own
  lockfile (`app/package-lock.json` /
  `crates/epiora-wasm/bench/package-lock.json`, both pinned at 1.64.1).
  The suffix matters: a subset entry must never serve a job needing
  more browsers — this PR's first run proved it when the bench job
  saved a chromium-only entry under the shared key and the app job
  then restored it and had no firefox/webkit binaries. The app job
  keeps a single unconditional `npx playwright install --with-deps
  chromium firefox webkit`: on a hit the download half verifies the
  binaries (and tops up a partial entry), the system-deps half always
  runs. `npm ci` stays (lockfile authoritative); the setup-node npm
  cache stays.
- **wasm packages** — `crates/epiora-wasm/pkg` and
  `crates/epiora-import-wasm/pkg` cached in desktop.yml's `bundle` job
  only; a hit skips `scripts/wasm-build.sh`. Key:
  `<os>-wasm-pkg-wbindgen<0.2.129>-wpack<0.15.1>-<version_133>-` +
  `hashFiles('crates/**', 'Cargo.lock', 'rust-toolchain.toml',
  'scripts/wasm-build.sh')`. The key covers every build input and
  desktop.yml sets no `GIT_SHA`, so a hit restores byte-identical
  output. Not cached elsewhere: app.yml's `app` job builds inside
  `check-app.sh`, which cannot skip the build without a script change;
  engine.yml's `engine` job runs the `wasm-size.sh` budget inside
  `check-engine.sh` — no pkg cache there means the gate keeps measuring
  a real, fresh build of the PR's tree — and engine.yml/import.yml set
  `GIT_SHA: ${{ github.sha }}`, which `build.rs` embeds, so a
  sha-less key would serve a binary stamped with another commit's sha
  and a sha-ful key would never hit (bench/validation-report get
  rust-cache only). The import.yml size report stays a fresh build for
  the same reasons.
- **desktop.yml `bundle` legs** — `windows-latest` and `macos-latest`
  run only on `push` to `main`, `workflow_dispatch` and `release`. A
  job-level `if` cannot read `matrix` (evaluated before the matrix
  expands), so a per-leg `RUN_BUNDLE` env flag on the existing
  `matrix.os` gates every step — no new matrix key, so the ubuntu
  check name `bundle (ubuntu-24.04, linux, …)` is unchanged. On PRs the
  non-ubuntu legs still appear as checks but finish in seconds with
  every step skipped (verified on this PR's first desktop run:
  `bundle (windows-latest, …)` 4 s, all 15 steps skipped). The ubuntu
  bundle and
  the Linux smoke test still gate every PR. Trade-off: an
  OS-specific bundle break is found on main after merge instead of on
  the PR; main pushes still build and upload both bundles, so the
  failure is caught there, and a PR that needs the other OS legs can
  trigger `workflow_dispatch`.
- **push-to-main triggers** — `app.yml`, `engine.yml` and `import.yml`
  now also run on `push` to `main` with the same `paths` filter as
  their `pull_request` trigger. Required to make the Rust build cache
  effective at all: `save-if: ${{ github.ref == 'refs/heads/main' }}`
  only writes on a run whose ref is main, and these three workflows
  had no trigger that ever ran on main, so their cache was never
  saved and every PR restored a miss (desktop.yml already ran on main
  pushes for its bundles). A merge that touches a workflow's paths now
  warms the cache its next PR restores from. Nothing deploys or
  publishes on `push`: app.yml's deploy-build and test-hook-assert
  steps explicitly require `release`, `workflow_dispatch` or a
  same-repo `pull_request`, the three Netlify deploy steps keep their
  single-event `if`s, and engine.yml/import.yml contain no deploy or
  publish steps (their `always()`/`failure()` steps only write job
  summaries and diagnostic artifacts). Cost: one extra run of each of
  the three workflows per merge that touches its paths.

Evidence: baseline timings from app.yml run 43025518092 and desktop.yml
run 43025517715 (`tauri build` 283–335 s/OS, `npm ci` 98–194 s, wasm
build 40–84 s, `check-app.sh full` 388 s, Playwright install 49 s,
native-store `cargo test` 81 s). First-run timings and the run URL are
in the PR body; `save-if` means the speed-up lands after merge.
Files outside the Files block: docs/decisions.md — required by the task
brief (decisions entry for the cache keys and the main-only move).
Supersedes: —

Regression and fix (2026-11-11, `ci/fix-rust-cache-advisory-db`): the
first warm-cache restore broke `cargo deny check` in every job that ran
it — main push run 43101360702 (6d26661, cold cache) passed cargo-deny
then saved the cache; PR #52 run 43101947021 restored it (`full match:
true`) and cargo-deny failed with `[ERROR] failed to load advisory
database: failed to load any advisories in the database`.
Root cause: both `deny.toml`s set
`db-path = "${EPIORA_DENY_DB:-<workspace>/target/advisory-dbs}"`, so the
RustSec DB clone lives inside `target/`, a rust-cache path.
rust-cache's pre-save `cleanTargetDir` (v2.9.3 `src/cleanup.ts`)
deletes every *file* under `target/` outside cargo profile dirs
(`build`/`.fingerprint`/`deps`); applied to the clone it leaves only an
empty directory skeleton — verified locally by replaying its exact
walk: 1,587 files in, 0 out. cargo-deny then loads zero advisories from
the restored skeleton. Fix, not a loosening: every step that runs
cargo-deny (`app.yml` native-store's `cargo deny check` and the app
job's `check-app.sh full`, `engine.yml`'s `check-engine.sh full`,
`import.yml`'s `cargo deny check`; `desktop.yml` runs none) sets
step-level `EPIORA_DENY_DB: ${{ runner.temp }}/advisory-dbs` — outside
every cached path, so the DB is fetched complete and fresh each run
while the Rust build cache still works. It must be step env: the
`runner` context doesn't exist in workflow- or job-level `env` (the
first push of this fix failed every workflow at startup with a
"workflow file issue" until the override moved onto the steps).
`EPIORA_DENY_DB` matches none of rust-cache's key env prefixes
(`CARGO`, `CC`, `CFLAGS`, `CXX`, `CMAKE`, `RUST`), so the cache keys
are unchanged. `check-engine.sh`'s offline mirror now writes to
`${EPIORA_DENY_DB:-target/advisory-dbs}` — the path deny.toml actually
resolves. The advisories check, its ignores list and every fetch still
run; no flag skips or ignores advisories.

### v0.3 Task 16 — Performance and size measurement

- `runs` gives N samples as N generated Playwright `test()`s inside one
  `perf` job invocation (`PERF_RUNS`), not a matrix of jobs: every
  `test()` already gets a fresh browser context, hence a fresh OPFS
  store, and one job produces one `results.jsonl` and one summary
  instead of N scattered artifacts and N copies of the Rust/wasm
  toolchain setup.
- The `perf` job runs only on `workflow_dispatch` (input `runs`,
  default 10); PR/push behaviour of the existing jobs is unchanged.
  The 1,000,000-row and 10,000-row CSVs are generated in the job by
  `perf/gen-csv.mjs` (seeded xorshift32, seed 20261105), never
  committed — generated files and `results.jsonl` live under
  `target/perf/`, which is already ignored.
- Measure only: `perf.spec.ts` asserts nothing about timings or sizes;
  the proposed thresholds in `docs/perf/v0.3-measurement.md` are
  labelled proposals for the owner (spec §10.3).
- Scenario names carry the row count (`import_1m` at the CI default,
  `import_100k` locally) so a smaller local run can never be mistaken
  for the 1M measurement. `open_tables_1m` clears the snapshot LRU,
  then times `runTables` on the current version (v2, after the
  re-import measurement): cold snapshot load + hash check + the 2×2
  TABLES of `exposure`/`outcome`.
- Carryover scenarios measured in the same spec: `empty_store_opfs`
  (`storageEstimate()` right after the first store RPC opens the
  store), `reimport_<label>` (the big CSV with the first data row's
  `age` field bumped — one updated record, `records` still fully
  rewritten), `grid_scroll_10k` (viewport stride, every scroll
  position) and `grid_scroll_<label>` (150 evenly spaced steps on the
  big dataset — a bounded pass, since each step's page loads pay the
  `items` rebuild that scales with `total`). Heap is Chromium's
  `performance.memory.usedJSHeapSize`; frame time is the rAF gap at
  each step. Each pass is capped at 300 s wall-clock — the 100k-grid
  local run exceeded 30 min without it — and a renderer crash is
  recorded (`{crashed: 1, stepsDone}`), never hidden: the local 100k
  run crashed the tab on one attempt. A page that crashes on the
  10k pass reports the big-grid pass as `skipped_page_crashed`.
- `summarise.mjs` p90 is nearest-rank; median averages the two middle
  samples at even n. First-load set = `index.html` + the entry module
  script + the entry stylesheet + `epiora_wasm_bg-*.wasm` +
  `sqlite3-*.wasm` from `dist`, each `gzip -9`, summed; import-wasm is
  the same `pkg/epiora_import_wasm_bg.wasm` `import.yml` already
  reports.
- Fix round 1 (review of run 43005184626, where `import_1m` threw
  `SQLITE_IOERR` ~80 s in and `mode: 'serial'` skipped the other nine
  samples): the spec drops `serial` so samples are independent, wraps
  every scenario so a throw is recorded as `{scenario, error, …diagnostics}`
  and dependent scenarios record `{scenario, skipped: reason}`; a
  failed scenario also records the last `commitImport` progress stage,
  `storage.estimate()`, every OPFS file's size with the sahpool
  path decoded from each file's 512-byte name header, and the worker
  console tail — the sahpool VFS `console.error`s the DOMException an
  OPFS op threw, the only place the underlying error name survives the
  RPC `sql` mapping. `summarise.mjs` counts ok/failed/skipped per
  scenario next to median/p90 of successful samples and prints a clear
  line when `results.jsonl` is missing. The job gains a `rows`
  dispatch input (`PERF_ROWS`) so the smallest failing row count can
  be probed without touching code, `df`/`free` steps around the spec
  so a full filesystem or exhausted RAM is distinguishable from an
  OPFS-internal cap, and a second `vite build` without
  `VITE_TEST_HOOKS` into `target/perf/dist-nohooks`: the flag changes
  the entry chunk (`TablesDialog`'s hook gate), so first-load sizes
  are measured on the no-hooks build with the hooks build printed
  alongside for comparison.
- Fix round 2 (owner's Option A on the `SQLITE_IOERR` ceiling): the
  `perf` job gains a `profile` dispatch input (`persistent` |
  `ephemeral`, default `persistent` → `PERF_PROFILE`). The spec stops
  using the managed `page` fixture and launches Chromium itself via a
  `perfPage` fixture: `persistent` gives each sample a
  `launchPersistentContext` on a fresh temp user-data dir (a real
  user's on-disk profile — the control the run 43010218889 diagnosis
  lacked), deleted after the sample; `ephemeral` is `chromium.launch`
  + `newContext` on the same project `use` options, matching the first
  run's environment. Both launch with `--enable-precise-memory-info`
  so `heapStartBytes`/`heapEndBytes` are exact bytes instead of
  Chromium's quantised figures, and every `results.jsonl` line
  records `profile`. `summarise.mjs` prints the profile(s) it read.
- Files outside the Files block, fix round 1: `app/tsconfig.json`
  (`perf/**/*.ts` added to `include` so the spec is typechecked with
  the app).
- Files outside the Files block: `app/src/worker/snapshots.ts`,
  `app/src/worker/api.ts`, `app/src/worker/client.ts` — the plan's
  "clearing the worker's handle LRU" needs an explicit hook
  (`__clearSnapshotCache`, `VITE_TEST_HOOKS` builds only, same pattern
  as `__simulateQuotaOnNextWrite`); evicting through unrelated loads
  would silently depend on `LRU_SIZE = 2`. `app/playwright.config.ts`
  — the `EPIORA_PERF=1` switch so the measurement spec never enters
  the e2e gate.
- Merge with `origin/main` (PR #53 CI speed-ups): the `perf` job gains
  the same `Swatinem/rust-cache` step as the `app` job (same pinned
  SHA, `save-if: ${{ github.ref == 'refs/heads/main' }}`), with
  `workspaces: .` — it compiles only the root workspace (the wasm
  builds), not `app/src-tauri`.

### v0.3 Task 17 — Performance and size gates (2026-11-12)

- Owner thresholds (PR #52 comment 2026-11-11T21:14:51Z,
  <https://github.com/dorrabbkhan/Epiora/pull/52#issuecomment-6807191386>,
  "Owner decision recorded by the conductor"), copied exactly:
  `import_1m` ≤ **89,550 ms** (p90 71,639.3 ms × 1.25, persistent-profile
  run 43025510601); `open_tables_1m` ≤ **2,874 ms** (p90 2,299.2 ms ×
  1.25, same run); import-wasm ≤ **573,616 bytes** gzip -9
  (521,469 × 1.10). Gate semantics: a scenario passes when its measured
  median over successful samples is ≤ the threshold; the size gates
  fail when the gzip -9 size exceeds 573,616 bytes (import-wasm) or
  reaches 5,000,000 bytes (first-load budget — the plan's fixed
  number, not a new threshold).
- Gate placement follows the owner's PR #52 decision (comment
  2026-11-11T15:07:55Z,
  <https://github.com/dorrabbkhan/Epiora/pull/52#issuecomment-6799432942>),
  which replaces the plan's "perf job on PRs" wording for this task:
  the `perf` job runs on push to `main` (median of 5 samples,
  `rows=1000000`) and on `workflow_dispatch` (the `runs` input kept),
  never on `pull_request`, and always on the persistent, disk-backed
  Chromium profile — the `profile` dispatch input was removed so the
  ephemeral context (which fails 1M imports with SQLITE_IOERR) cannot
  be selected; the spec's `PERF_PROFILE=ephemeral` path remains for
  local diagnosis. PRs keep the size gates: the first-load budget
  (`app/scripts/budget.mjs`, < 5,000,000 bytes) in `app.yml` and the
  import-wasm 573,616-byte gate (`scripts/wasm-size.sh` with a limit
  argument) in `import.yml`.
- The perf gate lives in `app/perf/perf.spec.ts`'s last test (the
  thresholds live where the plan put them): it reads this invocation's
  `results.jsonl` lines (each line carries `profile`, `cpu` and a
  `batch` id so stale local lines can never pass or fail it), computes
  the median of each gated scenario's successful samples, writes
  `target/perf/gates.md` (appended to the step summary), and fails when
  a median exceeds its owner threshold or a gated scenario has no
  successful sample. A run at `PERF_ROWS` ≠ 1,000,000 records `*_<label>`
  names the thresholds do not apply to and the gate is not evaluated.
- D7 report (spec §10.2): the `perf` job runs a second pass with
  `PERF_CPU_THROTTLE=4` (`Emulation.setCPUThrottlingRate`) over the two
  gated scenarios, 3 samples; recorded as `import_1m_cpu4` /
  `open_tables_1m_cpu4` so they never contaminate the gated medians —
  the medians land in the step summary via `summarise.mjs`, not gated.
- The first-load budget step in `app.yml` measures a second `vite
  build` made without `VITE_TEST_HOOKS` (the flag changes the entry
  chunk) into `target/budget-dist` — the same two-build pattern the
  perf job uses; the step runs on every event including fork PRs.
- Files outside the Files block: `app/vite.config.ts` —
  `build.manifest: true` so `budget.mjs` reads the entry's JS/CSS from
  `dist/.vite/manifest.json` as the plan's interface specifies;
  `scripts/check-app.sh` — a `node --test scripts/*.test.mjs` step so
  `budget.test.mjs` runs in the local gate and CI (the plan runs the
  test directly with `node --test app/scripts`).
- `.github/workflows/import.yml` also gained `scripts/wasm-size.sh` in
  its path filters so changes to the gate script itself run the gate.
- Runner variance (owner decision, 2026-11-12, PR #55): the thresholds
  stay exactly as they are — `import_1m` ≤ 89,550 ms, `open_tables_1m`
  ≤ 2,874 ms, import-wasm ≤ 573,616 B, first-load < 5,000,000 B. For
  the perf gate on push to `main`, a single red `perf` run is rerun
  twice (`workflow_dispatch` or job rerun); it counts as a real
  failure only if a rerun fails again. No automatic retry is added to
  the workflow. Evidence — three `perf` runs on identical code:
  `import_1m` medians 99,939.9 ms (run
  [43119574950](https://github.com/dorrabbkhan/Epiora/actions/runs/43119574950),
  FAIL), 70,682.3 ms (run
  [43126454921](https://github.com/dorrabbkhan/Epiora/actions/runs/43126454921),
  PASS) and 63,782 ms (run
  [43126475469](https://github.com/dorrabbkhan/Epiora/actions/runs/43126475469),
  PASS); `open_tables_1m` medians 2,757.9 / 2,292.8 / 1,942.9 ms (all
  PASS).

### Test fix — perf gate batch id and keyboard-path timeout (2026-11-12)

Decision: Two CI/test fixes on branch `ci/fix-perf-batch-keyboard-timeout`
(owner-approved 2026-11-12, "follow your recos"); not a plan task.

- Perf gate batch id (bug fix): `app/perf/perf.spec.ts` stamped each
  recorded line with `batch = pid-${process.pid}` and CI set no
  PERF_BATCH. When a sample hit the 1,800,000 ms test timeout,
  Playwright replaced the worker, so the gate test ran in a new
  process and discarded every earlier sample. Evidence: main run
  <https://github.com/dorrabbkhan/Epiora/actions/runs/43208388072>
  attempt 1 — samples 1–4 passed, sample 5 timed out, and the gate
  reported "import_1m: no successful sample; open_tables_1m: no
  successful sample". Fix: the batch id is now the invocation id —
  `run-${process.ppid}` (the Playwright runner is every worker's
  parent, replacement workers included) — and `app.yml` sets
  PERF_BATCH on each perf step from run id + run attempt + pass
  (`-x1` / `-cpu4`), so the two passes and reruns never share a
  batch. Gate semantics, thresholds (89,550 ms; 2,874 ms; 573,616
  bytes; 5,000,000 bytes), sample counts and the timeout are
  unchanged. The gate evaluation moved verbatim to
  `app/perf/gate-eval.ts` so `app/scripts/perf-gate.test.mjs` (the
  existing `node --test scripts/*.test.mjs` lane) exercises it: the
  new test fails with the pid-scoped batch ("no successful sample")
  and passes after the fix.
- Keyboard-path Firefox timeout — owner-approved: keyboard-path test
  timeout 30 s → 60 s (this test only), via `test.setTimeout(60_000)`
  inside `app/e2e/keyboard-path.spec.ts`. The path runs ~20–24 s
  alone but hits the 30 s default inside AxeBuilder.analyze when the
  full suite runs. Evidence: main run 43208388072 attempt 1 (firefox,
  failed + retry) and PR #56 run 43250830143 (failed twice before
  passing). The global timeout, retries, workers and every other
  test are unchanged.
- Files outside the task's listed files: `app/perf/gate-eval.ts`
  (gate logic + batch id extracted from perf.spec.ts) and
  `app/scripts/perf-gate.test.mjs` (the required fail-before /
  pass-after test) — placed in the existing `node --test
  scripts/*.test.mjs` lane because a `*.test.ts` under `perf/` would
  run under Playwright's testMatch, and the Vitest unit project only
  includes `src/**/*.test.{ts,tsx}`.
Supersedes: —

### Release checkpoint and helpers (2026-11-12)

Decision: owner-requested release tooling ("release checkpoint doc +
helpers + GitHub form values", 2026-11-12); not a plan task. Branch
`docs/release-checkpoint`.

- `docs/releasing.md` — the owner's checklist (Phases 0–6 plus
  rollback and a filled-in values table for v0.3.0).
- `scripts/release/checkpoint.sh <version>` — the Phase 1 gate: on
  `main`, clean and up to date; all five app version fields equal the
  release version while the engine workspace version is explicitly
  left alone; tag/release still free; tag ruleset and its owner
  bypass; every `[OWNER GATE]`/`[OWNER REVIEW]` PR merged;
  `docs/validation/v<MM>.md` and the closing decisions.md entry
  present; the latest completed main-push run of every push-triggered
  workflow (app, desktop, import, engine, spikes) green on an ancestor
  of HEAD with no changes under that workflow's path filter since;
  the `perf` job's summary table read from its log; the three desktop
  artifacts present and the Linux smoke step green on the *ubuntu*
  matrix leg; Netlify secrets checked when the token can list them
  (MANUAL fallback otherwise). Writes
  `target/release/v<V>/{checkpoint.md,release-notes.md,ci.env}`; exit
  1 on any FAIL.
- `scripts/release/collect-assets.sh <version>` — refuses unless the
  checkpoint ran with 0 FAILs; downloads the three desktop artifacts
  of the checkpoint-verified run; copies installable bundles
  (.AppImage/.deb/.rpm/.msi/.exe/.dmg) into
  `target/release/v<V>/upload/` with `SHA256SUMS.txt`; refuses when a
  bundle filename's version ≠ the release version; prints the exact
  `gh release create/upload/edit` commands for the owner.
- `scripts/release/verify-deploy.py <url>` (stdlib only) — post-deploy
  checks: http→https redirect, `/` and `/index.html`, the SPA
  fallback, every `app/public/_headers` rule (CSP exact; Cache-Control
  compared without optional whitespace — Netlify re-serializes it),
  `/sw.js` and `/manifest.webmanifest`, a full asset crawl from
  index.html and the JS (every `.wasm` must be `application/wasm`),
  no `VITE_TEST_HOOKS` in any JS, the `test hooks are disabled` canary
  present. `--self-test` runs a compliant and a deliberately broken
  fixture site in-process.
- `scripts/tests/release.bats` — usage errors, the version-mismatch
  path (in a temp repo via `--versions-only` and `--check-name`), the
  FAIL-refusal and the verify-deploy self-test.
- Comment fix: `.github/workflows/desktop.yml` claimed Windows/macOS
  bundle legs run "on main pushes, manual runs and releases" — the
  workflow has no `release:` trigger, so publishing a release builds
  nothing; the comments now say "main pushes and manual runs" and note
  that release bundles come from the main-push run at the tagged
  commit. Comment only — no trigger or behaviour change.
- The helpers are read-only towards GitHub and Netlify: they never
  create a tag or release, never push, never upload assets, never
  change settings. Tagging and publishing stay owner-only.
Supersedes: —

### v0.3 Task 18 — v0.3 closed (Foundations proven)

- v0.3 delivered every feature its spec's F-table lists (F1–F15,
  `docs/superpowers/specs/2026-11-05-epiora-v0.3-app-shell-design.md`
  §3): deployed-preview PWA and unsigned desktop artifacts (F1, F10,
  F14); record-level change log shaped for v2.0 sync with re-import
  diff by record key (F2); `epiora-import`/`epiora-import-wasm` (F3,
  F13); one `Store` interface over OPFS and native
  SQLite3MultipleCiphers backends (F4); desktop encryption at rest
  (F5); dialog-driven Analysis workspace with persisted output log
  and receipts (F6, F12); preview-and-confirm import with saved
  settings (F7); token-driven design system with zero axe violations
  (F8); offline PWA with tab lock (F9); owner-picked performance and
  size gates (F11); the dedicated-worker app architecture (F15).
- Evidence: `docs/validation/v0.3.md` — generated by
  `app/scripts/validation-report.mjs` from the `main`-push runs for
  commit `e19042cdde36954b40461e390ebcbae5abfbbd0b` and committed as
  the CI artifact byte for byte. Source runs: `app`
  <https://github.com/dorrabbkhan/Epiora/actions/runs/43208388072>
  (attempt 3, green; attempts 1–2 facts stated in the report),
  `import`
  <https://github.com/dorrabbkhan/Epiora/actions/runs/43208388078>,
  `desktop`
  <https://github.com/dorrabbkhan/Epiora/actions/runs/43208388030>.
  Every gate passed: first-load 781,385 < 5,000,000 bytes; import-wasm
  521,469 ≤ 573,616 bytes; `import_1m` median 71,621.8 ≤ 89,550 ms;
  `open_tables_1m` median 2,263.3 ≤ 2,874 ms.
- The five platform-spec edits of v0.3 spec §14 were applied to
  `docs/superpowers/specs/2026-10-28-epiora-platform-design.md`
  exactly once each and nowhere else. Placement notes: edit 1's
  second half was already satisfied ("SPSS, Stata, SAS and Parquet
  import and export" was already under v1.5); edit 3 ("Hosting:")
  became decision row A9 in §6, the spec's decisions table — the
  platform spec has no hosting section; edit 4 landed as a "Store
  encryption" bullet on §8.3's on-device-store discussion plus a
  v0.3-posture clause on the D6 row; edit 5's "§4 of this spec" was
  qualified to "§4 of the v0.3 app-shell spec" so the reference does
  not point at the platform spec's own §4.
- Spec-edit placements — owner decision (2026-11-12, option B on
  PR #56): the five §14 edits stand as placed on the branch — edit 2
  → §12 item 8; edit 3 → decision row A9 in §6 (the platform spec
  has no Hosting section); edit 4 → §8.3 bullet + D6 row; edit 5 →
  §7.3, with "§4 of this spec" rendered as "§4 of the v0.3 app-shell
  spec" so the reference resolves. A9's rationale is corrected to
  "Deploys run from GitHub Actions after the gates; production
  deploys only on a published release" — the release-only production
  deploy decided in v0.3 Task 13.
- Netlify URL — owner decision (2026-11-12, option B): the site is
  <https://epiora-dev.netlify.app>, named after the site name in the
  preview deploy log of run
  <https://github.com/dorrabbkhan/Epiora/actions/runs/43198689215>.
  No production deploy has ever run: production deploys are
  release-only (v0.3 Task 13 decision) and no release is published,
  so the site holds no production deploy until the owner publishes
  the first release.
- Known follow-ups (not fixed in v0.3): `app/perf/perf.spec.ts` is
  ~769 lines (the ~400-line guideline); `app/scripts/budget.mjs` does
  not follow the entry's static imports; `grid_scroll_1m` crashed 0/5
  samples (DataGrid); the perf batch id is the test worker's pid, so
  one timed-out sample discards all samples in the gate
  (`app/perf/perf.spec.ts:80`); `import_1m` swings ~70 s vs ~100 s
  between runs of identical code (Task 17 run
  [43119574950](https://github.com/dorrabbkhan/Epiora/actions/runs/43119574950),
  e19042c attempt 2); `e2e/keyboard-path.spec.ts` runs at ~21–31 s
  against its 30 s timeout on Firefox under full-suite load.
- Files outside the Files block: `app/scripts/validation-report.test.mjs`
  — required by Step 1 (`report_lists_every_gate`) though not named
  in the Files block; `app/scripts/collect-validation-run.mjs` —
  the `gh api` collection half of the report generator, split out so
  `validation-report.mjs` stays under the ~400-line guideline.

### v0.3.0 released (2026-11-12)

- Release: <https://github.com/dorrabbkhan/Epiora/releases/tag/v0.3.0>, a
  pre-release published 2026-11-12T18:57:45Z by the owner. Tag `v0.3.0`
  points at `300434c28469993342c7270ff986bf9d8475feab`.
- Pre-release checkpoint: `scripts/release/checkpoint.sh 0.3.0` on
  `300434c` reported 0 FAIL, 0 WARN, 5 MANUAL. The owner did the manual
  items: tag-ruleset bypass, Netlify secrets and token, browser and
  desktop checks.
- Evidence (main push runs): app
  [43272671408](https://github.com/dorrabbkhan/Epiora/actions/runs/43272671408)
  on `94bd873`, perf gate: `import_1m` median 71,593.2 ms ≤ 89,550,
  `open_tables_1m` median 2,351.2 ms ≤ 2,874; desktop
  [43272817786](https://github.com/dorrabbkhan/Epiora/actions/runs/43272817786)
  on `5f493f1` (macOS leg re-run after a cancelled runner; Linux smoke
  passed). Import, engine and spikes runs as listed in the release notes.
- Assets: the six unsigned desktop bundles from desktop run 43272817786
  (AppImage, deb, rpm, NSIS setup exe, msi, dmg) plus `SHA256SUMS.txt`,
  collected by `scripts/release/collect-assets.sh`.
- Production deploy: release run
  [43285225716](https://github.com/dorrabbkhan/Epiora/actions/runs/43285225716)
  (success), Netlify deploy `6af60e4f8b28b699a219a43f`
  (<https://6af60e4f8b28b699a219a43f--epiora-dev.netlify.app>). The site
  <https://epiora-dev.netlify.app> now holds its first production deploy,
  superseding the "no production deploy until the first release" note in
  the v0.3 Task 18 entry.
- Post-release check: `scripts/release/verify-deploy.py
  https://epiora-dev.netlify.app` reported 36 checks, 0 FAIL.

### v1.0 Task 1 — calculator inventory (2026-11-13)

`docs/v1.0-calculators.md` inventories every OpenEpi 3.01 menu calculator and all
eight Epi Info 7 StatCalc calculators, with inputs, outputs, methods and R
references read from the live site source and from
`Epi-Info/Epi-Info-Community-Edition @ 4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6`.

- Counts per family: counts 10 (`proportion`, `two_by_two`, `two_by_n`, `r_by_c`,
  `dose_response`, `matched_case_control`, `screening`, `smr`, `binomial`,
  `poisson`), person_time 2, continuous 4, sample_size 4, power 3, other 1 — 24
  calculator ids, all from the plan's expected list, none renamed or added.
- Merges with reason: OpenEpi's Power → Cohort / Clinical Trial / X-Sectional are
  one computation with different labels → one `power_cohort` calculator with a
  `design` input. EI7 "Tables (2x2, 2xN)" is stratified 2×2 → `two_by_n`,
  separate from `two_by_two` because its output set is EI7's StatCalc block.
- Excluded items: OpenEpi `Calculator/calculator.htm` (arithmetic calculator, not
  statistical); Home, Info/Help pages, Language/Settings and Searches
  (navigation/documentation); "Internet Links" (external links), "Download OpenEpi"
  (download page) and the "Development" branch — Proposal, Toolkit Description,
  Translations, JavaScript Tips (project documentation). No statistical menu item
  dropped.
- OpenEpi terms verdict: automated access is allowed — the site has no terms of
  service beyond the Licensing/Disclaimer page (quoted in the doc), which grants
  unrestricted use under MIT wording, and no robots.txt. Each page was fetched
  once for this inventory; Task 3's capture workflow stays at one page at a time
  with ≥2 s between cases.
- Method differences recorded in the doc: OpenEpi's dose-response χ² includes a
  continuity correction that EI7 lacks; EI7's population-survey and binomial
  calculators have outputs OpenEpi lacks (per-cluster sizes, count-form CI);
  OpenEpi caps exact computations for speed (`n > 100 001` in Proportion, "large
  numbers" in TwobyTwo) where Epiora computes them; several calculators accept
  arbitrary confidence percentages in OpenEpi where spec §5.3 fixes Epiora's
  list at 90/95/99%.
- R package list for Task 2 (`renv.lock`): `exactci` 1.4-5 (GPL-3), `exact2x2`
  1.7.0 (GPL-3), `PropCIs` 0.3-0 (DESCRIPTION states `GPL`; CRAN expands to
  GPL-2 | GPL-3), plus already-pinned `epitools` 0.5-10.1, `epiR` 3.0.0 (renv
  pin; CRAN now serves 3.0.1), `DescTools` 0.99.60, `survival` 3.8.12
  (recommended package shipped with R 4.6.2, LGPL ≥ 2 — added in the fix round
  for `clogit`, the 1:n conditional-MLE reference) and base `stats`. Every
  `pkg::fn` in the doc was verified with `exists()` in R at these versions
  (output posted on PR #62; `survival::clogit` verified in the fix round, also
  posted on PR #62). Mid-p in `exactci`/`exact2x2` is `midp=TRUE`, not
  a `tsmethod` value. Each output row names exactly one method, one R reference
  (or "no R reference" for trivial/input-echo statistics) and one tolerance
  class. Methods without a published R function are named as
  `fixtures/R/methods/*.R` functions with their literature source (see the
  doc's "R packages" section).
- Owner decisions (2026-11-13, via the conductor, PR #62) on the two questions
  raised in review: (1) confidence levels keep spec §5.3's fixed list
  90/95/99 — the `conf_level` inputs are unchanged; (2) 1:n matching stays in
  v1.0 as Epiora's own implementation — only the 1:1 outputs are compared with
  OpenEpi and Epi Info 7; the `matched_case_control.matched_n.*` outputs are
  validated against R only. The 1:n input is a `controls_per_case` integer
  M = 1..4 plus a 2×(M+1) matched-set table (case exposure × number of exposed
  controls); the 1:1 pair table is its M = 1 special case. Two choices the fix
  flags for the owner in the PR body: the M ≤ 4 bound (relative efficiency of
  M controls per case is M/(M+1), reaching 80% at M = 4 — Breslow & Day 1980,
  ch. 5) and variable per-set ratios excluded (a 2×(M+1) set table cannot
  express mixed M; neither reference tool has it). Calculator counts are
  unchanged: 1:n adds outputs under the existing `matched_case_control` id.
### v1.0 Task 2 — R corpus v3

- Scope shipped: `fixtures/R/methods/*.R` (nine files, SPDX MIT header, plain
  side-effect-free functions), `fixtures/R/means.R`, `fixtures/R/calc.R`,
  MEANS datasets and fixtures, calculator fixtures, `generate.R` source
  order, `renv.lock`, fixture schema. No `crates/`, `app/` or `spikes/`
  file touched; `fixtures/R/special.R` and `fixtures/special/*` untouched.
- Deferred: the seven special.* grids (owner decision 2026-11-13, conductor
  escalation) — they land in a fixture-only PR sequenced with Task 5.
- Fixture counts: `fixtures/means/` 10 fixtures (ungrouped, grouped,
  include_missing, ties, one-obs group, constant, all-missing, dates,
  Oswego AGE, Oswego AGE×SEX) over 6 new `means_*.dataset.json` datasets
  plus `oswego`; `fixtures/calc/` 83 fixtures over 23 calculators (every
  calculator in `docs/v1.0-calculators.md` except `random_numbers`, which
  has no R reference — spec §5.2): proportion 5, two_by_two 5, two_by_n 3,
  r_by_c 3, dose_response 4, matched_case_control 4, screening 3, smr 4,
  binomial 3, poisson 3, rate_one 3, rate_compare 4, mean_ci 3, median_ci
  4, t_test_summary 4, anova_summary 4, ss_proportion 4, ss_unmatched_cc 3,
  ss_cohort 4, ss_mean_difference 3, power_unmatched_cc 4, power_cohort 3,
  power_mean_difference 3. Each calculator carries at least a textbook,
  boundary/zero-cell and large or option-variant case; every input-option
  combination in the inventory is covered (`sd_or` ×3 for mean_ci, the
  four ss_cohort alternates, the three power_cohort designs, conf-level
  variants 90/95/97/99).
- R versions (renv.lock): `exactci` 1.4-5, `exact2x2` 1.7.0, `PropCIs`
  0.3-0, `epitools` 0.5-10.1, `epiR` 3.0.0, `DescTools` 0.99.60,
  `survival` 3.8-12, R 4.6.2.
- Methods files and literature (header comment in each): `proportion.R`
  — Wilson score + FPC (Cochran 1977 §2.8), Fleiss score CC; `tables.R` —
  Rothman & Greenland direct pooling, Breslow & Day 1980, OR/risk
  etiologic fractions (Kahn/Sempos), stratified CMLE OR (Martin & Austin
  1991/1996, `MartinStats.js` port); `dose_response.R` — extended MH trend
  (Rosner 5th ed. p.606 CC; Schlesselman 1982 p.203 uncorrected);
  `matched.R` — pair OR b/c Taylor CI; `screening.R` — diagnostic odds
  (Simel et al. 1991 log method), entropy/bias/level LRs
  (DiagnosticTest.js); `poisson.R` — Byar (Rothman & Boice 1979),
  Rothman & Greenland, Ury–Wiggins 1985, Vandenbroucke 1982; `rates.R` —
  RD Byar, CMLE RR (Martin & Austin 1996), PEPI etiologic fractions
  (Abramson & Gahlinger), Breslow–Day rates 1987; `means.R` — mean CI
  with FPC, Conover percentile-rank CI, ANOVA/Bartlett from summary;
  `sample_size.R` — Kelsey et al. Table 12-15, Fleiss 3.18–3.19, Lwanga &
  Lemeshow (OpenEpi + EI7 population-survey forms), power z ±CC.
- MEANS conventions: quartiles are EI7's Tukey hinges (`fivenum()` — no
  single `quantile(type)` reproduces hinges for all n); mode is the
  smallest value among tied modes (EI7 `cDataSetHelper.cs`); dates are
  days since 1970-01-01 including negatives; a one-observation group
  emits undefined variance/std_dev (null leaves); "(missing)" is the last
  group when `include_missing` is true.
  - Correction (2026-11-13, owner decision 2.1 on PR #67): the quartile
    claim above is wrong — EI7 7.2.7.0's printed MEANS output matches
    `quantile(x, c(.25,.5,.75), type = 2)`, not Tukey hinges
    (`fivenum()` disagrees on the Oswego overall and Male quartiles).
    See the "v1.0 Task 2 follow-up — MEANS quartiles and field names"
    entry below.
- API notes recorded for the engine tasks: `epiR` 3.0.0 emits no
  prevented-fraction or per-group-risk outputs, so `etiologic.*` and
  `etiologic_or.*` come from `fixtures/R/methods/tables.R` (OpenEpi
  TwobyTwo.js formulas) and `risk.*` uses a Taylor proportion CI scaled
  ×100, matching `epiR::epi.2by2`'s percent units; `risk_difference` is
  `epi.2by2 ARisk.strata.wald` (percent). `epitools::rate2by2.test`
  returns only wald/midp p-values; `zscore.{value,p_1t}` uses the same
  z statistic it computes internally. Stock `mantelhaen.test` cannot run
  on K = 1, so single-table MH uses the corpus's existing `mh_2x2xk`
  port (bit-for-bit checked wherever the stock function runs).
- Files outside the Files block: `.github/workflows/fixtures.yml` —
  added `means` to the schema-validation family list and a `calc/<id>`
  depth-2 loop so the new fixture families are actually validated;
  fix round 3 added a `push` trigger on `main` with the same `paths`
  filter as `pull_request`, so fixture drift on main (like the
  `ods_dates/input.ods` drift this PR exposed) is caught instead of
  going unnoticed until the next PR.
- `fixtures/import/ods_dates/input.ods`: the bytes committed by
  6d26661 carried a foreign host's font list (drift first exposed by this
  PR's fixtures run — the fixtures workflow had not run since 6d26661).
  The file was regenerated with the pinned TDF 26.2.6.3 build; the
  output (git hash 9b3e712) is byte-identical on this host and on the CI
  runner, so the drift check is green again. Determinism verified: three
  consecutive `Rscript fixtures/generate.R` runs produced byte-identical
  fixtures otherwise.
- Green fixtures workflow run (regeneration + schema validation + drift
  check over all families):
  https://github.com/dorrabbkhan/Epiora/actions/runs/43348086896
- Generation: `Rscript fixtures/generate.R` run three times consecutively
  with no drift (`git status --porcelain fixtures/` unchanged after the
  second and third runs, tracked files only).
- Fix round 1 (PR #64 review):
  - `dose_response.R::xmh_trend` continuity correction changed from the
    signed `(v1sum - 0.5)^2` to the Rosner p.606 literature form
    `max(|v1sum| - 0.5, 0)^2`: the signed form *raised* the statistic
    when V < 0 (textbook: 3.7956 corrected vs 3.6198 uncorrected); a
    correction only shrinks |V|. The corrected statistic is now always
    <= the uncorrected one (textbook 3.4481 vs 3.6198; decreasing 40.74
    vs 41.49). Only pre-existing fixture whose values changed:
    `fixtures/calc/dose_response/textbook.json`.
  - 18 new calculator fixtures so every calculator has a textbook, a
    boundary/zero-cell and a large or option case: dose_response
    `decreasing` (V < 0 sign coverage + zero case) and `large`;
    rate_compare `boundary` (equal rates, RR = 1) and `large`; median_ci
    `boundary` (p = 5 — normal-approximation ranks leave [1, n],
    recorded as the reference output) and `large`; t_test_summary
    `boundary` (n1 = 2) and `large`; anova_summary `small_group`
    (a group at n = 2) and `large`; ss_proportion `small_population`
    (uncorrected n > N — FPC at the limit) and `large`;
    ss_mean_difference `small_effect` (diff 0.5) and `high_power`
    (99/99, ratio 2); power_unmatched_cc `small_effect` (31% vs 30%)
    and `large`; power_mean_difference `small_effect` and `large`.
    Boundary constraints of the MartinStats port discovered while
    picking cases: a person-time stratum with sum_a - min_a <= 1 makes
    the numerator polynomial degenerate (degree 0) and large case
    counts overflow the convolved coefficients — both error the
    generator, so such inputs are not fixtures.
  - OpenEpi MIT attribution (copyright + permission notice) added below
    the SPDX line in `fixtures/R/methods/tables.R` and `rates.R`, the
    two files that port `js/MartinStats.js` / PersonTime2 code.
  - Stratified CMLE cross-check — resolved at fix round 2. The 1e-6
    bound in the round-1 instruction was the conductor's, not a plan
    or fixture tolerance; it is unmeetable because stock
    `stats::mantelhaen.test(exact = TRUE)` finds its exact-CI roots
    with `uniroot`'s default tolerance (`tol` unset, so
    `.Machine$double.eps^0.25` ~ 1.2e-4). The generation-time check
    now lives in `calc.R::check_cmle_exact` at 1e-3 relative — a
    sanity check at stock's own convergence chosen by the conductor,
    not a fixture tolerance; it never changes a fixture value.
    Measured on the `stratified` fixture (port vs stock): est
    2.7651727076105361 vs 2.7651730451773968 (1.2e-7 rel);
    fisher_lower 1.5446268009946689 vs 1.5445617077397695
    (4.2e-5 rel); fisher_upper 5.0277823325556676 vs
    5.0270823927319812 (1.4e-4 rel) — all within 1e-3. The
    `r_default` variant (port run at stock's own uniroot tolerance)
    sits est 1.1e-5, lower 4.7e-5, upper 1.4e-4 rel from stock — the
    two root-finders land on different endpoints inside the same
    loose envelope. Against stock's own algorithm re-converged at
    `tol = 1e-14` the port agrees within 6.8e-15 rel
    (2.7651727076105344 / 1.5446268009946793 / 5.0277823325556668 vs
    the port's 2.7651727076105361 / 1.5446268009946689 /
    5.0277823325556676), produced on R 4.6.2 from the repo root by:

      ```sh
      Rscript - <<'EOF'
      source("fixtures/R/methods/tables.R")
      strata <- list(matrix(c(30, 70, 10, 90), 2, 2, byrow = TRUE),
                     matrix(c(5, 15, 20, 60), 2, 2, byrow = TRUE),
                     matrix(c(8, 12, 3, 25), 2, 2, byrow = TRUE))
      arr <- array(unlist(strata), dim = c(2, 2, 3))
      # stock's mantelhaen.test with tol = 1e-14 injected into its
      # six uniroot interval arguments; nothing else changes.
      src <- capture.output(print(stats::mantelhaen.test))
      src <- paste(src[!grepl("^<", src)], collapse = "\n")
      src <- gsub("c\\(0,\\s*1\\)\\)", "c(0, 1), tol = 1e-14)", src)
      src <- gsub("c\\(\\.Machine\\$double\\.eps,\\s*1\\)\\)",
                  "c(.Machine$double.eps, 1), tol = 1e-14)", src)
      f <- eval(parse(text = src))
      environment(f) <- environment(stats::mantelhaen.test)
      port <- stratified_cmle(strata, 0.95)$tight
      t <- f(arr, exact = TRUE)
      cat(sprintf("%.17g %.17g %.17g\n", unname(t$estimate),
                  t$conf.int[1], t$conf.int[2]))
      EOF
      ```

      Output: `2.7651727076105344 1.5446268009946793 5.0277823325556668`.
  - Fixtures workflow run (green on the round-1 head):
    https://github.com/dorrabbkhan/Epiora/actions/runs/43355570221
  - Fixtures workflow run (green on the round-2 head, covering the
    new `check_cmle_exact` generation-time check and a zero-drift
    regeneration):
    https://github.com/dorrabbkhan/Epiora/actions/runs/43358183499
- Fix round 3 (PR #64 review, owner-authorised third round) — CMLE
  reference edge cases plus the fixtures workflow on push to main:
  - Robust convolution. `poly_strat_cc`/`poly_strat_pt` now return
    log coefficients (`lchoose`-based, normalised to a max of 0);
    `mult_poly` is replaced by `logmult_poly` (convolution via
    log-sum-exp, the same O(n1*n2) shape, every product renormalised);
    `eval_poly` is replaced by `logeval_poly` (log-sum-exp
    evaluation — the linear P(r) is unrepresentable at the roots for
    large supports), and `martin_func` gets an exact `r = 0` branch
    from the lowest-degree nonzero coefficients. Existing fixture
    values moved only in the last ulps: 5 files changed —
    `fixtures/calc/two_by_two/stratified.json` (max 2.0e-14 rel),
    `rate_compare/textbook.json` (2.9e-15), `stratified.json`
    (5.5e-15), `boundary.json` (5.8e-15), `large.json` (2.2e-13,
    the round's maximum) — all under the 1e-12 stop bound. `git
    status --porcelain fixtures/` shows no other tracked-file
    changes and the second of two consecutive `Rscript
    fixtures/generate.R` runs is byte-identical (293 files, sha256).
  - Support boundaries are explicit in `martin_cmle`/`martin_lim`:
    observed sum at the support minimum gives estimate 0 and both
    lower limits 0; at the maximum the estimate and both upper
    limits are infinite and emit the corpus's existing
    undefined/null leaf (`{value: null, class}` — the engine
    answers `{"undefined": <reason>}`, where the reason is
    engine-side, as elsewhere in the corpus; the leaf schema
    carries no reason field). One step inside the boundary computes
    normally. `martin_root` now falls back to 1 when the crude
    `approx` is non-finite and returns `NA` (undefined leaf) rather
    than erroring when the bracket expansion overflows.
  - Generation-time stock cross-checks at 1e-3 relative (sanity
    checks, not fixture tolerances): `check_cmle_single_cc` runs
    `fisher.test(conf.int = TRUE)` on every two_by_two stratum and
    the crude table; `check_cmle_single_pt` runs the
    conditional-binomial identity (RR = (a/b)·(n0/n1), limits from
    `binom.test(a, a+b)$conf.int` mapped p → p/(1-p)·(n0/n1)) on
    every rate_compare stratum and the crude table. Measured
    agreement includes the boundary shapes — port vs stock at the
    support maximum: est Inf = Inf, fisher_lower 9.874124 =
    9.874124 (4e-15 rel), upper Inf = Inf.
  - `check_cmle_exact` is gated to `sum(arr) <= 5e4`: stock
    `mantelhaen.test(exact = TRUE)` itself silently overflows at
    large counts — measured on this corpus it still agrees with the
    port at ~8e4 total counts, drifts past the 1e-3 sanity bound by
    ~1e5 and returns a garbage estimate (~8e-5 for OR ~0.33 strata)
    by ~1.3e5 — so no stock reference exists beyond that gate. The
    `large_counts` stratified fixture instead uses two identical
    strata `[[1e6, 500], [2e6, 500]]`: the convolved CMLE then
    solves the same E_r[K] = a equation as the single-table
    `fisher.test` CMLE, keeping a real stock check at million counts
    (port est 0.5000001 vs stock 0.4999849, 3.0e-5 rel).
  - 8 new fixtures by category: `two_by_two` and `rate_compare`
    `zero_event` (support minimum → est 0, lower limits 0),
    `support_max` (→ est and upper limits null leaves),
    `one_inside_boundary` (degree-0 Fisher numerator — the round-1
    blocker — computes normally; its strata are sized so stock
    `fisher.test`'s looser boundary root, ~1.3e-3 relative on small
    tables measured against the port's 1e-14 root, stays inside the
    1e-3 sanity bound) and `large_counts` (counts in the millions).
  - `.github/workflows/fixtures.yml` gained a `push` trigger on
    `main` with the same `paths` filter as `pull_request`; nothing
    else in the workflow changed.
  - Fixtures workflow run (green on the round-3 head):
    https://github.com/dorrabbkhan/Epiora/actions/runs/43412250457
    (6m41s — regenerate fixtures, schema validation, Epi Info 7
    parser tests, no drift after regeneration)

### v1.0 Task 2 follow-up — R method fixes (2026-11-13)

Owner-directed follow-up, found while validating Task 3 (PR #65,
the OpenEpi capture): five method bugs in the Task 2 R reference
corpus. Fixtures still change only via `Rscript
fixtures/generate.R`; the run is deterministic (second run produced
no drift).

- **Power (`power_norm` in `fixtures/R/methods/sample_size.R`).**
  The helper computed `(1 - Q(|z|)/2)·100`; the inventory's quantity
  is one-sided normal power `Φ(z_pow)·100`. OpenEpi's `Norm(z)`
  returns the two-sided tail `2Q(z)`, so OpenEpi's
  `(1 - Norm(powerz)/2)·100` is `Φ(powerz)·100`; at `powerz ≤ 0`
  OpenEpi's own A&S polynomial mis-extrapolates (it shows 3.128 on
  `power_mean_difference/small_effect` where `Φ(z_pow)·100` is
  3.867). Now `power_norm <- function(powerz) pnorm(powerz) * 100`;
  all five callers share it. Evidence: `power_cohort/cohort` →
  63.9147 (OpenEpi 63.92), `rct` → 51.6075 (OE 51.61),
  `cross_sectional` → 72.9455 (OE 72.95),
  `power_mean_difference/textbook` → 70.5414 (OE 70.55),
  `small_effect` → 3.8672 (OE's own display 3.128 is its
  extrapolation artefact; stock `power.t.test(n=30, delta=0.5,
  sd=10, strict=TRUE)` → 5.4166 ≈ the owner's "≈5.4" cross-check —
  the leaf holds the named one-sided quantity, not the two-sided
  strict power). All power_* fixtures updated (15 leaves).
- **CMLE point estimates.** `cmle_or.midp.est` in `two_by_two`
  (every stratum and crude table) now uses
  `stats::fisher.test(tab)$estimate` — the conditional MLE,
  bit-identical to `exact2x2`'s estimate and 0/Inf at the support
  boundaries — instead of `epitools::or.midp`'s median-unbiased
  estimate (4.3089 → 4.38445 on textbook; OpenEpi 4.385). The mid-p
  limits stay `or.midp`'s where it runs (`or.midp` vs `exact2x2`
  mid-p limits agree to ~1e-4 on every corpus table); on zero cells
  where `or.midp` fails, `exact2x2(midp=TRUE)` supplies the same
  mid-p limits. The Fisher-central limits (`exact2x2`,
  tsmethod="central") are unchanged. Audit: `two_by_n`'s
  `odds_ratio_mle.midp.est` already used `exact2x2` (the CMLE);
  `matched_case_control.cmle_or.midp.est` is
  `exact2x2(paired=TRUE)`'s matched CMLE (= b/c, verified 2.6667 =
  8/3); `matched_n.cmle_or` is `survival::clogit` (the conditional
  MLE for 1:n sets); `rate_compare.cmle_rr.est` is the MartinStats
  port's conditional MLE (check_cmle_single_pt vs the
  conditional-binomial identity); `adjusted.cmle_or` is the
  stratified conditional MLE (check vs
  `mantelhaen.test(exact=TRUE)`). No change needed in those.
- **mean_ci t-row FPC.** `mean_ci_write` applied the
  finite-population correction `√((N−n)/(N−1))` only to the z-row;
  the t-row now applies the same factor (OpenEpi `CIMean.js:161`
  applies it to both). Only `mean_ci/variance_input`
  (population_size = 250) changes: t → [9.6868, 11.3132] from
  [9.6446, 11.3554].
- **Adjusted Mantel–Haenszel estimates.** `epiR::epi.mh` adds 0.5
  to every cell of any stratum containing a zero, which changes the
  estimator. `adjusted.mh_or.*` now uses
  `stats::mantelhaen.test(correct=FALSE)` — the uncorrected MH
  estimate and Robins–Breslow–Greenland CI, already generated for
  `mh_chisq` and verified bit-for-bit by `check_mh_port`
  (`one_inside_boundary` → 0.2420575; OpenEpi 0.2421).
  `adjusted.mh_rr.*` uses the new port
  `fixtures/R/methods/tables.R::mh_rr_2x2xk` (uncorrected MH RR with
  the Greenland & Robins 1985 log-variance CI — epi.mh's own
  formulas minus its 0.5 loop), checked bit-for-bit against
  `epi.mh` on every all-positive-stratum fixture by the new
  `check_mh_rr_port`. Zero-cell cases match OpenEpi's captures:
  `zero_event` → est 0, lower 0, upper undefined (OE 0/0/'?');
  `support_max` → 4.8467/2.9861/7.8666 (OE 4.847/2.986/7.866);
  `one_inside_boundary` → 0.2511/0.0347/1.8172 (OE
  0.2511/0.03471/1.817). Genuinely-undefined leaves stay null
  (`support_max` mh_or: Σbc/n = 0 → est Inf; `zero_event` mh_or CI:
  0/0 variance).
- **Zero-cell exact results.** Where `epi.2by2`/`or.midp` errored
  and a stock routine defines the quantity, the leaves now hold
  values: `cmle_or.midp.est` → conditional MLE 0 at support minimum
  (fisher.test; stays undefined — null — at support maximum, Inf);
  `cmle_or.midp.{lower,upper}` → `exact2x2(midp=TRUE)` limits
  (`zero_cell` [0, 1.0439], `zero_event` strata [0, 1.9672]/[0,
  1.0439], crude [0, 0.5682] — matching OpenEpi's captured
  0/1.967/1.044/0.5682); `risk_difference.*` →
  `DescTools::BinomDiffCI(method="wald")` ×100, verified
  bit-identical to `epi.2by2`'s `ARisk.strata.wald` on every
  non-zero-cell table. `two_by_two` and `two_by_n` fixture `source`
  blocks gain `DescTools 0.99.60` (already pinned in renv.lock; GPL
  ≥2). 42 previously-null leaves now hold defined values; all other
  nulls are mathematically undefined (log-Wald upper limits on zero
  cells, etiologic fractions, direct-OR at support max, clogit
  failures) and stay null.
- **Confidence-level leak (owner-directed round 4, same PR).**
  `two_by_two_table` and `two_by_n_table` called
  `epitools::riskratio`/`oddsratio` without `conf.level` (epitools
  defaults to 0.95), and `two_by_two_expected`/`matched_expected`
  called `stats::mantelhaen.test` without it. All now pass
  `conf.level = conf / 100`. Only `two_by_two/large`
  (conf_level 99) changed: `risk_ratio` → [1.5149191000281459,
  1.689859214232917], `odds_ratio` → [1.847354820987517,
  2.1652581055661906] on the stratum and the crude table — the
  closed-form log-Wald 99% limits computed independently
  (`exp(log RR ± qnorm(0.995)·√(1/a−1/n1+1/c−1/n0))` and the 1/cell
  OR form) are bitwise the new leaves, and OpenEpi displays
  1.515–1.69. The mantelhaen.test sites had no non-95
  multi-stratum/matched case yet, so no other fixture moved.
  Sweep of every interval-producing call in `fixtures/R/*.R` and
  `fixtures/R/methods/*.R` (epitools, epiR, exactci, PropCIs,
  exact2x2, DescTools, stats::* .test calls, survival::clogit,
  own helpers taking z or conf): all other sites already receive
  the case level — `conf / 100` where the API wants a fraction
  (`conf = cl` for epitools' pois.*), percent for the own
  z-helpers, and the round-3 sanity checks take `conf` as a
  fraction because their callers pass `conf / 100`
  (check_cmle_exact, check_cmle_single_cc/pt). Calls that never
  read `$conf.int` (fisher/mcnemar/chisq p-values, clogit's
  hand-built z CI, mh_2x2xk's statistic-only uses,
  check_mh_port's stock-vs-port identity at 0.95 on both sides)
  need no level. Fixed-95-by-design corpora (2x2, stratified,
  freq, means §4.2, binomial `ci_95_count`, anova `qt(0.975)`)
  keep 0.95 — those fixtures have no conf_level input. New
  generation-time guard `check_conf_leak` (fixtures/R/calc.R):
  every conf-taking calc writer rebuilds its expected at 95, and
  for `conf_level ≠ 95` each interval leaf must differ bitwise —
  demonstrated to stop generation on the unfixed code, naming the
  eight leaked leaves, and to pass after.
- **Inventory rows amended in `docs/v1.0-calculators.md`**
  (R-function column + note only):
  `calc.two_by_two.<s>.risk_difference.*`,
  `calc.two_by_two.<s>.cmle_or.midp.*`,
  `calc.two_by_two.adjusted.mh_or.*`,
  `calc.two_by_two.adjusted.mh_rr.*`,
  `calc.two_by_n.<s>.risk_difference.*`.
- **Changed-leaf census** (fixtures/calc): 22 fixture files, 99
  leaf-value changes + `source.packages` (DescTools) on the 12
  two_by_two/two_by_n files: 15 power leaves (fix 1), 2 mean_ci t
  leaves (fix 3), 15 `cmle_or.midp.est` recomputed on non-boundary
  tables (fix 2), 6 `cmle_or.midp.est` + 15 `cmle_or.midp` limit
  leaves filled on zero-cell tables (fix 5), 24 `risk_difference`
  leaves filled (fix 5), 22 `adjusted.mh_or`/`mh_rr` leaves
  (fix 4). Round 4 adds 1 file, 8 leaf-value changes:
  `two_by_two/large` `risk_ratio.{lower,upper}` and
  `odds_ratio.{lower,upper}` on stratum 1 and crude (fix 6). No
  other leaf changed.
- **Files outside the Task 2 Files block:** none — edits confined
  to `fixtures/R/methods/sample_size.R`,
  `fixtures/R/methods/tables.R`, `fixtures/R/common.R`
  (`check_mh_rr_port`), `fixtures/R/calc.R`, the regenerated
  `fixtures/calc` JSONs, `docs/v1.0-calculators.md` and this file.
  `fixtures/R/` and `docs/` are Task 2's own files; no crates/, app/,
  spikes/, fixtures/openepi/, fixtures/special/ or
  fixtures/R/special.R was touched.

### v1.0 Task 2 follow-up — MEANS quartiles and field names (2026-11-13)

Owner decision 2.1 on PR #67 (v1.0 Task 4): a fixtures-only PR moves
`fixtures/R/means.R` to `quantile(type = 2)` and renames the R MEANS
leaves to the plan's Task 6 "Produces" names, regenerated with
`Rscript fixtures/generate.R`.

- **Owner decision.** PR #67 comment "Owner decisions (given to the
  conductor in session, 2026-11-13)", decision 2, option 1: the
  quartile oracle is Epi Info 7's printed output; `quantile(type = 2)`
  replaces `fivenum()`, and the Task 6 field renames fold into the same
  fixtures PR.
- **Evidence.** The Task 4 capture (`fixtures/ei7/oswego.json` on
  `v1.0/task-4-ei7-means-reference`, EI7 7.2.7.0, drift-verified by run
  https://github.com/dorrabbkhan/Epiora/actions/runs/43500418310)
  matches `quantile(x, c(.25,.5,.75), type = 2)` on all nine Oswego
  quartiles — the only R definition that does; `fivenum()` prints
  16.5/57.5 where EI7 prints 16/58 (overall) and 16/61 where EI7 prints
  15/62 (Male). After regeneration, all 61 mapped `means.*` keys
  compare at `ei7_display` (`0.5 × 10^−d`): **59 agree, 2 disagree** —
  the disagreements are only `means.t_test.welch.diff_ci_95.{lower,
  upper}` (EI7 −10.5528/10.4663 vs R −10.5564/10.4699), the separate
  owner-decision-3 investigation; they are unchanged here.
- **Renames (Task 6 "Produces").** `minimum`→`min`, `maximum`→`max`;
  `t_test.{pooled,welch}.p`→`p_value` and `.{lower,upper}`→
  `diff_ci_95.{lower,upper}` (v0.2 `Ci` field names);
  `anova.p`→`p_value`; `bartlett.chisq`→`statistic`,
  `bartlett.p`→`p_value`; `kruskal_wallis.p`→`p_value`. Every means
  fixture gains an `overall` GroupStats block over all non-missing
  (analysed) values, matching `MeansResult.overall`.
- **Leaf census.** No leaf removed; each of the 10 fixtures gains the
  11 `overall` leaves. Value changes are confined to quartiles where
  hinges differ from type 2: `means_oswego_age` groups[0] 16.5→16,
  57.5→58; `means_oswego_age_sex` groups[1] (Male) 16→15, 61→62;
  `means_grouped` and `means_include_missing` six p25/p75 leaves each;
  `means_dates` groups[1] −3484.5→−3487, −3477→−3472.
  `means_all_missing`, `means_one_obs_group`, `means_constant`,
  `means_ties`, `means_ungrouped` are unchanged apart from the renames
  and `overall` (n = 0 and n = 1 handling as before:
  `means_all_missing` keeps `groups: []` and gains an all-null
  `overall`; the one-observation group keeps null variance/std_dev).
- **Ambiguities left for Task 6** (shape kept, not invented): what
  `groups` holds when ungrouped (kept: a single group, with `overall`
  duplicating it); the top-level `total`/`missing` leaves vs Task 6's
  `exclusions` map (kept as-is); fixture `groups[i]` carries no `label`
  leaf — groups are positional, as in the ei7 key mapping.
- **Files outside the Task 2 Files block:** none — edits confined to
  `fixtures/R/means.R`, the regenerated `fixtures/means/*.json` and
  this file. `fixtures/schema/fixture.schema.json` needed no change
  (`expected` is a free-form node tree; it constrains no leaf names).
  No crates/, app/, spikes/, fixtures/ei7/ or other fixture directory
  touched; every other fixture regenerated byte-identical.

### G8 — Roadmap review from user signals (2026-11-13)
Decision: owner accepted every proposed roadmap change, including v1.0. v1.0 gains Tasks 22 (engine assumption
notes, OWNER REVIEW of rules), 23 ("which calculator?" guide, notes on cards and in explain) and 24 (public
validation page); Task 11 depends on 22; Task 21 on 23 and 24 and applies only v1.0 spec §14 items 3–4. Phase 1
reordered: v1.1 Epi Info projects + data management; v1.2 regression, DAG helper (D11), read-only
REDCap/Kobo/ODK bridge, SAS/Stata scripts (D10); v1.3 forms and entry with phone entry and file merge; v1.4
dashboards and maps; v1.5 survival, complex samples, growth; v1.6 reports, R package, fr/es/pt, polish (parity).
v2.2 full connectors; v3.3 adds causal and latent-variable methods; v4.4 audits the validation dossier (public
from v1.0). First pilot: an FETP or health department with legacy Epi Info data. The open decisions on earlier
phone collection and earlier fr/es/pt are answered by v1.3 and v1.6.
Evidence: docs/research/d9-competitor-scan.md, "User signals" (r/epidemiology thread, pasted by the owner).
Supersedes: platform spec §14 Phase 1 order (2026-10-28); v1.0 spec G2's v1.1 content.

### v1.0 Task 3 — OpenEpi reference capture (2026-11-13)
- OpenEpi version 3.01 (openepi.com). Terms: the Task 1 entry records automated access as allowed
  (MIT-worded Licensing/Disclaimer page, no robots.txt, no prohibiting ToS), so the plan's human-step
  STOP does not apply. Capture is polite: one page at a time, >=2 s between cases.
- `cases.json`: 79 cases whose ids equal the Task 2 `fixtures/calc/<id>/<case>.json` ids for the 20
  calculators OpenEpi can represent. `binomial`, `poisson`, `two_by_n` and `random_numbers` have no
  OpenEpi page, so they are absent from the OpenEpi reference; their R fixtures still govern.
- `labels.json`: 893 emitted labels, 429 mapped to canonical `calc.<id>.<path>` statistic ids; the
  rest are `null` — input echoes, derived table cells, displays with no corresponding R fixture
  leaf, and the round-2 unmapped different-quantity labels (recorded "shown but not compared" per
  spec §9). Counts:
  `python3 -c "import json; lj=json.load(open('fixtures/openepi/labels.json')); print(sum(len(v) for v in lj.values()), sum(1 for v in lj.values() for x in v.values() if x))"` → 893 429
  (round 1: 893 437 — 8 mappings nulled and 2 retargeted in the same-quantity audit).
- Section-label rule (round 1): a label that repeats under more than one section is qualified by its
  section context in the emitted label — `1 / Uncorrected chi square`, `Crude / Risk in Exposed`,
  `Adjusted / Mantel-Haenszel Summary Chi Sqr`, `Statistics / d.f. / p-value (2 sided) / Byar
  approximation`, `Lower CL / SMR / Upper CL / Byar approximation` — so no flat label ever maps
  ambiguously. Labels with no trustworthy R statistic (screening multi-level cut-off rows, the SMR
  Byar z column, dose-response input echoes) map to `null`, never to a wrong statistic.
- Percent rule (round 1): displays on a 0-100 scale whose fixture leaf is a 0-1 fraction are stored
  divided by 100 with decimals +2, leaving the 0.5×10^-d tolerance exactly unchanged. Applies to
  `proportion` (multiplier 100), `screening` sensitivity/specificity/ppv/npv/accuracy/entropy,
  `two_by_two.adjusted.direct_rd` and `power_cohort.risk_difference`. Power estimates themselves are
  percent-scale in the R fixtures and are NOT converted; strata/crude risk differences are
  percent-scale in both and are NOT converted.
- Person-time rule (round 1): OpenEpi picks its own display unit ("Per N Person-Time Units"). The
  scraper reads N from the panel and restates rate values on the case's `inputs.per_unit`
  (value × N/per_unit, decimals −log10(N/per_unit)); the directly adjusted RD, stored per single
  person-time unit in the R fixture, is stored divided by N.
- Input guard (round 1): before scraping, the capture reads back `dataMatrix` (multiplicity-aware
  subset — sample-size pages legitimately write derived values back) plus `Evals` (dose-response
  exposure scores live outside data cells) and `ConfLevel`, and fails the run non-zero if OpenEpi
  did not hold every written input.
- Bounds: `<0.0000001`-style displays, `'?'` and `'undefined'` are stored as displayed text with
  `statistic: null` — never parsed as equalities.
- `--compare-r` resolves every mapped statistic path into `fixtures/calc/<id>/<case>.json` and
  reports |openepi − R| against 0.5×10^-decimals. A mapped path with no leaf in that case's
  fixture is listed as NO-R-COUNTERPART and does not fail the run (round 2 rule): it means the
  case's fixture does not cover that statistic. A missing fixture file still fails as UNRESOLVED
  (non-zero exit). Fixture leaves whose value is null are listed as R-NULL, numeric disagreements
  are listed for the owner (spec G7).
- Unrequested-emission rule (round 3): OpenEpi's Proportion page has no "no comparison" state —
  the "Compare to %" cell ships with 50.0 (Proportion.htm `data:50.0`) and Proportion.js always
  runs the one-sample z-test on whatever the cell holds. A case without `compare_with` now leaves
  the cell in that shipped state (the `''` write was a state no user produces — `parseFloat('')`
  is NaN and the page displayed "Two-sided p-value=2"), and a driver `unrequested(stat, inputs)`
  predicate marks emissions OpenEpi always produces although the case did not request them —
  `calc.proportion.ztest.*` when `compare_with` is absent — which are recorded with
  `statistic: null`. This removed the corpus's only NO-R-COUNTERPART (`proportion/zero_numerator`
  ztest).
- Same-quantity audit (round 2): every mapped label was re-checked against OpenEpi's own
  JavaScript (local site mirror) and corrected where the display is a different quantity than
  the fixture leaf: `t_test_summary` "Test for equality of variance2 [1..4]" → `null`
  (OpenEpi's Hartley F test divides the larger variance by the smaller and reports the df of
  the larger-variance group first — t_testMean.js:124-137,216 — never the input-order
  `f_variance.*` ratio/df pair, and its displayed p is its own FishF approximation, not the
  leaf's `2·min(pf,1−pf)`); `rate_compare` per-stratum/crude "z-score [1]" → `null` (OpenEpi
  absolutizes the z statistic — PersonTime2.js:485 `zstat=Math.abs(zstat)` — while
  `zscore.value` is signed; the p-value columns [2]/[3] stay mapped because both compute the
  p on |z|); `dose_response` "Extended Mantel-Haenszel chi square for linear trend" retargeted
  from `xmh.uncorrected` to `xmh.cc` (OpenEpi applies the continuity correction to the signed
  numerator `(V1sum−0.5)²` — DoseResponse.js:144 — so its display is the corrected statistic;
  it equals `xmh.cc` exactly when V1sum>0 and inflates the statistic when V1sum<0, recorded as
  a genuine disagreement). Everything else verified same-quantity: Fisher/Mid-P two-sided p is
  OpenEpi's `min(1, 2×min tail)` convention (MartinStats.js:670-682) vs R's probability-ordered
  p — a method difference on the same estimand; power `Norm(z)` leaves `Math.abs` commented
  out (PowerMean.js:35) so negative powerz scores below 50%; ss_mean_difference's power lookup
  covers only {95,90,80,70,60} (SSMean.js:63-68), leaving power=0 for other requests.
- Tooling: Playwright 1.65.0, latest release at task time, licence Apache-2.0 per its package
  metadata, pinned in `fixtures/openepi/package-lock.json`; Chromium only.
- `captured_at` is preserved across reruns so a no-drift rerun is byte-identical (ei7.yml precedent);
  output files are produced only by openepi.yml and are never hand-edited.
- Notes: `.github/workflows/openepi.yml` carried a temporary `push` trigger on this branch so the
  workflow can run before it existed on the default branch (v0.2 Task 3 precedent); it was removed
  in the final commit, leaving `workflow_dispatch` + a monthly schedule.
- Producing runs: https://github.com/dorrabbkhan/Epiora/actions/runs/43439924337 (round-1
  regeneration — artifact committed byte-for-byte; the run itself stopped at schema validation,
  which exposed and fixed the negative-decimals/`value` union schema gaps) and
  https://github.com/dorrabbkhan/Epiora/actions/runs/43441647872 (green: capture, schema
  validation, drift check — regenerated output byte-identical to the committed reference).
  Round 2: https://github.com/dorrabbkhan/Epiora/actions/runs/43448188046 (capture produced the
  committed artifact byte-for-byte; only the drift step failed because the remapped output was not
  yet committed) and https://github.com/dorrabbkhan/Epiora/actions/runs/43449861312 (green:
  capture, schema validation, drift check — regenerated output byte-identical to the committed
  reference). Round 3: https://github.com/dorrabbkhan/Epiora/actions/runs/43471843290 (capture
  produced the committed `proportion.json` byte-for-byte — the only changed output; only the drift
  step failed because the regenerated output was not yet committed) and
  https://github.com/dorrabbkhan/Epiora/actions/runs/43473532537 (green: capture, schema
  validation, drift check — regenerated output byte-identical to the committed reference; the
  input guard held on all 79 cases). `--compare-r` on the committed corpus against post-#66
  `fixtures/calc`: `ok 1003 mismatch 279 unresolved 0 no-r-counterpart 0 r-null 0`, exit 0 —
  all R-BUG/R-NULL rows resolved by #66's six fixes (pre-#66: `ok 949 mismatch 303 unresolved 0
  no-r-counterpart 0 r-null 30`).
- Files outside the Files block: `.github/workflows/fixtures.yml` (schema-validates the new family
  and excludes `fixtures/openepi` from the generate.R drift check — otherwise regeneration flags a
  family it does not own); `fixtures/openepi/drivers.mjs` (per-page input driving/validation —
  capture.mjs stays the runner); `fixtures/openepi/capture.test.mjs` (unit tests for the capture's
  pure functions — parsing, emissions, scaling, compare — required by the tests-ship-with-code
  rule); `fixtures/openepi/package.json` and `package-lock.json` (pin the required latest
  Playwright for this fixture tool only).

### v1.0 Task 5 — special functions (t, F, gamma/chi-square quantiles, Poisson tails, binomial quantile)

- Grid tests replaced (owner decision 2026-11-13, option a): the seven
  `special.*` grids were deferred out of Task 2 because they break the
  corpus gate before the runner knows the names, and the fixture guard
  forbids `fixtures/` in a `crates/` PR. This PR therefore replaces the
  plan's `*_matches_grid` tests with `*_matches_reference` tests whose
  values come from a single `Rscript -e` command (R 4.6.2 `pt`, `qt`,
  `pf(lower.tail=FALSE)`, `qgamma`, `qchisq`, `ppois`, `qbinom`), quoted
  verbatim in a comment block at the top of the section in
  `crates/epiora-core/tests/special.rs`. Tolerances are the plan's
  classes only: closed-form 1e-9 relative, root-found quantiles 1e-6
  (rule E7), `binomial_quantile` exact; plus the plan's proptests
  `t_quantile_symmetry` (1e-9 relative) and `poisson_tail_complements`
  (1e-12 absolute). The follow-up fixtures-only PR adds
  `fixtures/R/special.R` and the generated `fixtures/special/*.json`
  grids (OWNER REVIEW); `run_special.rs` already accepts all eight new
  names (`t_cdf`, `t_sf`, `t_quantile`, `f_sf`, `gamma_quantile`,
  `chisq_quantile`, `poisson_tail`, `binomial_quantile`) in the
  `inputs.grid` shape that script emits, verified by an in-memory
  dispatch test, so that PR needs no `crates/` change.
- Implementation notes: quantile solvers run Brent in shifted log
  coordinates (`v = ln x − u_est`); the root then sits near 0 where
  `brent_root`'s absolute 1e-14 tolerance is reachable — at |ln x| ≳ 45
  the ulp alone exceeds it. `t_quantile` inverts the beta relation via
  `beta_quantile` with a residual check and log-space fallback.
  `t_quantile(0.5, df)` returns the exact median 0; the reference table
  uses 0 rather than R's printed Newton residual 2.57e-16 for
  `qt(0.5, 0.5)` (noted in the test table). `binomial_quantile` resolves
  `p ≤ 0`/`p ≥ 1` before the degenerate `prob` cases, matching
  `qbinom(1, 10, 0) = 10`. `poisson_tail(upper)` encodes `P(X ≥ k)`
  (`ppois(k−1, λ, lower.tail=FALSE)`), the corpus numeric-`upper`
  convention.
- Fix round 1 (PR #69 review): `gamma_quantile`/`t_dist::log_solve`
  bracket expansions now double each step and evaluate the bound point
  itself on a crossing (a huge `u_est` ≈ −1e300 made `hi += 4` hang);
  `gamma_quantile` returns `Ok(0.0)` when P(a, min subnormal) ≥ p —
  the f64 quantile is 0. `binomial_quantile` searches `pbinom`'s
  `pbeta` identity `I_{1−prob}(n−k, k+1)` (O(log n) constant-time
  evaluations, was O(n·log n)); verified exact against R's `qbinom`
  on the reference table plus 21 large-n spot checks. `poisson_tail`
  uses `saturating_add` for `k + 1` (u64::MAX debug panic).
- Fix round 2 (PR #69 second review), method changes:
  `binomial_cdf` uses `pbeta(prob, k+1, n−k, lower.tail = FALSE)`
  through a new `reg_inc_beta_upper` — the upper incomplete-beta tail
  computed directly, not `1 − I` (a ~1e-15 tail resolved); plus R
  `qDiscrete_search`'s left-continuity fuzz `p *= 1 − 8ε`, giving
  exact `qbinom` parity on a 210-point seeded R table.
  `t_cdf`/`t_sf` follow modern `pt.c`: `nx = 1 + (t/df)·t`, the
  small-argument `pbeta` tail in both directions, and the A&S 26.5.4
  log form only when `nx > 1e100` — replacing the A&S 26.7.8 branch
  (4.2e-8 relative error at df = 1e6 in the deep tail).
  `t_quantile` for df > 4e5 solves `tail_val(t) = 2·min(p, 1−p)` on
  `v = ln t` — inverting the same function `pt` evaluates — because
  `x = df/(df+t²)` is then within ulps of 1 for every ordinary `t`.
  `f_sf` picks R `pf.c`'s argument (`z` vs `w` side) so the beta
  argument never rounds to 1. `gamma_quantile`/`chisq_quantile` for
  `p > 0.5` solve `Q(a, y) = 1 − p` on `inc_gamma_upper` — `P − p`
  cancels below ~1e-8. `reg_inc_beta`/`reg_inc_beta_upper` are now a
  TOMS 708 port (R `toms708.c`: `beta_large.rs` L30 dispatch/
  `bfrac`/`basym`, `beta_small.rs` `fpser`/`apser`/`bpser`/`bup`,
  `beta_grat.rs` `bgrat`+`gam1`/`gamln1`/`rexpm1`/`erf__`/`digamma`,
  `beta_math.rs` `brcmp1`/`algdiv`/`rlog1`/`bcorr`/`erfcx`) — the
  plain continued fraction was wrong by ~7e-4 in mixed
  small/huge-parameter regions; verified against R `pbeta` on a
  9,261-point grid and on the 256-point pt/pf/qt audit at df up to
  1e300.
- Fix round 3 (PR #69 third review, owner-authorized option 1), method
  change: `inc_gamma_lower` no longer forms `1 − inc_gamma_upper` in
  the region `x ∈ (0.8(a+50), a−1]` — that double-complemented through
  `1 − lower_series` and floored P ≲ 2·DBL_EPSILON
  (`gamma_quantile(1e-16, 1e4, 1)` 7.2e-4 off R, `chisq_quantile`
  1.0e-3 off). The lower tail is now computed directly as R 4.6.2
  `pgamma_raw` does, branch for branch, in the new
  `special/gamma_lower.rs`: `lower_series` for `x < 1`,
  `pd_upper_series × dpois_wrap` for `x ≤ a−1 && x < 0.8(a+50)`, the
  `1 − d·sum` branch (`dpois_wrap` + `pd_lower_cf`/`pd_lower_series`)
  for `a−1 < x && a < 0.8(x+50)`, and `ppois_asymp` (Temme expansion
  with `logcf`/`log1pmx`/`lgamma1p`/`dnorm4`, pnorm via `normal.rs`)
  otherwise — never a complement of a value near 1. Symmetric audit:
  `inc_gamma_upper` for `a < 1 && x < a+1` floored Q (negative values
  at a ≲ 1e-13, e.g. −2.2e-16 vs R's +2.19e-16 at a=1e-15, x=1); it
  now delegates to `inc_gamma_upper_lt1` (`pgamma_smallx` `!lower_tail`
  for `x < 1`, R's `d·sum` form otherwise) — post-fix audit vs R:
  0 negatives, worst rel diff 1.1e-14. OLD-vs-NEW `inc_gamma_lower` on
  an ordinary grid (a 0.1..1e4, x around the bulk): max bulk rel diff
  1.28e-9 at a=1e6, x=997500 where NEW matches R to ~1e-18 (the diff
  is OLD's error); deep-tail max 1.0 where OLD returned 0 and NEW
  returns ~9e-18 matching R; 0 outside the delegated region. New test
  `gamma_lower_tail_matches_reference` (30 `pgamma` + 20 `qgamma` +
  20 `qchisq` rows, one `Rscript -e`, verbatim output quoted in the
  test; classes 1e-9/1e-6 per the plan).
- Fix round 4 (PR #69 fourth review, owner-authorized option 1), method
  change: `inc_gamma_lower`/`inc_gamma_upper` are now R 4.6.2
  `pgamma_raw` end to end — both tails and both output modes through
  one dispatcher, like `reg_inc_beta` = TOMS 708. `pgamma_raw`
  (rewritten `special/gamma_lower.rs`) runs R's branches verbatim in
  `log_p` and non-log form — `pgamma_smallx` (`f1·f2` lower form,
  `−(f1m1 + f2m1 + f1m1·f2m1)` upper), `pd_upper_series·dpois_wrap`,
  the `d·sum`/`1 − d·sum` branch, `ppois_asymp(a−1, x, !lower_tail)` —
  then applies R's `res < DBL_MIN/DBL_EPSILON` redo: recompute in log
  space and exponentiate, which recovers the denormal digits the
  direct path underflows away (all five review denormal points now
  bit-identical to R, verified by `git show e8f6f97`-era vs NEW probe:
  0 ulps). `inc_gamma_upper` for `a ≥ 1` now routes through those same
  branches instead of `upper_cf` (deleted with `lower_series`,
  dead) — `chisq_sf` at df = 1e6, x = 1.002e6 is now bit-identical to
  R and x = 1.005e6 is 3.1e-13 relative (were 1.7e-9/2.3e-9). New
  files: `special/gamma_ebd0.rs`
  (R `ebd0.c`/`dpois.c` `ebd0` + `bd0_scale` table — the log-space
  Poisson density `dpois_wrap` calls when `log_p`), and
  `special/gamma_asymp.rs` (`ppois_asymp` both modes, `dpnorm`,
  `dnorm_std` split-form density; `normal.rs` gains `pnorm_log` +
  `do_del_log`, R `pnorm.c` `pnorm_both` log branch). OLD-vs-NEW on an
  ordinary grid (a ∈ {0.05..1e4}, x = a·r and fixed points, 1456
  points, `/tmp/task5fix/probe`): max rel diff P 1.03e-13 (deep-tail
  redo band), Q 2.45e-11 at a=1e4 x=10500 — the intended
  `ppois_asymp` correction toward R. New tests:
  `gamma_denormal_matches_reference` (212 rows: the 5 review points +
  `a` 1e-15..1e6 both tails down to 1e-321, one `Rscript -e`,
  verbatim + machine-converted), `gamma_upper_large_df_matches_reference`
  (112 `pchisq(·, lower.tail=FALSE)` rows, df 2..1e6, one
  `Rscript -e`), `inc_gamma_stays_in_unit_interval` (proptest:
  `a ∈ [1e-300, 1e7]`, x ≥ 0: P, Q ∈ [0,1] and `P+Q` within 1e-12 of 1
  where both ≥ 1e-3). No fixture, golden, corpus or existing-test
  value changed.
- Fix round 5 (PR #69 fifth review), constant corrections: `gamma.rs`
  `M_CUTOFF` corrected to R `pgamma.c`'s `M_LN2*DBL_MAX_EXP/DBL_EPSILON`
  = 3.196577161300664e18 (was 3.196577180618028e18), with a unit
  assertion that the const equals `LN_2*1024/ε`; `gamma_ebd0.rs`
  `X_LRG` set to the double nearest R `dpois.c`'s
  2.86111748575702815380240589208115399625e+307,
  i.e. 2.8611174857570283e307 (was 1 ULP low). The
  `PGAMMA_DENORMAL_REF`/`CHISQ_SF_LARGE_DF_REF` literals were
  machine-shortened to shortest round-trip form (342 of 1184 rewritten,
  verified bit-identical by parse-compare) and their two
  `#[allow(clippy::unreadable_literal, clippy::excessive_precision)]`
  attributes removed.
- Files outside the Files block: `docs/decisions.md` (this entry, as the
  owner note requires); `crates/epiora-core/src/special/beta_large.rs`,
  `crates/epiora-core/src/special/beta_small.rs`,
  `crates/epiora-core/src/special/beta_grat.rs` and
  `crates/epiora-core/src/special/beta_math.rs` (new — the TOMS 708
  incomplete-beta port could not fit in `beta.rs` under the ~400-line
  file rule; `beta.rs` keeps the public API and dispatch);
  `crates/epiora-core/src/special/gamma_lower.rs` (new — the R
  `pgamma_raw` lower-tail port plus `ppois_asymp` and the small-`a`
  direct upper tail could not fit in `gamma.rs` under the ~400-line
  file rule; `gamma.rs` keeps `ln_gamma`, the shared series/CF helpers
  and the `inc_gamma_upper` dispatch);
  `crates/epiora-core/src/special/gamma_asymp.rs` (new — round 4:
  `ppois_asymp` log/non-log, `dpnorm`, `dnorm_std`) and
  `crates/epiora-core/src/special/gamma_ebd0.rs` (new — round 4: R
  `ebd0`/`bd0_scale` log-space Poisson density);
  `.github/workflows/app.yml` (CI repair — the
  `app` job pinned `ubuntu-latest` → `ubuntu-24.04` because
  ubuntu-latest moved to 26.04 resolute on 2026-11-13 and Playwright's
  WebKit build needs `libicudata.so.74`, absent on resolute; the
  store-opfs suite and Playwright e2e fail at browser launch on every
  PR until reverted or Playwright supports resolute).

### v1.0 Task 5 follow-up — R qt/qbinom parity

- Owner decision 2026-11-14, option 1: exact R 4.6.2 nmath parity for
  the `special.*` quantiles before the deferred grid fixtures land
  (crates-only PR; `fixtures/`, `app/`, `spikes/` untouched). The
  fixtures-only WIP grid PR (`v1.0/task-2-special-grids`) had exposed
  `closed_form` mismatches in `t_quantile`, `binomial_quantile` and
  the `p = 0` gamma/chi-square boundaries.
- `t_quantile` is now `nmath/qt.c` L44-215 verbatim for
  `lower_tail = TRUE, log_p = FALSE` (new `special/qt.rs`): the
  `df < 1` `pt`-inversion bisection (L59-88, `p > 1 − DBL_EPSILON →
  +Inf`), the df ≈ 2 and df ≈ 1/Cauchy branches (L112-139), the
  `P_ok`/`P_ok1` small-probability log-space initial estimate
  (L146-157), Hill's AS 396 asymptotic and "y"-form estimates
  (L159-183) and the two-term Taylor/Newton refinement driven by
  `dt`/`pt` (L186-213). `dt` is `nmath/dt.c` L38-82 non-log form;
  `tanpi` is `cospi.c` L73-88 `Rtanpi`; the `!P_ok` `qnorm` log arm
  reuses `normal.rs`'s `qnorm_tail_val` (unreachable for non-log `p`
  but kept for parity). Signed-infinity results map to
  `Err(NotBracketed)`: `p ∈ {0, 1}`, `p > 1 − DBL_EPSILON` for
  `df < 1`, the `P ≤ DBL_MIN` arms, and the underflowed `df < 1`
  bisection (`qt(1e-300, 0.5) = −Inf` in R). The old
  `beta_quantile`/log-space Brent inversion is deleted; `t_cdf`/`t_sf`
  unchanged.
- `binomial_quantile` is `nmath/qbinom.c` L53-87 +
  `qDiscrete_search.h` L61-126, L165-224 verbatim
  (`lower_tail = TRUE, log_p = FALSE`, new `special/qbinom.rs`):
  Cornish–Fisher start `floor`-to-nearest via `libm::rint`
  (`R_forceint` = `nearbyint`), the `p * (1 − 8·DBL_EPSILON)`
  left-continuity fuzz, the unit-step search for `y < 4096` and the
  `_iShrink_ = 8` shrinking-increment loop above it. `pbinom` is
  `pbinom.c` L44-47 with the `floor(x + 1e-7)` integer rounding,
  evaluated as `reg_inc_beta_upper` — which required `bratio`'s real
  complement rule: `pbeta`/`pbeta_raw` now pass `x1 = 0.5 − x + 0.5`
  to `bratio` (pbeta.c L62) and the un-computed tail is formed as
  `0.5 − w + 0.5` before `do_swap`'s un-swap (toms708.c `L_end`),
  never `1 − w`. That last-ulp form is what the 1e5/1e9-n boundary
  quantiles at `p = 1 − 1e-16` were missing (engine was 1-28 below R).
  `beta.rs` also gained `pbeta_raw`'s point-mass limit cases
  (a = 0 / b = 0 / non-finite) in R's order.
- `gamma_quantile`/`chisq_quantile` keep R's check order (qgamma.c
  L141-148, qchisq.c L28-31 delegates `qgamma(p, df/2, 2)`): the IEEE
  `ISNAN` checks run first (a NaN argument is NaN even at `p = 0`),
  then `R_Q_P01_boundaries` — `p = 0 → Ok(0.0)` even for invalid
  shape, `p = 1 → Err(NotBracketed)` (R's +Inf) — then the argument
  checks; `shape = 0 → Ok(0.0)` for interior p. `qt`'s `P_ok1` is
  qt.c L147 verbatim (`P > DBL_MIN || !log_p` = always true in the
  `log_p = FALSE` port).
- wasm `epiora_wasm` size: 151,451 bytes gzip -9 (budget 409,600),
  measured by CI run
  https://github.com/dorrabbkhan/Epiora/actions/runs/43620791490
  (validation-report job) at ac56a18. The exact size shifts by a few
  bytes per commit because the build embeds the commit hash.
- Local-gate environment accommodations (this sandbox; no repo file
  changed): `CARGO_HOME=/tmp/cargo-home` (the real cargo registry is
  read-only), `PLAYWRIGHT_BROWSERS_PATH=target/ms-playwright` (browsers
  are vendored there, not in `~/.cache`), `CI=1` (Playwright
  `retries: 1`), and a `/tmp/check-bin/npx` shim that injects
  `--workers=4` into `npx playwright test` — Firefox's OPFS/SQLite
  layer returns `SQLITE_IOERR`/`Could not create actor` under the
  default 11 parallel profiles on this tmpfs (3 runs: 14/13/6 varying
  failures, all storage-layer; 57/57 firefox tests pass at
  `--workers=4`, and the gate's 171 e2e tests then pass with
  5 flaky-retried, 0 failed).
- Files outside Task 5's Files block: `docs/decisions.md` (this
  entry); `crates/epiora-core/src/special/qt.rs` (new — the qt.c port
  plus `dt`/`tanpi`/`qnorm-log` helpers; `t_dist.rs` could not hold it
  under the ~400-line rule), `crates/epiora-core/src/special/qbinom.rs`
  (new — the qbinom.c + qDiscrete_search.h port plus `pbinom`;
  `discrete.rs` keeps `poisson_tail`),
  `crates/epiora-core/src/special/beta.rs`,
  `beta_large.rs`, `beta_small.rs` (the `x1`/complement pass-through
  and point-mass limits — already recorded as outside-block files in
  the Task 5 entry), and `crates/epiora-core/src/special/normal.rs`
  (the qnorm tail extracted as `qnorm_tail_val` for `qt`'s log arm).
- Verification: `cargo test -p epiora-core --all-features --test
  procedures every_fixture_passes_via_run_case` on a detached
  worktree = my crates + the WIP fixtures plus a full-coverage
  regeneration of the three grids the WIP generator truncated
  (`t_quantile` all 187 cells, `gamma_quantile` 288, `chisq_quantile`
  120 — `/tmp/gen_full_grids.R` in the worktree, not committed): 0
  failures.

### v1.0 Task 5 follow-up round 2 — bratio L131 redo, tiny a,b, and df = +Inf

- Owner decision (in-session, fix round 2 on PR #70): close the
  pre-existing R-parity gaps the reviewer found, in the same PR.
- `bratio`'s `max(a,b) < eps·0.001` = 1e-18 block (toms708.c
  L168-186) is now ported in `inc_beta`: non-log `w = b/(a+b)`,
  `w1 = a/(a+b)` — the `log_p` ratio forms are unreachable (the
  engine only exposes `log_p = FALSE`), documented at the site.
- `bratio`'s L131 underflow redo (toms708.c L248-275) is now
  complete: a `w1` of `0` or subnormal after `bup`/`bgrat` is
  recomputed on the log scale — `bup(·, give_log = TRUE)` when
  `did_bup`, else `ML_NEGINF` — folded by `bgrat`'s `log_w`
  `logspace_add` accumulation and closed by `L_end_from_w1_log`'s
  `w = -expm1(log_w1)`, `w1 = exp(log_w1)`. `n` is `20` on every
  L131 call. `mu` in `bup` is provably `0` on both reachable
  entries (`b > 1 + a/10` is needed for `mu > 0`; `b = a0 <= 1`),
  so `d = 1` and `esum(0,·)` reduce correctly.
- Two further parity gaps surfaced by the new reference points and
  closed in the same round (outside the original fix list, recorded
  here): toms708.c's own `psi` (L1842-2036, the Cody–Strecok–Thacher
  rational fits) replaces the recurrence+asymptotic `digamma` `apser`
  used — the two differ by ~3.1e-13 at `psi(0.9)`, which is visible
  in `f_sf`'s subnormal `w1` results; and `dpois_raw`'s non-log form
  is now R 4.6.2's `exp(-yl)·exp(-yh)/sqrt(2πx)` over the `ebd0`
  double-double pair (dpois.c) instead of the pre-4.0
  `exp(-bd0-stirlerr)/sqrt(2πx)` — a few-ulp difference reachable
  through `pf(x, df1, +Inf) → pchisq` (e.g. `pchisq(12, 4, lo=F)`
  was `0.017351265236664505` vs R's `0.017351265236664512`).
- `pt(x, df = +Inf)` returns `pnorm(x)` (pt.c L40, checked after
  the NaN/`n <= 0`/`!R_FINITE(x)` order); `t_sf` mirrors it and
  `qt`'s `ndf > 1e20 → qnorm` arm already covers `ndf = +Inf`
  (qt.c L100). `pf`'s infinite-df paths (pf.c L44-56): `df2 =
  +Inf` → `pchisq(x·df1, df1, lo=F)`, `df1 = +Inf` → `pchisq(df2/x,
  df2, lo=T)`, both infinite → the `x < 1` / `x = 1` / `x > 1`
  piecewise — all before the `pbeta` call, R's check order kept.
- Tests (`crates/epiora-core/tests/special.rs`, additions only):
  `beta_tiny_ab_ratio_block`, `beta_l131_log_scale_redo`,
  `t_cdf_sf_infinite_df_is_pnorm`,
  `t_quantile_infinite_df_is_qnorm`,
  `f_sf_infinite_df_pchisq_paths` — each row carries the R 4.6.2
  `Rscript` command and output in the doc comment. All five failed
  before the ports (wrong value, `NaN`, or subnormal last-bits) and
  pass bit-for-bit after.
- wasm `epiora_wasm` size: 151,451 bytes gzip -9 (budget 409,600),
  measured by CI run
  https://github.com/dorrabbkhan/Epiora/actions/runs/43620791490
  (validation-report job) at ac56a18. The exact size shifts by a few
  bytes per commit because the build embeds the commit hash.
- Native/wasm parity: `corpus-runner /tmp/epiora-grid/fixtures`
  (release, `--features corpus`) vs `node
  crates/epiora-wasm/parity/run.mjs /tmp/epiora-grid/fixtures` —
  37 parity lines, `diff` byte-identical.
- Grid verification (detached worktree `/tmp/epiora-grid` at this
  branch + the `v1.0/task-2-special-grids` WIP fixtures + the
  `/tmp/gen_full_grids.R` full-coverage regeneration — `t_sf` 187,
  `t_quantile` 187, `gamma_quantile` 288, `chisq_quantile` 120
  cells): `cargo test -p epiora-core --all-features --test
  procedures every_fixture_passes_via_run_case` — 0 failures across
  3,066 `special` cells and the whole corpus.
- `./scripts/check.sh` (the full local gate): ALL CHECKS PASSED.
- Files outside the fix-round Files block: `docs/decisions.md`
  (this entry);
  `crates/epiora-core/src/special/beta_math.rs` (new `psi` port
  plus the `brcmp1` give_log corrections — `log1p`-split `c`, the
  `lx0 = -log1p(b/a)` centred form, and `log` of the truncated
  `const__` literal which is not ln(1/√2π));
  `crates/epiora-core/src/special/gamma.rs` and
  `gamma_ebd0.rs` (the `dpois_raw` non-log `ebd0` switch; `ebd0`,
  `M_SQRT_2PI`, `X_LRG` exported `pub(crate)`).
- Same sandbox accommodations as the previous entry
  (`CARGO_HOME=/tmp/cargo-home`, `PLAYWRIGHT_BROWSERS_PATH`,
  `CI=1`, the `/tmp/check-bin/npx` `--workers=4` shim); no repo
  file changed for them.

### v1.0 Task 2 follow-up — special.* grids

- Owner decision 2026-11-13, option (a): the seven `special.*` grids
  deferred out of Task 2 land here as a fixtures-only PR (no `crates/`,
  `app/` or `spikes/`). Owner decision 2026-11-14, option 1: exact
  R 4.6.2 nmath parity first — PR #70 merged at 076feeb — so no
  representable grid point may be left out on an engine mismatch.
- Generated by `Rscript fixtures/generate.R` on
  `R version 4.6.2 (2026-10-29) -- "Diversified Portfolio"`; the grids
  live in `fixtures/R/special.R` (v1.0 `inputs.grid` shape, all
  `closed_form`) and the generated JSON is under
  `fixtures/special/<fn>.json`. Leaf counts from the generated files:
  `t_cdf` 442, `t_quantile` 486, `f_sf` 732, `gamma_quantile` 1260,
  `chisq_quantile` 336, `poisson_tail` 144,
  `binomial_quantile` 1442 — 4,842 `special` cells total, of which
  674 are `value: null` leaves (123 / 486 / 65 in the three
  `Result`-typed quantiles) pairing R's Inf/NaN with the runner's
  `{"undefined": code}`.
- R calls (argument order is `run_special.rs`'s): `t_cdf` =
  `pt(t, df)`; `t_quantile` = `qt(p, df)`; `f_sf` =
  `pf(x, df1, df2, lower.tail = FALSE)`; `gamma_quantile` =
  `qgamma(p, shape, scale = scale)`; `chisq_quantile` =
  `qchisq(p, df)`; `poisson_tail` lower = `ppois(k, lambda)`, upper =
  `ppois(k - 1, lambda, lower.tail = FALSE)`; `binomial_quantile` =
  `qbinom(p, n, prob)`. Coverage: p in {0, 1} and outside [0, 1]
  (p = -0.5, 1.5) on every `Result`-typed quantile, deep tails
  (p to 1e-300, sf to underflow), df/shape at 0 and -1 and down to
  1e-300, df up to 1e300, x and t to ±1e300/±1e10, lambda 0 to 1e9,
  k 0 to `u64::MAX` (1.8446744073709552e19) plus the fractional
  k = 2.5 (R floors `ppois`, the runner's `as u64` truncates the same
  way), n 0 to 1e9, prob {0, 1}, and every reference point in
  `crates/epiora-core/tests/special.rs` including the seeded
  `QBINOM_R_REF` (seed 42) and `QBINOM_PARITY_REF` (seed 20261114)
  tables, regenerated by the same commands in the same order.
- Points left out — each is unrepresentable by the format or the
  runner, never an engine mismatch:
  - Non-finite inputs anywhere (t_cdf/t_quantile/f_sf df = ±Inf,
    f_sf x = ±Inf, t_cdf t = ±Inf, poisson_tail lambda = +Inf, NaN
    arguments): jsonlite writes non-finite input as null, and a null
    argument is not the same call.
  - `t_cdf` and `f_sf` at df <= 0, `poisson_tail` at lambda < 0:
    R returns NaN (`leaf()` → null) but the scalar-CDF runner paths
    return f64 NaN, which serialises as plain JSON `null` — the
    comparator pairs a null leaf only with `{"undefined": code}`.
  - `binomial_quantile` at p or prob outside [0, 1] and at n < 0:
    R returns NaN but the engine's `u64` signature clamps to the
    integer edges (u64 has no NaN channel), so no leaf can pair.
  - `poisson_tail` at k < 0: the runner's `a[0] as u64` maps -1 to 0,
    which would mis-pair R's `ppois(-1, .) = 0` with the engine's
    `ppois(0, .)`.
- Regeneration is deterministic: `Rscript fixtures/generate.R` was run
  twice consecutively (pinned LibreOffice 26.2.6.3 via
  `EPIORA_SOFFICE`, workspace-local `RENV_PATHS_ROOT` because the
  sandbox denies `~/.cache/R/renv`); all 1,274 files under `fixtures/`
  were md5-identical between runs.
- Files outside the task's Files block: `docs/decisions.md` (this
  entry). The `ods_dates` date leak was fixed separately in PR #72
  and this PR no longer touches `input.ods`.
- Platform-ulp note (found by the fixtures workflow, run
  43640700793): `qt(0.25, 1)` and `qt(0.75, 1)` — the Cauchy
  `tanpi` branch — produce `∓1.0000000000000002` under this
  machine's R 4.6.2 build and exactly `∓1` under the workflow's
  ubuntu build. The two leaves in `t_quantile.json` carry the
  workflow's bytes (blob `3282673`, quoted from the CI drift log);
  a regeneration on this machine drifts back by one ulp on those
  two leaves only. Both values pass `closed_form` against the
  engine's result either way. Owner accepted the CI-generator bytes
  for these two leaves (2026-11-14, PR #71 option B1).

### fixtures: ods_dates styles.xml date pinned (2026-11-14)

- Owner decision (PR #71, option A1, 2026-11-14): fix the drift at
  the generator — pin the wall-clock date LibreOffice writes into
  `styles.xml`, regenerate `fixtures/import/ods_dates/input.ods`.
- Cause: `lo_convert` runs LibreOffice under `SOURCE_DATE_EPOCH=0`
  and `TZ=UTC`, and `repack_zip(..., normalize_ods_meta)` pinned
  meta.xml only. LO 26.2.6.3 still writes the conversion day's date
  into styles.xml's default page header as
  `<text:date text:date-value="YYYY-MM-DD">` — the one timestamp
  `SOURCE_DATE_EPOCH` does not cover. The committed file carried
  2026-11-13, so a regeneration on any other day differed by that
  string and the `fixtures` workflow drift check failed daily.
- Fields pinned: `normalize_ods` (new `repack_zip` patch, wired in
  `fixtures/R/import.R`) runs `normalize_ods_meta` plus
  `normalize_ods_styles`, which rewrites every
  `text:date-value="..."` in styles.xml to `2026-01-01` (the date
  part of `LO_META_DATETIME`); `stop_if` fires when the field is
  missing so an LO layout change fails loudly at generation.
- Other members inspected on a fresh pinned-LO conversion:
  meta.xml's producer fields (already pinned), settings.xml,
  manifest.rdf and META-INF/manifest.xml carry no date/time value;
  `<text:time>` in the same header has no `text:time-value`
  attribute and its element text is the fixed placeholder
  `00:00:00`. content.xml's `office:date-value` cells are the data
  under test and stay exactly as LO wrote them.
- Proof (this sandbox has no faketime/libfaketime, so the fallback
  applies): a raw `lo_convert` on 2026-11-14 produced
  `text:date-value="2026-11-14"`; after `normalize_ods` the
  regenerated `input.ods` carries `2026-01-01`; `grep -r
  2026-11-14` over every extracted member and over the zip's raw
  bytes → 0 matches; two consecutive `Rscript fixtures/generate.R`
  runs produced sha256-identical `input.ods`
  (45d2ae4a55eb2cb5a20294a6c2ae8a59034abc0bf0ab74e19ddcf651e093f762);
  `git status --porcelain fixtures/` shows only `input.ods` and the
  two R files.
Supersedes: —

### v1.0 Task 4 — Epi Info 7 MEANS reference

- `oswego.pgm` gains `MEANS AGE` and `MEANS AGE SEX` before `CLOSEOUT`
  (v1.0 spec §4.4). Provenance, verified byte-for-byte by SHA-256
  against the downloaded `ei7-output` artifacts: the committed
  `oswego.html` is byte-identical to the `ei7-output` artifact of both
  runs https://github.com/dorrabbkhan/Epiora/actions/runs/43498896712
  and https://github.com/dorrabbkhan/Epiora/actions/runs/43500418310
  (sha256 `1a599aa29da4898177816ad7d02636d0df8ee73c3103695828019df45cc12820`
  in all three); the committed `oswego.json` is byte-identical to the
  artifact of run 43500418310 — dispatched at commit `d01030b`, the
  final parser + labels (sha256
  `6dbfcfc15e32d8c9c6f1602fe966b26100ec4a6c3daa8b16fffc3738cc8d1fea`).
  Run 43498896712's `oswego.json` artifact (sha256
  `923ec1e1aa7ed6768791296178bc32c1813a7e4de4338e3625c8012e15aeba05`)
  differs: it predates the final labels. `ei7.schema.json` unchanged
  (the oswego.json shape did not change). Superseded by the
  owner-decision regeneration recorded in the resolution bullet below.
  No file outside the task's Files block.
- **Percentile definition: R `quantile(type = 2)`.** Verified by
  computation, not assertion:
  `quantile(x, c(.25,.5,.75), type = k)` for `k = 1..9` and `fivenum(x)`
  were computed on Oswego AGE overall and per SEX
  (`fixtures/data/oswego.csv`, same values as the bundled Sample.mdb —
  totals match at 2761). EI7 prints 16/36/58 overall, 16.5/35.5/53.5
  Female, 15/36/62 Male. Only `type = 2` reproduces all nine values at
  display precision (type 2: `h = np + 1/2`; when `h` is an integer the
  two bracketing order statistics are averaged, otherwise the single
  nearest one). `fivenum()` gives 16.5/36/57.5, 16.5/35.5/53.5,
  16/36/61 — it fails on the overall and Male quartiles.
- **Mode rule: the smallest value among tied modes.** Overall AGE modes
  tie {11, 17, 35, 62} at frequency 4 and EI7 prints 11; Female modes
  tie {35, 62} at frequency 3 and EI7 prints 35; Male has a unique mode
  17. Same rule as Task 2 recorded from `cDataSetHelper.cs`, now derived
  from printed output.
- **EI7–R fixture disagreement — six leaves beyond `ei7_display`, not
  four (owner escalation, recorded per standing notes; no tolerance
  change):** of the 61 mapped `means.*` leaves in `oswego.json`, 55
  agree with the Task 2 R fixtures and six disagree. The count and the
  per-leaf diffs come from comparing every `oswego.json` `means.*`
  value against the matching R-fixture leaf at `ei7_display`
  (`0.5 × 10^−d`, `d` = EI7's displayed decimals) — script and output:

  ```text
  $ node ei7_vs_r.mjs   # run from the repo root; script reproduced below
  DISAGREE means.overall.p25: EI7 16.0000 vs R 16.5 (|d|=5.000e-1, tol=0.00005)
  DISAGREE means.overall.p75: EI7 58.0000 vs R 57.5 (|d|=5.000e-1, tol=0.00005)
  DISAGREE means.groups[1].p25: EI7 15.0000 vs R 16 (|d|=1.000e+0, tol=0.00005)
  DISAGREE means.groups[1].p75: EI7 62.0000 vs R 61 (|d|=1.000e+0, tol=0.00005)
  DISAGREE means.t_test.welch.diff_ci_95.lower: EI7 -10.5528 vs R -10.556442937780664 (|d|=3.643e-3, tol=0.00005)
  DISAGREE means.t_test.welch.diff_ci_95.upper: EI7 10.4663 vs R 10.469932673851053 (|d|=3.633e-3, tol=0.00005)

  55 agree / 6 disagree of 61 mapped leaves (1 null-mapped)
  ```

  | `means.*` leaf (`oswego.json`) | EI7 | R fixture |
  |---|---|---|
  | `means.overall.p25` | 16.0000 | `means_oswego_age.json` `groups[0].p25` = 16.5 |
  | `means.overall.p75` | 58.0000 | `means_oswego_age.json` `groups[0].p75` = 57.5 |
  | `means.groups[1].p25` (Male) | 15.0000 | `means_oswego_age_sex.json` `groups[1].p25` = 16 |
  | `means.groups[1].p75` (Male) | 62.0000 | `means_oswego_age_sex.json` `groups[1].p75` = 61 |
  | `means.t_test.welch.diff_ci_95.lower` | −10.5528 | `means_oswego_age_sex.json` `t_test.welch.lower` = −10.556442937780664 |
  | `means.t_test.welch.diff_ci_95.upper` | 10.4663 | `means_oswego_age_sex.json` `t_test.welch.upper` = 10.469932673851053 |

  The four quartile leaves are the known percentile-definition
  difference: the Task 2 fixtures were generated with `fivenum()` while
  EI7 computes R `quantile(type = 2)`; Female and all medians agree.
  The two Welch CI leaves are a separate disagreement whose cause is
  not established. Facts computed in R 4.6.2 — the Welch CI of
  `mean(Female) − mean(Male)` with
  `se = sqrt(var(Female)/44 + var(Male)/31)` evaluated at the
  Satterthwaite df 56.047 and at df = 56, 57 and 73 (the exact command
  and its output):

  ```text
  $ Rscript -e '
    d <- read.csv("fixtures/data/oswego.csv")
    x <- d$AGE[d$SEX == "Female"]; y <- d$AGE[d$SEX == "Male"]
    diff <- mean(x) - mean(y)
    se <- sqrt(var(x)/length(x) + var(y)/length(y))
    df_s <- (var(x)/length(x) + var(y)/length(y))^2 /
            ((var(x)/length(x))^2/(length(x)-1) + (var(y)/length(y))^2/(length(y)-1))
    cat("diff =", format(diff, digits=17), " se =", format(se, digits=17),
        " satterthwaite_df =", format(df_s, digits=17), "\n")
    for (df in c(df_s, 56, 57, 73)) {
      tc <- qt(0.975, df)
      lo <- diff - tc*se; hi <- diff + tc*se
      cat(sprintf("df = %-8s t crit = %.10f  CI = (%.10f, %.10f)  at 4dp: (%.4f, %.4f)\n",
                  format(df, digits=10), tc, lo, hi, lo, hi))
    }
    cat("EI7 prints: (-10.5528, 10.4663)\n")'

  diff = -0.043255131964805571  se = 5.248187959901089  satterthwaite_df = 56.047354306487847
  df = 56.04735431 t crit = 2.0032033696  CI = (-10.5564429378, 10.4699326739)  at 4dp: (-10.5564, 10.4699)
  df = 56       t crit = 2.0032407188  CI = (-10.5566389534, 10.4701286895)  at 4dp: (-10.5566, 10.4701)
  df = 57       t crit = 2.0024654593  CI = (-10.5525702455, 10.4660599816)  at 4dp: (-10.5526, 10.4661)
  df = 73       t crit = 1.9929971259  CI = (-10.5028786522, 10.4163683882)  at 4dp: (-10.5029, 10.4164)
  EI7 prints: (-10.5528, 10.4663)
  ```

  None of df 56.047 (Satterthwaite), 56, 57 or 73 reproduces EI7's
  (−10.5528, 10.4663) at `ei7_display` precision; df = 57 is closest at
  (−10.5526, 10.4661) but still differs by ~2 × 10^−4, beyond the
  5 × 10^−5 tolerance. What differs in EI7's Welch interval is
  undetermined — no method is chosen here and no fixture changes. The
  owner decides whether the R fixtures move to `quantile(type = 2)`
  (they are the `closed_form` oracle Task 6 tests against) and which
  Welch-CI convention the oracle records.

  The comparison script (saved as `ei7_vs_r.mjs`, run from the repo
  root):

  ```js
  import { readFileSync } from 'node:fs';
  const oswego = JSON.parse(readFileSync('fixtures/ei7/oswego.json', 'utf8'));
  const fx = {
    'MEANS AGE': JSON.parse(readFileSync('fixtures/means/means_oswego_age.json', 'utf8')),
    'MEANS AGE SEX': JSON.parse(readFileSync('fixtures/means/means_oswego_age_sex.json', 'utf8')),
  };
  const statRename = { min: 'minimum', max: 'maximum' };
  const leaf = (exp, path) => path.reduce((o, k) => o?.[k], exp)?.value;
  function rValue(command, key, exp) {
    let p = key.replace(/^means\./, '').split('.');
    if (p[0] === 'overall') {
      const stat = statRename[p[1]] ?? p[1];
      return leaf(exp, ['groups', 0, stat]);
    }
    if (p[0].startsWith('groups[')) {
      const i = +p[0].slice(7, -1), stat = statRename[p[1]] ?? p[1];
      return leaf(exp, ['groups', i, stat]);
    }
    if (p[0] === 't_test')
      return leaf(exp, ['t_test', p[1], p[2] === 'diff_ci_95' ? p[3] : p[2] === 'p_value' ? 'p' : p[2]]);
    if (p[0] === 'anova')
      return leaf(exp, ['anova', p[1] === 'p_value' ? 'p' : p[1]]);
    if (p[0] === 'bartlett')
      return leaf(exp, ['bartlett', p[1] === 'statistic' ? 'chisq' : p[1] === 'p_value' ? 'p' : p[1]]);
    if (p[0] === 'kruskal_wallis')
      return leaf(exp, ['kruskal_wallis', p[1] === 'p_value' ? 'p' : p[1]]);
    return undefined;
  }
  let agree = 0, disagree = 0, unmapped = 0;
  for (const out of oswego.outputs) {
    if (!out.command.startsWith('MEANS')) continue;
    const exp = fx[out.command].expected;
    for (const v of out.values) {
      if (!v.key) { unmapped++; continue; }
      const r = rValue(out.command, v.key, exp);
      const tol = 0.5 * 10 ** -v.decimals;
      const d = Math.abs(v.value - r);
      if (d <= tol) agree++;
      else { disagree++; console.log(`DISAGREE ${v.key}: EI7 ${v.text} vs R ${r} (|d|=${d.toExponential(3)}, tol=${tol})`); }
    }
  }
  console.log(`${agree} agree / ${disagree} disagree of ${agree + disagree} mapped leaves (${unmapped} null-mapped)`);
  ```
- Other conventions observed: `Satterthwaite` is the Welch t-test
  (`means.t_test.welch.*`), `Pooled` the Student t-test
  (`means.t_test.pooled.*`); `Diff (Group 1 - Group 2)` is
  first-minus-second category order (Female − Male); the ANOVA p-value
  is a `<P>P-value = …</P>` paragraph, not a table cell;
  `T-Test: Diff (Group 1 - Group 2) Std Dev` is the pooled SD of the
  difference and maps to `null` (no Task 6 `TTest` field); the
  `Variances`/`Equal`/`Unequal` cells carry no number and are not
  emitted; a repeated label inside one block gets a ` (n)` suffix
  (Pooled vs Satterthwaite rows, and the two cells under the spanning
  `95% CL Mean` header).
- **Follow-up for Task 6 (recorded only; nothing changed here):** the
  Task 2 R fixtures `fixtures/means/means_oswego_age*.json` use field
  names that differ from Task 6's Produces block and from this PR's
  `labels.json` keys: `minimum`/`maximum` (vs `min`/`max`), `p` (vs
  `p_value`), `t_test.*.lower`/`upper` (vs `diff_ci_95.lower`/`upper`),
  `bartlett.chisq` (vs `statistic`), the ungrouped overall stored as
  `groups[0]` (vs `overall`), and no `overall` block in the grouped
  fixture. Task 6 must reconcile these; the owner decides which side
  moves.
- **Resolution (owner decision on PR #67, comments 6865307135 and
  6865623756): option 1 — the engine keeps the exact Welch CI; the two
  EI7 Welch CL labels are mapped to `null` here.** The cause is in the
  Epi Info source (Epi-Info-Community-Edition @ 4cd353c — D2 covers
  the source at that commit only, not CDC release binaries):
  `Epi.Analysis.Statistics/Means.cs:1469-1478` computes the Welch CLs
  (1464-1468 the p-value) through `StatLib.TfromP`
  (`StatisticsRepository/StatLib.vb:518-565`; `PfromT` at 80-143),
  which linearly interpolates the t critical value between floor and
  ceil of the Satterthwaite df instead of evaluating the t quantile at
  the fractional df: `t* = T(57) + 0.04735·(T(56) − T(57)) = 2.002502`
  vs `qt(0.975, 56.0474) = 2.003203`. The interpolation reproduces
  EI7's printed (−10.5528, 10.4663) exactly:

  ```text
  $ Rscript -e 'df<-56.04735430648786; f<-df-56; t<-qt(.975,57)+f*(qt(.975,56)-qt(.975,57)); d<-read.csv("fixtures/data/oswego.csv"); x<-d$AGE[d$SEX=="Female"]; y<-d$AGE[d$SEX=="Male"]; diff<-mean(x)-mean(y); se<-sqrt(var(x)/length(x)+var(y)/length(y)); cat(sprintf("interp t*=%.10f vs qt(.975,df)=%.10f\n",t,qt(.975,df))); cat(sprintf("interp CI=(%.10f, %.10f) -> (%.4f, %.4f); exact CI=(%.4f, %.4f); EI7 prints (-10.5528, 10.4663)\n",diff-t*se,diff+t*se,diff-t*se,diff+t*se,diff-qt(.975,df)*se,diff+qt(.975,df)*se))'
  interp t*=2.0025021712 vs qt(.975,df)=2.0032033696
  interp CI=(-10.5527629164, 10.4662526524) -> (-10.5528, 10.4663); exact CI=(-10.5564, 10.4699); EI7 prints (-10.5528, 10.4663)
  ```

  The EI7 Welch p-value and df still agree with R at display
  precision and stay mapped (`means.t_test.welch.p_value`,
  `means.t_test.welch.df`). `labels.json` maps the two CL labels to
  `null`; `parse.test.mjs` asserts them parsed but unmapped.
  `oswego.json` was regenerated only via the ei7 workflow: producing
  run https://github.com/dorrabbkhan/Epiora/actions/runs/43513433527
  (its drift step failed by construction — the committed json
  predated the labels change; the `ei7-output` artifact was committed
  byte-for-byte, `oswego.json` sha256
  `7928845c3504dc9a172813e4f548ff2f7354f323baf008d08fdfc513b19b2e4c`,
  `oswego.html` unchanged at sha256 `1a599aa2…`), and green
  verification run
  https://github.com/dorrabbkhan/Epiora/actions/runs/43513632984
  whose `ei7-output` artifact is byte-identical to the committed
  files (cmp + sha256). The only json change vs the previous commit
  is the two leaves losing their key. The engine CI red on
  `oswego_matches_epi_info_7` (`means.*` "actual missing") is known
  and stays per owner decision 1.1.
- **After PR #68 (owner decision 2.1), the EI7–R comparison is 59
  agree / 0 disagree of 59 mapped leaves.** #68 resolves the four
  quartile disagreements and the field-name mismatches recorded above
  by regenerating `fixtures/means/means_oswego_age*.json` with R
  `quantile(type = 2)` and the `labels.json` key names (`min`/`max`,
  `p_value`, `diff_ci_95.lower`/`upper`, `bartlett.statistic`, a real
  `overall` block); the two Welch CL labels are now unmapped.
  Recomputed with the same comparison script (shortened — post-#68
  fixtures share the labels.json key paths, so `rValue` walks
  `key.split('.')` directly) against
  `origin/v1.0/task-2-means-quartiles-and-names:fixtures/means/means_oswego_age*.json`:

  ```text
  $ node ei7_vs_r.mjs /tmp/ei7fix/oswego_new.json /tmp/ei7fix
  59 agree / 0 disagree of 59 mapped leaves (3 null-mapped)
  ```

  The shortened script (fixture files extracted to `/tmp/ei7fix/` via
  `git show origin/v1.0/task-2-means-quartiles-and-names:<path>`;
  `oswego_new.json` is the committed `oswego.json` — verified
  byte-identical to the green run's artifact):

  ```js
  import { readFileSync } from 'node:fs';
  const oswego = JSON.parse(readFileSync(process.argv[2] ?? 'fixtures/ei7/oswego.json', 'utf8'));
  const fxDir = process.argv[3] ?? 'fixtures/means';
  const fx = {
    'MEANS AGE': JSON.parse(readFileSync(`${fxDir}/means_oswego_age.json`, 'utf8')),
    'MEANS AGE SEX': JSON.parse(readFileSync(`${fxDir}/means_oswego_age_sex.json`, 'utf8')),
  };
  function rValue(key, exp) {
    const parts = key.replace(/^means\./, '').split('.');
    let o = exp;
    for (const p of parts) {
      const m = /^groups\[(\d+)\]$/.exec(p);
      o = m ? o?.groups?.[+m[1]] : o?.[p];
    }
    return o?.value;
  }
  let agree = 0, disagree = 0, unmapped = 0;
  for (const out of oswego.outputs) {
    if (!out.command.startsWith('MEANS')) continue;
    const exp = fx[out.command].expected;
    for (const v of out.values) {
      if (!v.key) { unmapped++; continue; }
      const r = rValue(v.key, exp);
      const tol = 0.5 * 10 ** -v.decimals;
      const d = Math.abs(v.value - r);
      if (d <= tol) agree++;
      else { disagree++; console.log(`DISAGREE ${v.key}: EI7 ${v.text} vs R ${r} (|d|=${d.toExponential(3)}, tol=${tol})`); }
    }
  }
  console.log(`${agree} agree / ${disagree} disagree of ${agree + disagree} mapped leaves (${unmapped} null-mapped)`);
  ```

### v1.0 Task 6 — MEANS statistics, procedure and registry

- **Files outside the Files block:** `crates/epiora-core/src/corpus/mod.rs`
  (one line: `pub mod run_means;` plus its `pub use` neighbours — the
  module must be declared for the new `corpus/run_means.rs`);
  `app/src/ui/strings.ts` (display strings for the new
  `too_few`/`zero_variance` undefined reasons and the new
  `not_numeric_variable`/`too_many_groups` error codes — the app's
  exhaustive `undefinedReason` map fails `tsc --noEmit` without them)
  and `app/src/worker/errors.ts` (`ENGINE_ERROR_CODES` must mirror
  `EngineError::code()` — `contract.test.ts` fails without the two new
  codes);
  `crates/epiora-wasm/bindings/Estimate.ts` and the new
  `bindings/{Anova,Bartlett,GroupStats,KruskalWallis,MeansOptions,MeansResult,TTest,TwoGroupTests}.ts`
  (regenerated by `cargo test -p epiora-core --features ts --lib
  export_bindings` — the engine gate's "TypeScript bindings diff" step
  fails until the regenerated files are committed; they are outputs of
  the ts-rs derive, not hand-written code); `docs/decisions.md` (this
  entry, as the conductor note requires).
- **Block-presence rule taken from the generator, not the plan's
  shorthand.** The plan's line "tests present iff `group_by` is set:
  `t_test` iff exactly two non-empty groups; `anova`, `bartlett`,
  `kruskal_wallis` iff ≥ 2 non-empty groups" reads literally as
  "bartlett always present for ≥ 2 groups" — but the fixture for
  `means_one_obs_group` has no `bartlett` block because R's
  `bartlett.test` errors on a one-observation group and the
  generator's `tryCatch` drops it. Implemented as: `anova` and
  `kruskal_wallis` present whenever ≥ 2 non-empty groups (degenerate
  statistics inside them are `Undefined`, not absent — `means_constant`
  proves it); `bartlett` absent when any group has fewer than two
  observations; `t_test` absent unless R's two `t.test` calls would
  both succeed (both groups ≥ 2 observations and neither standard
  error "essentially zero", R's `10 * .Machine$double.eps` rule —
  `means_constant` has no `t_test` block).
- **`TooManyGroups` counts emitted groups.** The check counts the
  groups actually emitted (non-empty categories, including a
  non-empty `"(missing)"` group), matching the run's output size;
  categories left without a kept value vanish exactly as R's
  `sort(unique(gs[keep]))` does.
- **Top-level `total`/`missing`.** The fixture's `total` is the column
  length (dataset row count) and `missing` is the union count of
  records dropped for a missing analysis-variable or group value —
  computed as `row_count − overall.observations`, identical to the
  generator's `sum(dropped)` (a record missing both is counted once
  in `missing` but under each variable in `exclusions`, as for
  TABLES per spec §4.1).
- **Local gate on a stacked branch.** The branch is based on
  `origin/v1.0/task-4-ei7-means-reference` (conductor note), whose
  fixture changes are legitimately part of the base; the local gate
  was run as `BASE=e6b1b1d ./scripts/check.sh [quick]` so the
  fixture guard and changed-file detection see Task 6's diff only.
  CI on the eventual PR compares against `main` — once PR #67
  merges, `git merge origin/main` (per the conductor note, never
  rebase) gives a fixtures-clean diff.
- **`out_of_range` undefined reason (fix round 1, fuzz crash).** The
  `means_any` fuzz target crashed in CI (run 43651492371,
  `result.anova.ss_between: null`): computed `f64`s were wrapped in
  `Estimate::Value(...)`, so ±∞/NaN from extreme-but-finite input
  (1e300-scale) serialised as `null`. Every computed leaf now goes
  through `Estimate::from_f64`; a non-finite result reports the new
  `UndefinedReason::OutOfRange` (`out_of_range` — the spelling
  `run_special.rs` already emits for out-of-domain quantile inputs).
  Reasons were chosen so `too_few`/`zero_variance` stay reserved for
  structural degeneracy (n below the statistic's minimum, an explicit
  zero denominator or the "essentially constant" standard-error rule)
  while `out_of_range` means "finite input, unrepresentable output";
  undefined reasons propagate to the statistics derived from them
  (`ms_between`, `ms_within`, `f`, `ss_total`). Remaining
  `Estimate::Value` leaves are exact integer counts
  (`observations`-like df leaves: `k − 1`, `N − k`, `N − 1`,
  Bartlett/Kruskal–Wallis `k − 1`) or the literal `0.0` total of an
  empty group — always finite. Mirrored as the round-0 reasons were:
  `code()`, the ts-rs `type` union, regenerated `bindings/Estimate.ts`,
  and `app/src/ui/strings.ts` (`'the value is out of range'`). Files
  outside the Files block touched by the fix:
  `crates/epiora-core/fuzz/regressions/means_any/crash-300469b405452edd2914bf057aa24260acdd5f75`
  (the committed CI crash input, per the fuzz-nightly convention) and
  `crates/epiora-core/tests/fuzz_regressions.rs` (`means_any` added to
  the replay list so the convention actually covers it).

### v1.0 Task 7 — Calculator registry, receipt v3, counts family

- **Non-counts calculators fail closed, not skipped.** The corpus's
  existing mechanism for unimplemented `calc.<id>` procedures is that
  `run_case` errors — fixtures for families without a `run` are simply
  not listed in the `FAMILIES` walk (same as `means` before Task 6).
  Task 7 follows it: `calc::run` returns `InvalidOptions` ("registered
  but its family is not implemented yet") for the 14 non-counts ids —
  a defined engine error, no new code, no skipped fixture. The ten
  counts directories are listed per-calculator in both
  `corpus-runner`'s `FAMILIES` and `parity/run.mjs`; the other 14 join
  when Tasks 8/9 implement them.
- **`every_output_has_registry_coverage` measures emittable output.**
  Validation-`REGISTRY` rows exist for every statistic the ten counts
  calculators can emit (pointed at the Task 2 fixtures that actually
  contain the leaf); the other 14 calculators' `OutputDef` metadata is
  complete in `calc::REGISTRY` but adds no validation rows — a row for
  output nothing can emit would be unmeasurable coverage. Tasks 8/9
  add their rows with their implementations.
- **Iterative roots use a port of R's `zeroin`, not `brent_root`.**
  Fixture leaves that come out of R `uniroot` (`exactci` mid-p,
  `epitools::or.midp`, `exact2x2` limits) are recorded at uniroot's
  default tolerance, not a tight root; `special/zeroin.rs` is a
  bit-faithful port of R's `src/library/stats/src/zeroin.c` run at
  `UNIROOT_TOL = .Machine$double.eps^0.25`. `brent_root` stays for the
  tighter v0.2 corpus semantics. New file outside the Files block.
- **`matched_n.cmle_or` is always `{"undefined": ...}` in Task 7.** The
  generator's `clogit` call fails ("could not find function `strata`"),
  so every `matched_n` fixture records the leaf as `null`; emitting
  `Undefined` reproduces the oracle exactly.
- **`smr.conf_level` is `Percent`, not `ConfLevel`.** The inventory
  names 90/95/99 "everywhere", but the authoritative
  `calc/smr/conf_97` fixture exercises `conf_level: 97` for the
  conditional Ury–Wiggins/Vandenbroucke blocks; the input therefore
  accepts any percentage in `[0, 100]`.
- **Canonical defaults serialize integral values as JSON integers** —
  `conf_level: 95`, never `95.0` — so the canonical-input hash is
  stable across engines that distinguish the two.
- **Large counts take the mathematically exact path (PR #74 review
  fix).** Valid counts `< 2^53` no longer drive O(n) work: counts
  calculators call a calc-only `pbinom` — R `pbinom`'s own
  `pbeta(p, x+1, n−x, !lower.tail)` identity on the Task 5
  `reg_inc_beta`/`reg_inc_beta_upper` — instead of
  `special::binomial_tail`'s literal sum (untouched; the non-calc
  corpus and the native/wasm parity output are bound to it);
  `pair()`'s `fisher_2t` binary-searches the `1+1e-7` tie band on the
  symmetric `Binom(n, 0.5)` and sums the two equal tails with
  `pbinom`; `matched_n` accumulates the ≤ `2·(M+1)` stratum shapes
  times their multiplicities instead of allocating one `Table2x2`
  per set (O(M) memory at ~1e12 sets). No input cap, work limit or
  new `Undefined` reason was added. `two_by_two`/`two_by_n` keep the
  noncentral-hypergeometric support sums and OR roots, which have no
  closed form — reported on the PR under "Still unbounded (owner
  decision)", not capped.
- **Point masses use R's own `dbinom_raw`/`dpois_raw` (PR #74 review
  fix round 2).** The calc-layer `dbinom`/`dpois` composed
  `ln_gamma` differences, which cancel ~17 digits once `n` swamps
  the result — `binomial.tail.eq` returned 3.1e21 at n = 2^53−1 and
  `pair()`'s `midp_p` −6.6e7 for valid counts. Both now call the
  already-ported Loader saddle-point routines (`dbinom_raw` in
  `special/hypergeom.rs`, `dpois_raw` in `special/gamma.rs`,
  re-exported `pub(crate)`), exactly what R's `dbinom`/`dpois`
  compute; edge semantics are unchanged. The same audit replaced
  `martin`'s `lchoose`-composed polynomial with `dhyper`'s log mass
  (the dropped `1/C(m+n,k)` factor is constant per stratum and
  cancels in its max-normalisation).
- **`pair()` `mcnemar_cc` applies the continuity 1 only when
  `db != dc` (PR #74 review fix round 2).** That is
  `mcnemar.test(correct=TRUE)`'s `any(x - t(x) != 0)` condition
  verbatim: equal discordant counts give statistic 0 (p = 1), not
  1/n. No fixture covers `db == dc`; `mcnemar_cc_balanced_discordance
  _is_zero` asserts R's `matrix(c(10,5,5,10),2)` result.
- **Files outside the Files block:**
  - `crates/epiora-core/src/special/zeroin.rs` + `special/mod.rs` —
    the `uniroot` port above. `special/{hypergeom,gamma}.rs` joined
    `mod.rs` in fix round 2: `dbinom_raw`/`dhyper`/`dpois_raw`
    visibility only, no behaviour change to the exported routines.
  - `crates/epiora-core/src/calc/registry*.rs`,
    `crates/epiora-core/src/calc/counts/defs*.rs`,
    `crates/epiora-core/src/calc/counts/{shared,shared_exact,exact_or,martin}.rs`,
    `crates/epiora-core/src/validation/registry/calc_*.rs` — the
    registry/coverage data and shared helpers, split per the ~400-line
    file rule (the Files block's `counts/*.rs` and `calc/*.rs` globs
    cover the pattern; named here for completeness).
  - `crates/epiora-core/src/corpus/compare.rs` — leaf detection now
    requires `{value, class}`: calc fixtures nest groups whose own
    key is `"value"`.
  - `crates/epiora-core/src/corpus/report/mod.rs` — strips the
    two-segment `calc.<id>.` prefix when resolving validation paths
    (other families keep one-segment stripping).
  - `crates/epiora-core/src/bin/corpus-runner.rs` and
    `crates/epiora-wasm/parity/run.mjs` — the ten counts `calc/<id>`
    families added to both parity walks (kept identical).
  - `crates/epiora-core/src/corpus/run_case.rs` — beyond the `calc.*`
    delegation line, the run emits the calculator's canonical-JSON v3
    receipt so the parity line covers it.
  - `crates/epiora-core/tests/{registry,receipt,fuzz_regressions}.rs`
    — `registry.rs` strips the `calc.<id>.` prefix for calc rows;
    `receipt.rs` unwraps the now-`Option` `dataset`;
    `fuzz_regressions.rs` registers `calc_any`.
  - `crates/epiora-core/fuzz/regressions/calc_any/.gitkeep` —
    convention for a new target's replay directory.
  - `crates/epiora-wasm/bindings/{CalculatorDef,CalculatorStamp,CellKind,
    DefaultValue,Family,InputDef,InputKind,OutputDef}.ts` and
    `bindings/Receipt.ts` — ts-rs generated output for the new public
    types and the v3 receipt fields (regenerated, not hand-edited).
  - `app/src/worker/errors.ts` and `app/src/ui/strings.ts` — the three
    new engine codes (`calc_unknown`, `calc_input_missing`,
    `calc_input_out_of_range`) mirrored into the app's code union and
    user strings.
  - `app/src/ui/screens/analysis/ReceiptPanel.tsx` — guards the
    dataset-hash row: `Receipt.dataset` is `null` on v3 calculator
    receipts.
  - `app/src/worker/integration.browser.test.ts` — `dataset` is now
    optional on `Receipt`; the runs it stores are procedure receipts,
    so the assertions assert presence.
  - The Files block's `stats/{matched,trend,screening}.rs` and
    `corpus/run_calc.rs` were not created: the matched/trend/screening
    maths is package-specific (`epiR`/`epitools`/`exactci` semantics,
    not generic stats APIs) and lives in `calc/counts/*`; the `calc.*`
    delegation is the single run_case branch the conductor note asked
    for.

### v1.0 Task 2 follow-up — clogit strata (1:n cmle_or) and smr conf_level kind (2026-11-15)

Owner-approved fixtures follow-up (owner decision:
<https://github.com/dorrabbkhan/Epiora/pull/74#issuecomment-6898203468>),
same class as the earlier #66/#68/#71 follow-ups; fixtures still change
only via `Rscript fixtures/generate.R`.

- **Cause.** Two bugs, found in sequence. (a)
  `matched_expected`'s 1:n branch in `fixtures/R/calc.R`
  called `survival::clogit(case ~ exposed + strata(stratum), data = …)`
  with `strata` not in scope (survival is used qualified and never
  attached), so the call raised `could not find function "strata"`. A
  `tryCatch(…, error = function(e) NULL)` swallowed it and wrote all six
  `calc.matched_case_control.matched_n.cmle_or.{est,lower,upper}` leaves
  as Undefined (`leaf(NA)`). (b) Once `strata` resolved, the same
  expansion inverted the case's exposure flag: `exposed = case_exp - 1`
  writes the row-1 (exposed) case as 0 and the row-2 (unexposed) case
  as 1 — the Mantel–Haenszel block above it uses the correct
  orientation (`case_exp == 1` → exposed), so `matched_n.mh_or` was
  right while `matched_n.cmle_or` carried the flipped estimate. A
  hand-written conditional likelihood from `matched_sets` gives
  est 1.721389 (1:2) / 2.175073 (1:4) with the correct orientation and
  exactly the first cut's 1.592878 / 3.764807 with the case flipped.
- **Fix (`fixtures/R/calc.R` only).** `survival::clogit` re-evaluates a
  literal `coxph` call in its caller's frame and resolves `Surv()` and
  `strata()` through the formula's environment, so `coxph`/`Surv` are
  bound in a `local({…})` scope and the strata term is the namespaced
  `survival::strata(stratum)` (namespaced specials are detected under
  R 4.6.2); the case's exposure flag is `2 - case_exp` (row 1 =
  exposed). The method is unchanged: conditional logistic regression,
  one stratum per matched set, Wald CI on the coefficient,
  `survival::clogit`. The `tryCatch`/NULL branch is removed — a real
  clogit failure now stops generation instead of writing Undefined.
- **Generation-time guard (fix round 1).** `matched_cmle_direct`
  refits the conditional likelihood straight from `matched_sets` —
  set contribution `psi^e / (t*psi + M + 1 - t)`, `t = e + k` — and
  `check_clogit_direct` asserts the clogit estimate agrees to 1e-6
  relative; `check_pair_cmle` asserts the M = 1 direct fit equals the
  classic pair-matched CMLE `b/c` when `b, c > 0`. Demonstrated: with
  the guard in place and the inverted flag still present,
  `Rscript fixtures/generate.R` stops with `clogit direct-fit check
  failed for matched_1to2.matched_n.cmle_or: direct=1.721389216246425
  clogit=1.5928784011722343`; after the fix generation completes.
- **Changed-leaf census** (`git diff` plus a leaf-level walk of every
  JSON under `fixtures/`, before vs after regeneration): exactly six
  leaves — `matched_1to2` and `matched_1to4`
  `expected.matched_n.cmle_or.{est,lower,upper}`, null → numbers. Every
  other fixture file is byte-identical, including the LO-regenerated
  `import/xls_basic/input.xls`. The only other local drift is the known
  platform-ulp pair in `t_quantile.json` (`qt(0.25, 1)`/`qt(0.75, 1)` →
  ∓1.0000000000000002 on this machine's R build, exactly ∓1 on the
  workflow build — owner-accepted CI-generator bytes, PR #71 option B1);
  that file was restored and is unchanged in this PR.
- **Independent check** (fresh `Rscript`, code + output pasted in the
  PR): the expansion and the fit were rebuilt from each fixture's
  `inputs.matched_sets` — `survival::coxph(…, method = "exact")` on a
  freshly expanded data frame plus a hand-rolled conditional-
  likelihood score root from `matched_sets` itself. All six leaves are
  bit-identical to `coxph(method = "exact")`, and the hand-rolled
  estimates agree to ~1e-11 relative. The first cut's "independent
  check" had re-run `clogit` on the generator's own expansion, so it
  could not catch the orientation bug; this round's check re-derives
  the expansion from the fixture inputs. Fixture values:
  `matched_1to2` est 1.7213892162463522, CI
  [0.9480759477623332, 3.125467786418363]; `matched_1to4` est
  2.1750734364802318, CI [1.1954040577809311, 3.9576111719615006].
- **`smr` `conf_level` kind → `percent`.** One line in
  `docs/v1.0-calculators.md`: the committed
  `fixtures/calc/smr/conf_97` case (conf_level 97) is authoritative —
  any level is allowed. Ury–Wiggins (90/95/99 only) and Vandenbroucke
  (95 only) stay `Undefined{reason}` elsewhere, as the inventory line
  already states.
- **Known CI consequence.** The engine still emits
  `Undefined{reason}` for `matched_n.cmle_or` (the stacked engine
  follow-up implements it), so the engine corpus check fails on these
  six leaves by design in this PR; no `crates/` code is changed.
- Files outside the follow-up's scope list: `docs/decisions.md` (this
  entry).

### v1.0 Task 7 follow-up — exact-path size limits (`too_large`), 1:n matched `cmle_or` (2026-11-15)

Decision: Implement the owner-approved Task 7 follow-up (PR #74 comment
6898203468, stacked on `v1.0/task-2-clogit-strata` / PR #75).

- **Exact-path size guards (D1).** New `UndefinedReason::TooLarge`
  (wire `too_large`). In `two_by_two`/`two_by_n`, leaves computed by
  enumerating a stratum's hypergeometric support (Fisher exact and
  mid-p p-values, the exact conditional OR `cmle_or.*`, its mid-p and
  Fisher limits) return `{"undefined":"too_large"}` when that stratum's
  support (`max − min + 1` of cell `a` given the margins, computed
  arithmetically, before any allocation) exceeds
  `MAX_EXACT_SUPPORT = 1_000_000`. The stratified `martin`
  `adjusted.cmle_or.*` leaves return `too_large` when the convolution
  work exceeds `MAX_EXACT_CONVOLUTION_WORK = 100_000_000`. Every
  non-exact leaf still computes.
- **k > 2 convolution-work generalisation — for owner review.** The
  owner's rule defines work for two strata as `support₁ · support₂`.
  For k > 2 strata `martin` convolves one distribution at a time, so
  the actual work is `Σ_{i≥2} (support of the polynomial convolved so
  far) × support_i`, where the running support is
  `cum + support_i − 1`; this equals `support₁ · support₂` at k = 2.
  Only informative strata enter the convolution.
- **1:n matched conditional MLE (D3).**
  `calc.matched_case_control.matched_n.cmle_or.{est,lower,upper}` is
  now a real conditional MLE matching `survival::clogit` (one stratum
  per matched set, Wald CI on the coefficient): the score is aggregated
  over the ≤ 2·(M+1) stratum shapes `(e, k)` with multiplicities —
  O(M) memory — `p = t·ψ / (t·ψ + M + 1 − t)`, `t = e + k`,
  `ψ = exp(β)`; a safeguarded Newton solve on the bracketed score root;
  `SE = 1/√I(β̂)` from the observed information. Verified against
  `clogit`: `matched_1to2` β 0.5431316487 → OR 1.7213892162463522;
  `matched_1to4` β 0.7770624279 → OR 2.1750734364802318. Boundary
  inputs (no informative sets, discordance in one direction only) are
  `Undefined{unbounded}`: `survival::clogit` reports a flat likelihood
  (`NA` coefficient) or a diverging coefficient ("beta may be
  infinite") respectively — the existing unbounded/undefined-MLE
  reason.
- **Files outside Task 7's Files block:**
  `crates/epiora-core/src/stats/estimate.rs` (new
  `UndefinedReason::TooLarge`), `crates/epiora-wasm/bindings/
  Estimate.ts` (regenerated union), `app/src/ui/strings.ts` (new
  `undefined.too_large` copy), `docs/decisions.md` (this entry).
  `app/src/worker/errors.ts` mirrors `EngineError` codes, not
  `Estimate` reasons — unchanged.
Evidence: PR opened against `v1.0/task-2-clogit-strata`;
`cargo test --test calc_counts` (owner-limit and clogit-parity tests);
wall times measured release-native (commands in the PR body).
Supersedes: —

### v1.0 Task 8 — Person-time and continuous-summary calculators (2026-11-16)

Decision: Implement `rate_one`, `rate_compare`, `mean_ci`, `median_ci`,
`t_test_summary`, `anova_summary` to the Task 2 corpus, sharing the
MEANS summary builders and the `martin` polynomial machinery.

- **Shared summary builders.** `GroupSummary`, `summarize`,
  `t_tests_from_summary`, `anova_from_summary` and
  `bartlett_from_summary` moved verbatim from `stats/means.rs` (was at
  the ~400-line limit) into the new `stats/means_summary.rs`;
  `stats/means.rs` re-exports them so `stats::means::*` paths and all
  MEANS fixture outputs are unchanged. Added
  `t_tests_from_summary_at(a, b, conf)` — the same builder with the CI
  level parameterised (`t_tests_from_summary` = `…_at(…, 95.0)`); the
  `t_test_summary` calculator calls it so MEANS and the calculator
  share one code path (`t`, `df`, `p`, `diff`, `lower`, `upper`).
- **Person-time CMLE rate ratio.** `counts/martin.rs` generalized:
  `Setup`/`Poly` are crate-visible, `convolve` takes `(a, poly)` pairs
  and `cmle(&Setup, approx, conf)` holds the shared root logic;
  `stratified_cmle` is a thin case-control wrapper (identical
  behaviour). `person_time/martin_pt.rs` adds `poly_strat_pt`
  (`C(m1,k)·(n0/n1)^(m1−k)`, `m1 = rint(a+b)`, zero-PT arms →
  single-point mass) per `tables.R`.
- **Bounded work (D1 applied to the binomial path).** The corpus's
  `large_counts` fixture (m1 = 1.2e6) must compute, so the 1e6
  *hypergeometric*-support cap — which is defined for the 2×2
  polynomial — cannot apply. `martin_pt::too_large` instead bounds the
  same `MAX_EXACT_CONVOLUTION_WORK` (1e8) over total work: `m1+1`
  coefficient evaluations per informative stratum plus the
  `logmult_poly` multiply pairs `Σ_{i≥2} s_{<i}·s_i`. Checked
  arithmetically before any allocation; all five `cmle_rr.*` leaves
  return `{"undefined":"too_large"}` when exceeded.
- **Zero-event rate CIs (Review Focus 3).** `cases = 0` → every
  method's lower limit is exactly `0`; mid-p/Fisher/normal/Byar
  uppers are finite: mid-p/Fisher come from the count-scale helpers
  (`[0, finite]`), normal/Byar lowers are clamped to `0`.
  Fix round 1 (review, PR #77):
  `rg_ci` no longer collapses to the fabricated point interval
  `[0, 0]` — it returns R's own degenerate values `[0, NaN]`
  (`exp(-Inf) = 0`, `exp(NaN) = NaN`), so `rate_one`'s
  `rothman_greenland.upper` is `{"undefined": "out_of_range"}`, the
  same leaf `smr` emits for the identical formula at `o = 0`.
- **`rate_compare` strata input contract (fix round 1, PR #77).** The
  `strata` table's uniform `CellKind::Real` let negative/non-integer
  case counts, non-positive person-time and >9 nested strata compute.
  Each stratum is now checked as one 2×2 — row 0 integers
  `0 ≤ n < 2^53` (integral reals like `4e15` count, since JSON writes
  them with an exponent), row 1 finite `> 0` — as typed
  `calc_input_out_of_range` errors on `strata`, and `input::table`'s
  nested branch bounds the outer list at `max_rows` (the Task 7 gap
  also affected `two_by_two`/`two_by_n`; only the upper bound applies
  to the outer list — `dose_response` fixtures carry single-stratum
  outer lists below `min_rows`).
- **`median_ci` method minimum.** The normal-approximation rank CI
  needs an expected observation on each side of the percentile:
  `n·p < 1` or `n·(1−p) < 1` → `Undefined { too_few }` on both ranks
  (covers the degenerate endpoints percentile 0/100 where `npq = 0`,
  matching OpenEpi `CIMedian.js` refusing `percentile ≤ 0`). All four
  corpus fixtures (np ≥ 1.5) stay defined; raw unrounded ranks are
  emitted per the fixture oracle, not OpenEpi's rounded display.
- **`rate_one` `per_unit` when omitted** (`DefaultValue::Omit`):
  OpenEpi `PersonTime1.js` auto-scales — smallest power of 10 making
  the raw rate ≥ 1; a zero rate keeps `1` (OpenEpi's loop never exits
  there).
- **`ef_person_time` clamp** uses `f64::clamp`, which propagates NaN
  like R's `max(-1, min(1, x))` — Rust's `min`/`max` would swallow it.
- **`t_test_summary_equals_means_on_same_data`** uses `means_ties`
  (the two-group corpus dataset): `means_basic` has three groups so
  its MEANS output contains no t-test to compare against — the plan's
  intent (same summaries → same t/df/p within 1e-12) is preserved.
- **Files outside the Files block:** `stats/means_summary.rs` (new;
  the conductor note mandates the split since `stats/means.rs` was at
  the ~400-line limit), `stats/means.rs` and `stats/mod.rs` (re-export
  + module decls), `calc/counts/martin.rs` and `calc/counts/mod.rs`
  (poly-generic core + `pub(crate)` visibility so person_time reuses
  `shared`/`martin`), `crates/epiora-core/src/bin/corpus-runner.rs`
  and `crates/epiora-wasm/parity/run.mjs` (conductor note requires the
  six family paths in both FAMILIES lists), and the validation
  registry `mod.rs` + `calc_rate_one.rs` / `calc_rate_compare.rs` /
  `calc_continuous.rs` (the registry is split one file per statistic
  family — no single `registry/calc.rs` exists to modify).
- **Strata-list input contract (fix round 2, PR #77).** The strata
  bound is now registry data — `InputKind::Table` carries
  `strata: Option<u8>`: `Some(bound)` means the input is a strata
  list (the flat single-stratum form or `1..=bound` nested tables),
  `None` rejects a list-of-tables as `bad_table_shape`
  (`screening`/`r_by_c`/`matched_case_control`/`anova_summary`
  previously accepted and then panicked or silently misread nested
  input). The outer cap is the inventory strata bound, not
  `max_rows` — `dose_response` keeps 2..10 level rows per stratum but
  caps at 9 strata. A 2×2-stratum input (`two_by_two`, `two_by_n`,
  `rate_compare`) is now `min_rows = max_rows = 2`: a 3-row stratum
  was silently truncated and a 1-row stratum silently padded; both
  are `bad_table_shape` flat or nested. `dose_response`'s `strata`
  integer is cross-checked against the list length and all strata
  must carry the same level count (the crude table and the MH loop
  index levels by position) — mismatches are `bad_table_shape` on
  `strata`. `anova_summary` gains the `rate_compare::check_strata`
  pattern: each `[n, mean, sd]` row needs `n` integer `≥ 2` and
  `< 2^53`, `mean` finite, `sd` finite `> 0` — `n = 0` underflowed
  `g.n − 1` and near-`u64::MAX` overflowed `n_tot` in
  `stats::means_summary`. `mean_ci` rejects a finite
  `population_size < n` (`below_min`); inventory requires `≥ n` or
  `0` = infinite. `t_test_summary`/`median_ci` audit clean — their
  declared bounds already match the inventory (n ≥ 2, sd > 0,
  percentile 0–100). `cols == 0` tables must be uniform-width: a
  ragged `r_by_c` rows array indexed out of bounds.
- **Files outside the Files block (fix round 2):**
  `calc/input.rs` (strata-bound split into `flat_table`/`table`,
  uniform `cols == 0` width), `calc/mod.rs` (the `strata` registry
  field — inside the block's "registry entries" but noted),
  `calc/counts/defs.rs`, `calc/counts/defs_rest.rs`,
  `calc/registry_rate.rs`, `calc/registry_continuous.rs` (registry
  `strata` values), `calc/counts/dose_response.rs` (strata
  cross-check), `calc/continuous/anova_summary.rs` (row contract),
  `calc/continuous/mean_ci.rs` (population check),
  `crates/epiora-wasm/bindings/InputKind.ts` (regenerated for the new
  field), `crates/epiora-core/tests/calc_registry.rs` (the round-2
  tests).
Evidence: `cargo test -p epiora-core --all-features` all green —
`every_person_time_fixture_matches` (11 fixtures incl. `zero_event`,
`support_max`, `large_counts`), `every_continuous_fixture_matches`
(15 fixtures), `zero_events_rate_ci_defined_lower_zero`,
`rate_compare_too_large_is_bounded`,
`t_test_summary_equals_means_on_same_data`,
`median_ci_needs_minimum_n`, `every_output_has_registry_coverage`,
`every_means_fixture_matches`, `golden_v1_unchanged`.
Supersedes: —
### v1.0 Task 8 follow-up — owner decisions T1–T3 (2026-11-16)

Decision: Apply the owner's three decisions recorded on [PR #77,
comment
6904346253](https://github.com/dorrabbkhan/Epiora/pull/77#issuecomment-6904346253)
(escalation [comment
6902876797](https://github.com/dorrabbkhan/Epiora/pull/77#issuecomment-6902876797)).

- **T1 — person-time CMLE rate-ratio support limit (owner-chosen
  threshold).** `MAX_PT_SUPPORT = 2_000_000` next to the D1
  constants in `calc/counts/shared.rs`: any informative stratum with
  `m1 = rint(a + b) > 2e6` turns all five `cmle_rr.*` leaves
  `{"undefined": "too_large"}` — stratum and crude alike (the pooled
  `m1` is the sum, so it crosses with the stratum). The 1e8
  `MAX_EXACT_CONVOLUTION_WORK` total-work cap stays in addition; both
  are checked arithmetically before any allocation in
  `martin_pt::too_large`. The `rate_compare/large_counts` fixture
  (m1 = 1.2e6) still computes. Measured on a release build
  (`/usr/bin/time -v`): m1 = 1.2e6 → 2.44 s / 30.6 MB peak RSS;
  m1 = 2e6 → 6.14 s / 49.2 MB; m1 = 2e6+1 → instant `too_large`.
- **T2 — `median_ci` minimum n.** The rule
  `n·p < 1 or n·(1−p) < 1 → Undefined { too_few }` (covers the
  degenerate percentile 0/100) is accepted and now recorded in
  `docs/v1.0-calculators.md` under `## median_ci` as a "Minimum n"
  line. Docs-only — the engine already implemented it.
- **T3 — `t_test_summary` `se_i`/`var_i` alternatives.** The
  inventory's "one of `sd_i`/`se_i`/`var_i` real >0" per group is now
  expressed: `CalculatorDef` gains `groups: &[InputGroup]` (each
  `InputGroup.members` lists alternative input names, canonical
  first; members are declared `Omit`-defaulted inputs), and
  `input::validate` enforces exactly-one — none given →
  `calc_input_missing` on the member list, more than one →
  `calc_input_out_of_range` on the member list with the existing
  `bad_choice` reason (no new code or reason). The same construct
  serves the Task 9 "one of" lines (`ss_unmatched_cc`, `ss_cohort`)
  and Task 14's renderer via the regenerated `InputGroup.ts` /
  `CalculatorDef.ts` bindings. `t_test_summary` converts `sd = se·√n`
  and `sd = √var`; the receipt's canonical inputs record only what
  the user gave.
- **Files outside the Files block:** none beyond the follow-up
  scope — `calc/counts/shared.rs` (the constant), `calc/person_time/
  martin_pt.rs` (the check), `calc/mod.rs` (`InputGroup` + the
  `groups` field), `calc/input.rs` (cardinality check),
  `calc/registry_continuous.rs` + the other six registry files
  (`groups` field on every literal), `calc/continuous/
  t_test_summary.rs`, `crates/epiora-wasm/bindings/{CalculatorDef,
  InputGroup}.ts` (regenerated), `docs/v1.0-calculators.md`,
  `tests/{calc_person_time,calc_continuous,calc_registry}.rs`.
Evidence: `cargo test -p epiora-core --all-features` green —
`pt_support_limit_is_2e6`, `rate_compare_too_large_is_bounded`,
`t_test_summary_se_and_var_equal_sd`,
`t_test_summary_needs_exactly_one_spread`,
`group_members_are_declared_optional_inputs`,
`every_person_time_fixture_matches` (incl. `large_counts`),
`every_continuous_fixture_matches`.
Supersedes: —
### v1.0 Task 8 follow-up — martin CMLE roots via `zeroin` (2026-11-16)

Decision: `martin_root` (the shared root step behind both the
person-time `cmle_rr` leaves and the Task 7 counts-path
`adjusted.cmle_or`) now solves with `special::zeroin` — the
R_zeroin2 port — at tol = 1e-14, the tolerance both
`stratified_cmle_pt_one` and `stratified_cmle_one` pass to
`uniroot` in `fixtures/R/methods`. `brent_root`'s absolute-only
tolerance can never satisfy its stopping rule at roots ≳ ~500, so
large-`m1` cases returned `{"undefined": "out_of_range"}` where R
returns finite roots. The same change fixes the shared
`adjusted.cmle_or` path for count tables with large margins.

- **Ill-conditioned extreme `est` (no oracle value).** For the
  person-time stratum `[[m1−1, 1], [2e6, 1e6]]` at m1 = 1.2e6 and
  2e6, R's `est` is an iteration-landing artifact: `martin_func`
  has no zero there, only a quantized residual plateau (R evidence:
  f(600131) < 0, f(600132) > 0, f(600133) < 0). The regression
  test `martin_cmle_large_roots_match_r` therefore asserts only
  that those `est` leaves are finite `Value`s (the regression was
  `out_of_range`); every other leaf still matches R at the
  `iterative` class — all five leaves at m1 = 100/1e3/1e5, the
  four limits at m1 = 1.2e6/2e6, and all five `adjusted.cmle_or`
  leaves.
- **Files outside the Files block:**
  `crates/epiora-core/src/calc/counts/martin.rs` (brent_root →
  zeroin at 1e-14), `crates/epiora-core/tests/calc_counts.rs` (the
  regression test), `docs/decisions.md` (this entry).

Evidence: `cargo test -p epiora-core --all-features` green —
`martin_cmle_large_roots_match_r`, `every_counts_fixture_matches`,
`every_person_time_fixture_matches`, `large_counts_terminate`,
`exact_paths_too_large_above_owner_limits`; full corpus 0
mismatches (calc 91 fixtures/3,207 leaves + means 10/484 — the
review's 3,691-leaf convention — plus the 44 v0.2 procedure
fixtures/5,937 leaves). Release `/usr/bin/time -v` after the
change (corpus-runner on scratch `calc.rate_compare` fixtures):
m1 = 1.2e6 → 0:04.18 / 30,860 kB; m1 = 2e6 → 0:06.78 / 49,524 kB;
m1 = 2e6+1 → 0:00.00 / 3,060 kB (all five leaves `too_large`);
9 strata × 2e6 → 0:30.59 / 49,700 kB (all strata compute; crude
`too_large`).
Supersedes: —
### v1.0 Task 8 follow-up — T4 total person-time support cap (2026-11-16)

Decision: Apply the owner's T4 decision recorded on [PR #78, comment
6912652577](https://github.com/dorrabbkhan/Epiora/pull/78#issuecomment-6912652577)
(escalation [comment
6905524130](https://github.com/dorrabbkhan/Epiora/pull/78#issuecomment-6905524130))
— option (a): cap `Σm1` across the call's informative strata at
2,000,000 for `rate_compare` `cmle_rr`.

- **T4 — `MAX_PT_TOTAL_SUPPORT = 2_000_000`** next to
  `MAX_PT_SUPPORT` in `calc/counts/shared.rs`. `rate_compare`
  computes each stratum's `cmle_rr` on its own single-stratum slice,
  so `martin_pt::too_large` never saw the call's total; the new
  `martin_pt::call_too_large` sums `m1 = rint(a + b)` over the
  call's informative strata (no convolution-work term — the strata
  are never convolved together) and `rate_compare::run` decides it
  once before any per-stratum work. Above the cap every stratum's
  and the crude `cmle_rr.*` leaves are `{"undefined": "too_large"}`;
  below it nothing changes. `too_large` itself also sums `m1` over
  its slice — a multi-stratum slice trips at the same total — while
  the per-stratum `MAX_PT_SUPPORT` and the 1e8
  `MAX_EXACT_CONVOLUTION_WORK` checks stay. Pure arithmetic before
  any allocation. `large_counts` (m1 = 1.2e6, single stratum) still
  computes.
- **Files outside the Files block:**
  `crates/epiora-core/src/calc/counts/shared.rs` (the constant),
  `crates/epiora-core/src/calc/person_time/{martin_pt,rate_compare}.rs`
  (the checks and the call-level plumbing),
  `crates/epiora-core/tests/calc_person_time.rs` (the new test plus
  added cases in `rate_compare_too_large_is_bounded`),
  `docs/decisions.md` (this entry).
Evidence: `cargo test -p epiora-core --all-features` green —
`pt_total_support_limit_is_2e6`, `pt_support_limit_is_2e6`,
`rate_compare_too_large_is_bounded`,
`every_person_time_fixture_matches` (incl. `large_counts`, 0
mismatches). Release `/usr/bin/time -v` (corpus-runner on scratch
`calc.rate_compare` fixtures, exact commands in the PR body):
2 strata × m1 = 1e6 (Σ = 2e6, worst admitted multi-stratum) →
0:02.42 / 49,792 kB, all leaves compute; 1 stratum × m1 = 2e6 →
0:04.24 / 49,496 kB, computes; 9 strata × m1 = 2e6 →
0:00.00 / 3,368 kB, every stratum's and crude `cmle_rr` leaves
`too_large` (was 0:30.59 / 49,700 kB computing strata before T4).
Supersedes: —

### v1.0 Task 2 follow-up — real-valued sample sizes (2026-11-16)

Decision: Owner decision 2026-11-16 (v1.0 Task 9, C1: "Fixture PR
first" — a Task 2 follow-up adds real-value (pre-ceiling) leaves via
`generate.R`; Task 9 stacks on it). Every ceiling-produced `ss_*`
leaf gained a `<leaf>_real` sibling (class `closed_form`) holding the
value the ceiling was applied to:

- `ss_proportion`: `n_openepi_real`, `n_ei7_real` and
  `cluster_size_real` per confidence level (`{80,...,9999}`); the
  EI7 `_real` values are `DEFF · round(n0/(1+n0/N)) / clusters` — the
  expression the ceiling is applied to, `round(n)` inside kept as
  written. `total` (clusters × integer cluster size) is a product,
  not a ceiling — no `_real` sibling.
- `ss_unmatched_cc` / `ss_cohort`: per method (kelsey, fleiss,
  fleiss_cc) `{cases|exposed}_real` = vn,
  `{controls|unexposed}_real` = vn·r (the second ceiling's argument),
  `total_real` = their sum.
- `ss_mean_difference`: `n1_real` = vn; `n2_real` = the argument of
  n2's ceiling as R computes it — `ceiling(n1) · ratio`, the
  already-ceiled n1 (OpenEpi `SSMean.js` `n2=Math.ceil(n1*ratio)`
  does the same), so `n2_real` is integer-valued. `total_real` =
  `n1_real + n2_real`. The alternative `n1_real · ratio` was not
  used: `ceiling(330.671) = 331 ≠ n2 = 332` on `high_power`, which
  would break `check_ss_real` and Task 9's `rounded_up_is_ceiling`
  (`rounded == ceil(real)`). Flagged for owner review in the PR.
- Generation-time guard `check_ss_real` (fixtures/R/calc.R): for
  every case, `ceiling(x_real) == x` elementwise for each ceiling
  pair. `total` pairs are not asserted — they are sums/products, not
  ceilings (`ceiling(a)+ceiling(b) ≠ ceiling(a+b)` in general).

Census (`git diff` + leaf walk vs main): 14 fixture files, 156 added
leaves, 0 removed, 0 changed — ss_proportion 4 files × 21,
ss_unmatched_cc 3 × 9, ss_cohort 4 × 9, ss_mean_difference 3 × 3.
`fixtures/special/t_quantile.json` drifted by one ulp on the two
known Cauchy-`tanpi` leaves on regeneration (PR #71 precedent) and
was restored from main.

Independent check (fresh R session, formulas typed from the
OpenEpi/inventory source, not the methods file): 14 comparisons
across `clustered`, `odds_ratio_input`, `risk_difference_input`,
`high_power` — all rel = 0 (≤ 1e-9 required).

Files outside the task's Files block: `docs/v1.0-calculators.md`
(one `_real` output line per group, owner decision C1). `fixtures/`
changed only via `Rscript fixtures/generate.R`.
Supersedes: —

### v1.0 Task 9 — sample-size, power and random-number calculators (2026-11-16)

Decision: Implement the remaining four sample-size calculators
(`ss_proportion`, `ss_unmatched_cc`, `ss_cohort`, `ss_mean_difference`),
the three power calculators (`power_unmatched_cc`, `power_cohort`,
`power_mean_difference`) and `random_numbers`, exactly as the
inventory and `fixtures/R/methods/sample_size.R` specify; remove the
Task 7 fail-closed family gate in `calc::run`.

- Owner decision C1 (2026-11-16, stacked base): this branch is stacked
  on `v1.0/task-2-ss-real-values` (f0c89a7, PR #79). Both the
  `<leaf>_real` and the rounded-up leaves are emitted exactly as the
  fixtures name them, each with a validation REGISTRY row;
  `rounded_up_is_ceiling` asserts `rounded == ceil(real)` for every
  ceiling pair in every `ss_*` fixture (`total` excluded — a
  sum/product, not a ceiling).
- Owner decision C2 (2026-11-16, `random_numbers` inputs): the
  inventory/registry ids stay `lower`, `upper`, `count`, `seed`,
  `columns` (`columns` display-only); `unique` and `sorted` added as
  Choice `no|yes`, default `no`. Bounds: `lower ≤ upper` (equal
  allowed), `upper − lower + 1 ≤ 2^32`, `seed` 0..2^53−1, `count`
  1..10,000, `unique = yes` requires `count ≤ upper − lower + 1` —
  all else `calc_input_out_of_range`. The registry def and the
  inventory section were updated accordingly.
- `random_numbers` algorithm (platform §7.4, spec §5.2):
  `ChaCha20Rng::seed_from_u64(seed)`; each draw is Lemire's
  nearly-divisionless bounded integer — `next_u32()` multiplied by
  `span = upper − lower + 1`, the high 32 bits kept, the low 32 bits
  rejected below `(2^32 − span) mod span`. `unique = yes` redraws
  duplicates; `sorted = yes` sorts ascending after drawing. i64
  output; no floating point anywhere in the path. It has no R
  fixture (spec §5.2): its coverage row points at the golden file
  `crates/epiora-core/tests/golden/random_numbers.json`, a
  corpus-shaped file (`procedure`/`inputs`/`expected`) carrying the
  three golden seeds (0, 1, 20261112) — the seed-0 case doubles as
  the `run_case`/`compare` coverage fixture, so no coverage-rule
  change was needed.
- Owner amendment C3 (2026-11-16, quoted): "keep OpenEpi's one-sided
  Φ(powerz) formula. The test (name unchanged) asserts that the
  `normal` variants (power_unmatched_cc.normal, power_cohort.normal,
  power_mean_difference.power) equal α/2 as a % — (100 − conf_level)/2
  — within 1e-9 at zero effect for conf 90, 95, 99, and that the CC
  variants are `Undefined` there (never NaN)." Amends the plan's
  `zero_effect_power_equals_alpha` expectation after the previous
  session's STOP.
- Review Focus 3: equal proportions — `percent_*` pairs equal, or
  derived through `odds_ratio` 1 / `risk_ratio` 1 / `risk_difference`
  0, and `ss_mean_difference` difference 0 — turn every affected
  sample-size leaf, real and rounded, into
  `{"undefined": "infinite_sample_size"}` via the new
  `UndefinedReason::InfiniteSampleSize` (code `infinite_sample_size`).
  No cap on large finite sample sizes was added.
- New dependencies (all MIT OR Apache-2.0 per the crates' own
  metadata; `cargo deny --offline check` clean — advisories, bans,
  licenses, sources all ok): `rand_chacha` 0.10.0, `rand_core`
  0.10.1, `ppv-lite86` 0.2.21 (rand_chacha's SIMD back-end).
- Engine wasm size (`scripts/wasm-build.sh release` + `gzip -9 -c
  crates/epiora-wasm/pkg/epiora_wasm_bg.wasm | wc -c`, measured on this
  machine): 157,914 bytes at base f0c89a7 (worktree build) → 159,390
  bytes with the new calculators and dependencies (the
  `./scripts/check.sh` full run's own gate value) — +1,476 bytes,
  well inside the 409,600 budget.
- Files outside the task's Files block: `docs/v1.0-calculators.md`
  (C2's `unique`/`sorted` inputs and the algorithm note, both
  mandated by the conductor notes); `app/src/ui/strings.ts` (display
  string for the plan-mandated `infinite_sample_size` reason — the
  new code must be renderable);
  `crates/epiora-wasm/bindings/Estimate.ts` (regenerated by
  `cargo test -p epiora-core --features ts --lib export_bindings`
  after the new `UndefinedReason`); `Cargo.lock` (the three new
  dependencies); `crates/epiora-core/src/bin/corpus-runner.rs` and
  `crates/epiora-wasm/parity/run.mjs` (the conductor-mandated
  FAMILIES additions); `crates/epiora-core/tests/calc_registry.rs`
  (`every_output_has_registry_coverage` widened to the now-implemented
  families — the plan's "registry coverage test" update);
  `crates/epiora-core/src/validation/registry/{calc_sample_size,
  calc_power,calc_random}.rs` + `mod.rs` (the plan's
  `registry/calc.rs` — the repository's actual split-file
  architecture); `crates/epiora-core/src/calc/registry_power.rs`,
  `registry_sample_size.rs` and `registry_sample_size_rest.rs` (the
  calculator registry defs — the conductor notes mandate the
  `registry_power.rs` edit for C2, and the `groups`/`_real` output
  declarations for C1); `crates/epiora-core/src/calc/other/mod.rs` (the
  `other` family module root — the Files block names the leaf file
  only; `sample_size/mod.rs` and `power/mod.rs` fall under the
  `*.rs` globs)
  and `crates/epiora-core/src/calc/mean_diff.rs` (the
  `mean_*`/`difference` + `sd_*`/`var_*` derivation shared by
  `ss_mean_difference` and `power_mean_difference` — one helper, not
  two copies).
Supersedes: —

### v1.0 Task 0a — judge script, waits, prompt builder, usage report (2026-11-17)

Decision: Ported babelgate's conductor-efficiency scripts to Epiora per
docs/superpowers/plans/2026-11-17-epiora-v1.0-conductor-efficiency.md.
Every check is Epiora's as written in the plan: fixtures (any `fixtures/`,
`**/golden/**`, `*baseline*` file), numbers (changed numeric literal on a
tolerance/budget/threshold-keyworded line or anywhere in a plan/spec),
assertions, skips, files-block, spikes-import, licences, spike-report,
secrets, diff-stat, owner-gate — plus the same uninspectable-entry rule as
babelgate (binary/rename-only/truncated entries FLAG every check that
cannot inspect them).
Evidence: `node --test scripts/conductor/*.test.ts` — 35 tests pass;
`./scripts/check.sh` — ALL CHECKS PASSED.
Resolved interpretations and port deltas:

- **Reference run:** `node scripts/conductor/judge.ts` / `usage-report.ts`
  use Node 24's built-in type stripping (explicit `.ts` import extensions,
  no enums/namespaces/parameter properties) and `node --test` — no vitest,
  no ts-node, no new dependencies, no package.json.
- **Baseline doc numbers** come from the babelgate script's run by the
  conductor (`.conductor-ref/usage-baseline.{txt,json}`, untracked); the
  doc's own-script table was produced by
  `node scripts/conductor/usage-report.ts --since 2026-10-28`, whose new
  attribution rules re-assign most of the 57 `unattributed` sessions
  (11 remain).
- **Attribution, first match wins:** `/conduct <v> <N>` in a user message;
  a Bash `devin.sh|build-prompt.sh <kind> <v> <N>` call; an
  `epiora-conductor/<v>-task<N>` path; `<v>-task<N>` text — scanned over
  user-message text and Bash tool_use commands. Task ids must start with a
  digit (`0a`, `11`), so `/conduct v1.0 N` placeholders stay unattributed.
- **devin.sh:** body inside `main()`; empty prompt exits 2 before any
  session; the empty-prompt check runs before `app-env.sh` is sourced so a
  missing app identity can't mask it; the checkout is restored to the
  starting branch only when no tracked file is modified; stdout is
  `tail -30` plus every `PR_URL:`/`STOP:` line, de-duplicated; exit code is
  Devin's. `DEVIN_CMD` keeps `--sandbox --permission-mode bypass -p` — this
  session did not prove the flag ignored under `--sandbox`.
- **diff-stat** defaults to the newest `~/epiora-conductor/<v>-task<N>/
  *.out.md` (the plan's words; babelgate used the union of task/fix outs).
- **spike-report** runs `scripts/check-spikes.sh` and flags a Measurements
  line (inside a `Measurements`/`measurement` heading section) that
  contains a digit but no `http` link or backticked command on the same
  line; N/A when `docs/spikes/**` is untouched.
- **licences** flags added rejected-dependency lines (`gadm|powersync|
  sqlite-?see|sqlite3-see`) in the four dependency manifests and any touch
  of `deny.toml` or a licence allow-list line (`--onlyAllow`,
  `license-checker`, …) — a "conductor must look" FLAG, not a verdict.
- **owner-gate** also reads the task's Gate column in the plan's Task
  Index table (parsed by header name, so extra columns can sit anywhere)
  for `OWNER GATE`, `OWNER REVIEW` or `(human step)`.
- **judge.ts** finds the plan as the newest
  `docs/superpowers/plans/*epiora-<version>-*.md` containing
  `### Task <task>:` (v1.0 has two plan files).
- **build-prompt.sh** resolves the plan the same way; when the resolved
  file is not the one in the skill's version mapping it names the plan by
  path in the first line.
- **Port fixes found on this branch's own diff:** the babelgate glob
  compiler collapsed `**` into one path segment (the `.*` it emits was
  re-matched by the single-`*` pass), so `fixtures/usage/**` never reached
  `<session>/subagents/` — `**` is now marked before the `*` pass.
  `numbers` flags only when a keyworded line was *removed* (a changed
  literal, per the plan's "removed line ↔ added line" — pure additions are
  new code); and the licence-list scan skips `.md` files (the allow-list
  lives in scripts/config; docs mentioning it are prose).
- **Files outside the Files block:** `scripts/conductor/usage-classify.ts`
  (session-log parsing, `<v>-task<N>` attribution and the tool-mix
  classifier), `scripts/conductor/usage-render.ts` (the text/JSON report
  tables) and `scripts/conductor/judge-checks-gates.ts` (the licence,
  spike-report and owner-gate checks, plan order 7/8/11) — split out of
  `usage-report.ts` (was 562 lines) and `judge-checks.ts` (was 485) so
  every file is under the ~400-line house rule (owner decision on this
  task). `usage-report.ts` re-exports the public surface and `judge.ts`
  imports the moved checks from `judge-checks-gates.ts`, so the tests and
  what they assert are unchanged.
- **check.sh** gained exactly one step — "conductor scripts" runs
  `node --test scripts/conductor/*.test.ts` when that dir has test files,
  in both modes. (`node --test <dir>` does not scan directories on Node
  24, hence the glob.)
- Shell-script tests are node:test files that spawn bash with stub
  `devin`/`gh` on `PATH` (bats is not installed).

### v1.0 Task 10 — explain traces for every statistic (2026-11-17)

Decision: Add `epiora_core::explain` — `ExplainStep`,
`Explanation`, `ExplainKind::{ClosedForm, Method}`, `FORMULAS`,
`explain_procedure` and `explain_calculator` — covering every
registry statistic. Each explain run executes the ordinary
procedure/calculator path, extracts the exact serialized leaf for
the requested statistic, and attaches the ordered formula steps; the
last step's `result` is that leaf bit-for-bit, so
`last_step_equals_result_bitwise` holds by construction. Method
statistics (exact, iterative, enumerative) get a single-step
`ExplainKind::Method` trace naming the method — no closed-form is
fabricated.

- Owner decision D1 (2026-11-16, quoted): "results_unchanged uses a
  per-fixture hash golden. BEFORE changing any engine code, on a
  clean checkout of main (6e8978a), generate
  `crates/epiora-core/tests/golden/corpus_parity_main.tsv`: one row
  per corpus-runner line, in corpus-runner order, with the fixture's
  stable key (its `id` plus its fixture file path relative to the
  repo, so rows are unique), the sha256 of that fixture file's
  bytes, and the sha256 of the exact parity line. Write the small
  generator as a test helper or `corpus-runner` flag (e.g.
  `--hash-golden`) so it is reproducible, and put the exact command
  in the PR body. The test (feature `corpus`, like the other corpus
  tests) reruns the corpus in-process and FAILS if, for any golden
  row whose fixture file hash is unchanged, the parity-line hash
  differs, or if such a row's fixture is missing from the run. Rows
  whose fixture file changed or was removed, and fixtures not in the
  golden, are reported in the test output (not silently ignored) but
  do not fail — so later fixture-only PRs (which may not touch
  crates/) cannot break it. Assert at least one row was compared. If
  a parity line depends on a dataset file as well as the fixture
  file, include that file's hash in the row too." This resolves the
  plan's "committed output from main", which does not exist on main.
  Implemented as `corpus-runner --hash-golden`; the golden was
  committed on a clean main checkout as commit 289a3ce before any
  engine edit, generated by `corpus-runner --hash-golden fixtures >
  crates/epiora-core/tests/golden/corpus_parity_main.tsv` (127 rows).
- Non-finite handling in explain values: `ExplainStep.values` never
  carries NaN or infinity — a non-finite intermediate is omitted
  from `values` and the step's `result` is the run's own
  `Estimate::Undefined` leaf (the trace relays the leaf's reason; it
  never invents one). A `{"undefined": "<code>"}` leaf always yields
  a trace — explain explains the undefined result itself rather than
  stopping early.
- `UndefinedReason::ZeroExpected` (code `zero_expected`) added:
  `r_by_c` already emitted `{"undefined": "zero_expected"}` for a
  zero-margin chi-square via the calc leaf writer, but the enum had
  no variant for it, so the explain leaf decoder could not map the
  code back. The variant decodes the existing code — no emitted
  value or code spelling changed; `Estimate`'s ts-rs type union was
  extended to match.
- Unknown statistic → `EngineError::ExplainUnknown { statistic }`,
  code `explain_unknown`, checked after input validation so an
  invalid input still reports its own error first.
- Explain reads no clock and no environment: `explain_calculator`
  runs the ordinary calculator under a fixed `"explain"` context.
- Files outside the task's Files block:
  `crates/epiora-core/src/bin/corpus-runner.rs`,
  `crates/epiora-core/src/corpus/mod.rs` and
  `crates/epiora-core/src/corpus/run_all.rs` (D1's `--hash-golden`
  flag — fixture-key + file-hash + line-hash emission);
  `crates/epiora-core/tests/golden/corpus_parity_main.tsv` (the D1
  golden itself, committed on clean main first);
  `crates/epiora-core/src/error.rs` (the conductor-mandated
  `ExplainUnknown` variant);
  `crates/epiora-core/tests/error.rs` (`ExplainUnknown` in the
  variant/code lists, matching how typed errors were added);
  `crates/epiora-core/src/stats/estimate.rs` (`ZeroExpected` above);
  `crates/epiora-core/src/procedures/mod.rs` (the `stats_json`
  re-export loses its `corpus`-feature gate — explain needs the
  serialized MEANS tree in feature-free builds such as the wasm
  release build; the function body is untouched);
  `crates/epiora-wasm/bindings/Estimate.ts` (regenerated by
  `export_bindings` after `ZeroExpected` — the same convention as
  Task 9) and `crates/epiora-wasm/bindings/ExplainKind.ts`,
  `ExplainStep.ts`, `Explanation.ts` (regenerated for the new
  ts-exported explain types);
  and the explain module split mandated by the ~400-line limit:
  `explain/path.rs` + `explain/steps.rs` (shared machinery),
  `explain/two_by_two/{mod,rxc,stratified}.rs` (the plan's
  `two_by_two.rs` — crude, r×c and stratified explainers) and
  `explain/calc/{counts,rates,ss,tables,summary}.rs` family files
  (the plan's `calc.rs` — one file per calculator family plus
  `ss/power.rs`, `ss/random.rs`, `ss/sample_size.rs`,
  `counts/{exact,matched,screening,smr}.rs`,
  `rates/{adjusted,compare,leaf_pt,one}.rs`,
  `tables/{adjusted,dose,ef,rxc,table}.rs` sub-splits to stay under
  ~400 lines each, as the conductor notes instructed).
- Fix round 1 (PR #81): every intermediate a trace shows now comes
  from the `pub(crate)` helper the run path itself calls — parity is
  structural, not asserted. Helpers extracted from the run code into
  new files so `stats/two_by_two.rs` (307 ≤ 350), `calc/counts/
  shared.rs` (347 ≤ 400) and `calc/counts/two_by_two.rs` (163 ≤ 400)
  meet the size rule: `stats/two_by_two_parts.rs`
  (`Parts2x2`/`parts`/`z_975`/`yates_adj_of`),
  `calc/counts/shared_ci.rs` (CI/diagnostic parts incl.
  `KappaParts`), `calc/counts/ef.rs` (`EfRiskParts`/`EfOrParts`),
  `calc/counts/adjusted.rs` (adjusted-table `adjusted_parts`/`woolf`),
  and `calc/power/parts.rs` (the Fleiss `powerz` chain). In-place
  extractions: `stats/rxc.rs` (`RxcParts`), `stats/rates.rs`
  (`rd_byar_parts`/`rate_ratio_parts`/`EfPtParts`/`AdjRateParts`),
  `stats/stratified.rs` (`StratDerived`), `stats/means_summary.rs`
  (`GroupSummary::total`/`ssq`, `BartlettParts::stat`),
  `calc/counts/dose_response.rs` (`mh_or_parts`),
  `calc/counts/smr.rs` (`byar`), `calc/person_time/rate_one.rs`
  (`pu_scale`), `calc/counts/shared.rs` (`oe_cells`, `t_crit`),
  `calc/continuous/{mean_ci,t_test_summary,anova_summary}.rs`
  (`se_parts`, `groups`/`f_ratio`, `group_ci_parts`/`pooled_h_base`),
  `calc/sample_size/{proportion,two_props,mean_difference,
  unmatched_cc,cohort}.rs` (`SsPropParts`/`vn`/`group2`/
  `block_parts`/`n1_n2`/`p1p2`). The only arithmetic left in
  `explain/` is `/100.0` percent normalization of raw inputs and
  index arithmetic — every statistical quantity is a run-path
  helper call. `results_unchanged` now implements D1's
  fail-on-missing rule (an absent golden row fails when its fixture
  and dataset files still exist unchanged), and both coverage tests
  fail on missing fixture paths, `run_case` errors and rows with no
  exercised instance instead of silently `continue`-ing.
  Additional files outside the task's Files block:
  `app/src/ui/strings.ts` (`zero_expected` undefined-reason label
  and `explain_unknown` error message — the regenerated
  `Estimate.ts` adds the union member the label map must cover);
  `app/src/worker/errors.ts` (`explain_unknown` in
  `ENGINE_ERROR_CODES`, matching the `EngineError::code()` list).
- Fix round 2 (PR #81): two release-hidden bugs the CI debug build
  caught.
  - `results_unchanged` was build-environment dependent: a
    receipt-bearing row hashed `engine.git_sha`
    (`env!("EPIORA_GIT_SHA")` — `"unknown"` where the golden was
    generated, the merge SHA in CI) and the `receipt_sha256` that
    seals it. D1 implementation detail: `git_sha` is build metadata
    fixed at compile time (spec §4.3), not a result, so
    `--hash-golden` and `results_unchanged` now hash
    `CaseOutput::parity_line_normalized` — the parity line with the
    receipt parsed, `git_sha` pinned to `"unknown"` and
    `receipt_sha256` resealed over the normalized receipt exactly as
    `Receipt::sha256_hex` seals it (structural normalization, never a
    string edit; every other byte is still compared). The committed
    golden is unchanged — a
    `GIT_SHA=deadbeef corpus-runner --hash-golden fixtures` run is
    byte-identical to it. `parity_line` itself is untouched: the
    native/wasm corpus diff still compares receipts as built.
  - `GroupSummary::ssq` computed `n − 1` on `u64` and panicked in
    debug on a zero-observation `overall` (release wrapped
    silently). `n = 0`/`n = 1` now give the NaN the run path flows
    into `Undefined { too_few }` — `summarize` computes no variance
    below `n = 2` — via a shared `df()` helper the explain
    `sample_variance`/`ss_within` intermediates use too. Result bits
    for `n ≥ 1` are unchanged.
  - Additional files outside the task's Files block:
    `crates/epiora-core/src/receipt/mod.rs`
    (`canonical_sha256_hex`, the reseal primitive
    `parity_line_normalized` shares with `Receipt::sha256_hex`) and
    `crates/epiora-core/src/corpus/run_case.rs`
    (`parity_line_normalized` itself — `corpus` was already in the
    D1 list above for `--hash-golden`, but `run_case.rs` is new in
    this round).
- Fix round 3 (PR #81): the explain tests re-ran the whole
  procedure or calculator once per statistic, which made the CI
  `engine` job take 2 h 17 min (tests/explain.rs alone 7,596 s in
  the debug build). Owner decision D2 (2026-11-17, quoted):
  "Follow all your recos; 2h is way too long, do something about
  it". Each family's explain is now split into a `run` half and a
  `trace` half: `run` executes the procedure/calculator once and
  keeps the output and the run-derived intermediates a trace reads
  (`FreqRun` — first table + its serialized tree; `MeansRun` —
  result, `group_numeric` buckets, `GroupSummary`s and the
  `stats_json` tree; `TablesRun` — `TablesResult`, pooled table and
  kept strata; `CalcRun` — canonical inputs + result tree). The
  public `explain_procedure`/`explain_calculator` keep their exact
  signatures and semantics — they are "run, then trace" on the
  same code path the tests drive through the `#[doc(hidden)]`
  `run_procedure`/`run_calculator` + `ProcedureRun::trace`/
  `CalcRun::trace` entry points. `every_registry_statistic_explains`
  still calls the public API for each registry row's first covering
  fixture and asserts its `Explanation` equals the run-once trace
  bit-for-bit (792 public-API checks), so the two paths cannot
  diverge. The corpus sweep runs once per test process behind a
  `OnceLock`, shared by `last_step_equals_result_bitwise`,
  `formula_ids_are_declared`, `every_registry_statistic_explains`,
  `iterative_statistics_are_method_kind` and `results_unchanged`
  (which reuses the same run outputs); shared test machinery moved
  to `tests/explain_support/mod.rs`. Coverage is unchanged —
  counts verified identical before/after (4,228 swept leaves / 248
  undefined, 4,228 stats / 4,228 formula steps, 688 registry rows /
  4,732 concrete stats, 530 iterative stats, 127 golden rows / 0
  notes, 18 zero-observation stats, 2 unknown-statistic errors).
  - Files outside the task's Files block:
    `crates/epiora-core/src/corpus/parity_norm.rs` (owner choice
    2A — `parity_line_normalized`/`normalize_receipt` and the
    parity-line rendering they share moved out of
    `corpus/run_case.rs`, which returns to 315 ≤ 324 lines) and
    `crates/epiora-core/tests/explain_support/{mod,synth,tree}.rs`
    (the sweep machinery above — split into sweep/run-once,
    synthetic-dataset builders and result-tree navigation so every
    file stays ≤ 400 lines). No `Cargo.toml` change: the run-once
    sweep alone met the target (debug `tests/explain.rs` 72 s vs a
    492.76 s release baseline; a `[profile.dev]` opt-level bump was
    not needed).
Supersedes: —

### v1.0 Task 0b — lean context, bounded waits, log reader, headless per-task runs (2026-11-17)

Decision: Cut the conductor's fixed per-turn context per
docs/superpowers/plans/2026-11-17-epiora-v1.0-conductor-efficiency.md Task
0b. `.claude/settings.json` gains `"disableClaudeAiConnectors": true`,
`"enabledPlugins": {"superpowers@synced": false}` and six
`permissions.deny` tool entries (`Artifact`, `Workflow`, `ScheduleWakeup`,
`ReportFindings`, `ShareOnboardingGuide`, `ListAgents` — the built-ins the
conductor never uses, E8). `scripts/conductor/run-plan.sh` runs one fresh
headless `/conduct` session per task with the explicit allow-list in
`scripts/conductor/headless-tools.txt`; bounded waits
(`DEVIN_DETACH=1 devin.sh` + `wait-for.sh`, `wait-ci.sh
WAIT_CI_DEADLINE_S`) keep every wait under one tool call; the `log-reader`
Haiku subagent digests long CI/Devin logs. The settings allow list, hook
and CLAUDE.md judgment-rules text are unchanged.
Evidence: `node --test scripts/conductor/*.test.ts` — all pass;
`./scripts/check.sh quick` — QUICK CHECKS PASSED; full gate: CI.

- **Context measurements** (conductor, 2026-11-17, ~/Epiora branch
  conductor/efficiency-plan, Claude Code 2.1.322, plain
  `claude -p "/context" --model opus`). Before (repo settings as on main):
  **32.5k** — system prompt 2.8k · system tools 17.2k · MCP tools 586 ·
  MCP server instructions 2.4k · memory files 2.4k · skills 5.8k ·
  messages 1.3k; deferred: MCP tools 30k (claude.ai connectors Atlassian,
  Claude Data, Claude Docs, Meetings), system tools 23.1k. After (same
  command plus `--settings` with the two lean keys): **28.2k** — system
  prompt 2.8k · system tools 18.1k · memory files 2.4k · skills 4.9k ·
  messages 10; MCP tools, MCP server instructions and deferred MCP
  tools 0; deferred system tools 22.9k.
- **What each key saved:** `disableClaudeAiConnectors` −2.9k (586 MCP
  tools + 2.4k server instructions; the 30k of deferred connector tools is
  gone too); `enabledPlugins: {"superpowers@synced": false}` −1.3k
  (superpowers skills 16 → 0). Not set: `disableBundledSkills` and
  `skillOverrides` — babelgate measured them at +6.2k / ≈ 0.
- **Real first-turn input tokens** (conductor, `claude -p "Reply with OK
  only." --model opus --output-format json`, input + cache-creation +
  cache-read, two runs each; raw in
  ~/epiora-conductor/v1.0-task0b/first-turn-api.txt): main 44,189 → lean
  settings 37,143 → lean + the six `permissions.deny` tools 18,974 /
  18,979 (19,028 with `--permission-mode bypassPermissions`). Agent, Bash,
  Edit, Read, Skill, ToolSearch, Write stay loaded. `/context` under the
  deny list reports system tools 941 — an undercount, since a direct tool
  listing shows those tools still loaded; the API numbers are
  authoritative, the /context one is context. Meets the ≤ 20k first-turn
  target.
- **T3 Code injection:** T3 Code conductor sessions start at a median
  45.1k — ~13k of host-injected t3-code MCP tools/instructions and
  orchestration text that project settings cannot remove; the lean
  settings apply on top of it.
- **spikes-import carry-over fix (from Task 0a):** `spikes/` now matches
  only at the start of a quoted path, after `../`/`./` segments, or as a
  Rust path segment or Cargo member/path value — `"docs/spikes/"` (the
  reports directory) no longer flags. The Rust-segment pattern is built at
  runtime so judge-checks.ts's own added lines never carry the literal it
  looks for (same trick as the licence allow-list pattern).
- **run-plan.sh resolutions:** the plan file is the
  `docs/superpowers/plans/*epiora-<version>-*.md` whose Task Index
  contains both range ids (v1.0 has two plan files; `PLAN_FILE`
  overrides). The id column is `#` when present, else `Task` (the two
  plans differ). `Conductor` → `Opus|Sonnet` maps to `--model
  opus|sonnet`; an absent column defaults every task to opus with a
  warning; any other value is an error. A `RESULT:` line counts only when
  this session appended to the task's conductor-log.md (line count
  recorded before the session) — a stale `READY` no longer lets a crashed
  session slip through. `READY <PR>` polls `gh pr view --json state` every
  `RUN_PLAN_POLL_S` (default 300 s) until MERGED; CLOSED stops like
  FAILED. `RUN_PLAN_DIR` overrides `~/epiora-conductor` for offline tests;
  `DRY_RUN=1` prints the `claude -p` commands and exits 0.
- **Bounded waits:** `DEVIN_DETACH=1 devin.sh` re-invokes itself as
  `--detached-worker` under `setsid nohup` (new session, survives the
  caller's exit), writes `<out>.md` then `<out>.rc` on exit and prints the
  `.out.md` path at once. `wait-for.sh <out.md> [max_s=540]` polls every
  `WAIT_FOR_POLL_S` (default 5 s), then prints the 0a trimmed stdout and
  exits with the session's rc; past the deadline it prints `PENDING <n>
  lines` and exits 4 — a re-call resumes. Foreground devin.sh runs write
  `<out>.rc` too, so wait-for.sh joins either mode. `wait-ci.sh` honours
  `WAIT_CI_DEADLINE_S` the same way: `gh pr checks --watch` in the
  background, killed at the deadline, exit 4.
- **Files outside the Files block:** `scripts/conductor/run-plan.test.ts`
  (the plan's Step 2 shell tests need a `node --test` file; the Files
  block names only the fixtures glob), `scripts/conductor/result.sh`
  (fix round 1: headless sessions have no echo/printf, so /conduct
  appends its `RESULT:` line through this script — covered by the
  existing `Bash(scripts/conductor/*)`), and the carry-over fix the owner
  assigned to this task: `scripts/conductor/judge-checks.ts`,
  `scripts/conductor/judge.test.ts`,
  `scripts/conductor/fixtures/judge/spikes-docs-path.diff`.
Supersedes: —
### v1.0 Task 22 — assumption-note rules (2026-11-18)

Decision: Add `epiora_core::notes` — `Note { code, statistics, values,
suggests }`, `NOTE_CODES`, `rules.rs` (one pure function per code) and
`attach.rs` (one adapter per run surface). `ProcOutput<T>` and
`CalcOutput` gain `notes: Vec<Note>` beside `receipt` — never inside
it — so receipts v1/v2 stay byte-identical, v3 receipt hashes are
unchanged, and the corpus parity line (which serializes `result` +
`receipt` only) is untouched. Notes sort by code, then the sorted
statistics list; every emitted code is a `NOTE_CODES` member; every
`values` entry is a finite engine-computed number.

Rules, named thresholds (constants in `notes/rules.rs`) and sources:

- `expected_count_below_5`: any expected cell of a chi-square table
  below `EXPECTED_CELL_MIN` (= 5.0) — Cochran (1954), "Some Methods
  for Strengthening the Common χ² Tests", *Biometrics* 10(2): 417–451
  (the same 5 `stats::rxc` already uses for `low_expected`). Fires on
  the pooled 2×2 of `tables`, every `strata[]` and `crude` block of
  the `two_by_two`/`two_by_n` calculators, and `r_by_c`. Suggests the
  block's Fisher two-sided p for a 2×2, nothing for R×C (no exact
  alternative is emitted).
- `zero_cell`: an observed zero among the cells the Wald log-scale
  interval's `1/cell` standard error sums over — Agresti (2003),
  *Categorical Data Analysis* 2nd ed., §2.3.3. Fires on the pooled
  2×2 of `tables`, every `two_by_two`/`two_by_n` block, and the two
  discordant cells of `matched_case_control`'s 1:1 pair (its Taylor
  OR interval is the same construction; the 1:n block is
  Mantel–Haenszel, not Wald, so the rule does not apply there).
  Suggests the block's exact/mid-p odds-ratio estimate.
- `normal_approx_small`: `n·p` or `n·(1−p)` below `NP_MIN` (= 5.0)
  for a Wald proportion interval — Brown, Cai & DasGupta (2001),
  *Statistical Science* 16(2): 101–133. Applies only to
  `calc.proportion` (the engine's only Wald proportion interval;
  FREQ emits Clopper–Pearson/Wilson and `screening`/`two_by_n` use
  score intervals). Suggests `calc.proportion.ci.wilson.lower`.
- `unequal_variances`: Bartlett p below `SIGNIFICANCE` (= 0.05) —
  Bartlett (1937), *Proc. R. Soc. London* A 160: 268–282. Fires on
  `means.bartlett` and `calc.anova_summary.bartlett` (the only
  Bartlett statistics; `t_test_summary`'s variance test is
  `var.test`, not Bartlett). Suggests `means.t_test.welch.t` /
  `t_test_summary` for two groups, `means.kruskal_wallis.h` / nothing
  for more.
- `heterogeneous_strata`: Breslow–Day homogeneity p below
  `SIGNIFICANCE` (= 0.05) — Breslow & Day (1980), IARC
  *Statistical Methods in Cancer Research* I, §4.4; Tarone (1985).
  The procedure fires on its Tarone-corrected
  `breslow_day_tarone.p_value`; the `two_by_two` calculator fires on
  its own `adjusted.breslow_day.or_p` (the Woolf OR-homogeneity test
  the corpus records under `breslow_day`). Suggests stratum-specific
  estimates where the output has them (`two_by_two.strata[].
  odds_ratio.est`), nothing for the procedure (it emits none).

Ambiguities resolved (all recorded for the owner review the task
requires): notes for a stratified `tables` run evaluate the pooled
2×2 only — the procedure emits no per-stratum chi-square or Wald
interval; per-block notes on the calculators carry a `stratum`
index value (`crude` blocks carry none); `freq` and `means`
ungrouped emit no notes, and the remaining calculators emit none
in this PR — but several of them do carry statistics a §6.1 rule
could reach (coverage audit below), so "no §6.1 rule's statistic
is present" is not the reason.

Coverage audit — the §6.1-adjacent statistics this PR emits no
note for, checked against `calc/**` and `notes/attach.rs`:

- `rate_compare` emits `adjusted.breslow_day.{rr_p,rd_p}` (with
  `rr_chisq`/`rd_chisq`) when it has ≥ 2 strata — Breslow–Day
  homogeneity chi-squares, df = k − 1, for the directly adjusted
  rate ratio and rate difference, computed by
  `stats::rates::adjusted_rates`. `heterogeneous_strata` could
  match: §6.1 names a Breslow–Day–Tarone p, and `rr_p`/`rd_p` are
  the same homogeneity construction for RR/RD. NO note is emitted
  for them in this PR — the rule is wired to the OR-homogeneity
  p's only (`tables`' `breslow_day_tarone.p_value`,
  `two_by_two`'s `adjusted.breslow_day.or_p`); extending a rule to
  another statistic is the owner's decision, not this PR's.
- `proportion` with `compare_with` emits `ztest.{z,p}` —
  `(p − p0)/√(p0·(1−p0)/n)`, the Wald-form one-sample z-test on a
  proportion. `normal_approx_small` could match: §6.1 names a Wald
  proportion interval and the same n·p / n·(1−p) caveat applies to
  the z-test's Wald form. NO note names `ztest` in this PR — the
  note lists only `ci.wald.lower`/`upper`; extending a rule's
  statistic list is the owner's decision, not this PR's.
- `dose_response` emits `xmh.{cc,uncorrected}.{value,p}` — the
  extended Mantel–Haenszel trend chi-square (df 1), built from
  per-stratum observed-minus-expected score sums (`xmh_parts`).
  `expected_count_below_5` could match: it is a χ² test over
  expected counts. NO note is emitted for it in this PR — §6.1
  names expected cells of a chi-square table and `dose_response`
  emits no expected-cell array for the rule to read; extending a
  rule to another statistic is the owner's decision, not this
  PR's.

Files outside the task's Files block (the plan's generic "the
procedure and calculator output types" covers the new `notes`
fields, but these files are not named literally; `git diff --stat
origin/main` confirms no `stats/*.rs` and no other `calc/**` file
changed — the existing `pub(crate)` helpers `strata2x2`,
`oe_cells`, `count_table`, `groups_of`, `woolf`, `woolf_chisq` and
`chi_p` already expose every trigger intermediate):
- `crates/epiora-core/src/calc/mod.rs` — `CalcOutput` gains
  `notes: Vec<Note>`; `run` calls `notes::calc`.
- `crates/epiora-core/src/procedures/mod.rs` — `ProcOutput<T>`
  gains `notes: Vec<Note>`.
- `crates/epiora-core/src/procedures/freq.rs` — returns
  `notes: Vec::new()` (FREQ emits Clopper–Pearson/Wilson only).
- `crates/epiora-core/src/procedures/means.rs` — calls
  `notes::means` on the result.
- `crates/epiora-core/src/procedures/tables.rs` — calls
  `notes::tables_two_by_two` / `notes::tables_rxc`.
- `crates/epiora-wasm/bindings/Note.ts` and
  `crates/epiora-wasm/bindings/ProcOutput.ts` — regenerated by
  `cargo test -p epiora-core --features ts export_bindings` after
  the `notes` field and `Note` type became ts-exported; the engine
  gate fails on stale bindings (same convention as Tasks 9/10).

Owner question: extend `heterogeneous_strata` to `rate_compare`'s
`rr_p`/`rd_p` (and `normal_approx_small` /
`expected_count_below_5` to the other calculators listed above)?
Default if no answer: not extended (Task 23 shows no note for
them).
Supersedes: —

### v1.0 Task 11 — wasm exports, parity, size, fuzzing; Epi Info command generator (2026-11-17)

Decision: Land the Task 11 wasm surface and parity coverage.
`epiora-wasm` exports `means`, `calcRegistry`, `calcRun`,
`explainProcedure`, `explainCalculator` and `epiInfoCommand`; the
native `corpus-runner` and `parity/run.mjs` emit, beyond the case
`parity_line`, the `{"command": …, "id": …}` line of every
dataset-backed fixture and a trailing `{"explanation": …, "id":
…, "statistic": …}` line per `validation::REGISTRY` row (its first
covering fixture, the `[]` pattern expanded to the first concrete
instance). `export::epi_info_command` renders `FREQ A B
[STRATAVAR=S]`, `TABLES E O [STRATAVAR=S1 S2]`, `MEANS V [G]` in
Epi Info's spelling and upper case, brackets non-identifier names
in `[ ]`, and appends options with no Epi Info equivalent
(`include_missing`, non-default `ci_method`, explicit `levels`) as
`* Epiora:` comment lines.

- `calcRegistry` serializes `calc::REGISTRY` with `serde_json` and
  parses it with `js_sys::JSON::parse` rather than going through
  serde-wasm-bindgen: `Integer` input bounds use `i64::MAX` for
  "unbounded", which has no JS `number` value and made
  serde-wasm-bindgen throw. `JSON.parse` maps it to the nearest
  f64 — the same JSON-number coercion every other engine output
  already undergoes at this boundary; no bound is load-bearing at
  that magnitude.
- `runCorpusCase` now returns the case line plus (for dataset
  cases) the command line, `\n`-separated; `explainCorpusPlan`
  returns the `(statistic, fixture-key)` pairs and
  `runCorpusExplainLine` emits one explain line — so the wasm run
  prints exactly the bytes `corpus-runner` prints, including the
  `..`-rooted `calc.random_numbers` golden key.
- The corpus `FAMILIES` gains `"means"`: the Task 11 parity clause
  covers `means` cases, and the means fixture directory landed in
  Task 10 without being enrolled in the corpus. The corpus grows
  from 127 to 137 case lines (10 means fixtures); the D1
  `results_unchanged` golden is unaffected — its 127 rows all
  still match, and means fixtures are extra rows it reports but
  does not fail on.
- `.github/workflows/engine.yml` (named in the Files block) is
  unchanged: `cargo fuzz run` already covers `means_any` and
  `calc_any` — the Step-3 instruction is conditional ("add the new
  fuzz targets … if not there").
- `crates/epiora-wasm/bindings/*.ts` were regenerated by
  `cargo test -p epiora-core --features ts export_bindings` and are
  byte-identical to the committed files — the gate's diff check is
  the verification, there is nothing to commit.
- Files outside the task's Files block:
  - `crates/epiora-core/src/corpus/explain_lines.rs` (new) —
    `command_for`/`command_line`, `explain_plan`, `explain_line`
    and `concretize`, the shared command/explain parity machinery.
  - `crates/epiora-core/src/corpus/synth.rs` (new) — the
    fixture → `(dataset, options)` construction moved up from
    `tests/explain_support/synth.rs` into product code so
    `corpus-runner` and the wasm parity build share it;
    `EngineError` instead of panics.
  - `crates/epiora-core/src/corpus/mod.rs` — module/export wiring
    for the two new modules plus `load_datasets`.
  - `crates/epiora-core/src/corpus/run_all.rs` — `load_datasets`
    factored out of `corpus_lines` (the runner reuses the loaded
    map for the explain section) and `"means"` in `FAMILIES`.
  - `crates/epiora-core/src/corpus/parity_norm.rs` — `encode_bits`
    becomes `pub(crate)` with a doc comment so the explain parity
    lines encode floats exactly as `parity_line` encodes `actual`.
  - `crates/epiora-core/tests/explain_support/synth.rs` — now
    panicking wrappers over `corpus::synth` (single source for the
    synthetic-dataset construction).
  - `crates/epiora-core/tests/explain_support/tree.rs` —
    `concretize` delegates to `corpus::explain_lines::concretize`
    (the same expansion the parity explain lines use).
Supersedes: —

### v1.0 Task 12 — OpenEpi comparison in the corpus and the validation report (2026-11-17)

- `corpus::openepi::compare_openepi` runs `calc::run` on each `openepi/*.json`
  case's `cases.json` inputs (the capture's `--dry-run` check guarantees they
  equal the Task 2 `fixtures/calc/<id>/<case>.json` inputs) and compares every
  mapped value at `openepi_display` = `0.5 × 10^−d`; negative `d` (per-unit
  restatements) loosens the bound. The stored values are already on the fixture
  scale (`capture.mjs` `scaleToR` ran at capture time).
- Measured result (this task's runs): 1,282 mapped values, **278 mismatches**
  vs the engine — the same disagreements Task 3's `--compare-r` listed as
  `mismatch 279` vs R (run `node fixtures/openepi/capture.mjs --compare-r`:
  `ok 1003 mismatch 279 unresolved 0 no-r-counterpart 0 r-null 0`). The one-count
  delta is `calc.matched_case_control.exact.fisher_p_1t` in `few_discordant`:
  OpenEpi `0.3438`, R leaf `0.34374999999999994` (fails the 5e-5 bound by
  ~1e-14), engine `0.3437500000000144` (passes) — a float boundary artifact;
  engine and R agree at `iterative` 1e-6. Per the plan's Step 3 the mismatches
  are listed in the PR and escalate as `STOP: OWNER REVIEW` (Task 13 resolves).
- Files outside the task's Files block:
  - `crates/epiora-wasm/bindings/ToleranceClass.ts` — regenerated by the
    `export_bindings` tests after the required `validation.rs` doc comment
    gained `openepi_display`; doc text only, the type is unchanged.
Supersedes: —

### v1.0 Task 13 — method choices

Where Epi Info 7, OpenEpi and the engine's R method disagree, the
`OutputDef.method` label now names the tool showing the differing
method and which output is the calculator's default display:

- `"; OpenEpi shows <method> — default display"` — this output is the
  calculator's default display for its slot; OpenEpi computes a
  different method or approximation for the same number.
- `"; OpenEpi shows <method> — alternative display (default: <leaf>)"`
  — the engine also emits this alternative; the named leaf is the
  calculator's default display for the slot.
- `"; EI7 shows <output>"` — an Epi Info 7 StatCalc output the engine
  emits for reference parity; OpenEpi shows a different formula or has
  no counterpart (these statistics are not OpenEpi-mapped).

The corpus leg (`corpus::openepi::check_entry`) treats a numeric
display-rule failure on a statistic whose `method` carries
`"OpenEpi shows"` as a **documented method difference**
(`OpenEpiReport.method_differences`), reported alongside — not as — a
mismatch; a missing engine value stays a mismatch. The comparison
rule itself (`0.5 × 10^−d`, spec §9) is unchanged, as are all
fixtures, tolerances and result values. Effect on the Task 12
measurement: 1,282 mapped values → 0 mismatches + 278 documented
method differences (measured by
`cargo test -p epiora-core --features corpus --test openepi` and the
`validation-report` `## OpenEpi check` section, this branch).

Default-display choices follow OpenEpi's own "editor's choice"
(`editorschoice`/`editorschoice1` in its source) where OpenEpi marks
one — mid-P CIs (`cmle_or.midp`, `ci.midp`, `test` mid-P), Wilson
score (`ci.wilson`) — and otherwise the first row/column OpenEpi
shows (`n_openepi`, `kelsey`, pooled t-test, uncorrected `normal`
power, `xmh.cc`). `mean_ci` defaults to `t`: OpenEpi shows z and t
with no choice marked and the t interval is the honest unknown-σ
default (z shown as the large-sample check).

MEANS differences (Task 4 — interpolated Welch CLs, percentile/mode
ties) were resolved by owner decision on PRs #67/#68 and are recorded
in the Task 4 section above; `means.*` leaves are procedure outputs,
not calculator `OutputDef`s, so no `method` label applies. EI7
StatCalc hard-codes z = 1.96 (Task 1 inventory); where its formula
differs from OpenEpi's the engine emits it as the `EI7 shows`
variants below.

Every disagreement below is OpenEpi-side only: the engine values are
the R-validated ones (the `calc.<id>` fixture corpus is green — R is
the numeric oracle). Evidence cites the OpenEpi source read from the
Task 3 capture tree (`/tmp/oe/` → `openepi.com` 3.01) and the capture
fixture ids that exhibit the difference.

| Statistic | Engine method | Other tool shows | Default display | Evidence |
|---|---|---|---|---|
| `calc.two_by_two.crude.risk.exposed.lower` | `epiR::epi.2by2` (risk strata) | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series"; capture `two_by_two/case_control, two_by_two/large, two_by_two/one_inside_boundary, two_by_two/stratified, two_by_two/support_max, two_by_two/textbook` |
| `calc.two_by_two.crude.risk.exposed.upper` | `epiR::epi.2by2` (risk strata) | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series"; capture `two_by_two/case_control, two_by_two/large, two_by_two/one_inside_boundary, two_by_two/stratified, two_by_two/textbook, two_by_two/zero_cell, two_by_two/zero_event` |
| `calc.two_by_two.crude.risk.unexposed.lower` | fixtures/R/calc.R | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series"; capture `two_by_two/case_control, two_by_two/one_inside_boundary, two_by_two/stratified, two_by_two/support_max, two_by_two/textbook, two_by_two/zero_cell, two_by_two/zero_event` |
| `calc.two_by_two.crude.risk.unexposed.upper` | fixtures/R/calc.R | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series"; capture `two_by_two/case_control, two_by_two/large, two_by_two/one_inside_boundary, two_by_two/stratified, two_by_two/support_max, two_by_two/textbook, two_by_two/zero_cell, two_by_two/zero_event` |
| `calc.two_by_two.crude.risk.overall.lower` | fixtures/R/calc.R | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series"; capture `two_by_two/case_control, two_by_two/one_inside_boundary, two_by_two/stratified, two_by_two/support_max, two_by_two/textbook, two_by_two/zero_cell, two_by_two/zero_event` |
| `calc.two_by_two.crude.risk.overall.upper` | fixtures/R/calc.R | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series"; capture `two_by_two/case_control, two_by_two/large, two_by_two/one_inside_boundary, two_by_two/stratified, two_by_two/support_max, two_by_two/textbook, two_by_two/zero_cell, two_by_two/zero_event` |
| `calc.two_by_two.strata[].risk.exposed.lower` | `epiR::epi.2by2` (risk strata) | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series" |
| `calc.two_by_two.strata[].risk.exposed.upper` | `epiR::epi.2by2` (risk strata) | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series" |
| `calc.two_by_two.strata[].risk.unexposed.lower` | fixtures/R/calc.R | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series" |
| `calc.two_by_two.strata[].risk.unexposed.upper` | fixtures/R/calc.R | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series" |
| `calc.two_by_two.strata[].risk.overall.lower` | fixtures/R/calc.R | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series" |
| `calc.two_by_two.strata[].risk.overall.upper` | fixtures/R/calc.R | OpenEpi shows a modified-Wald (Agresti–Coull) interval, p̃ ± z·√(p̃q̃/ñ), z = √cscrit | this | OpenEpi `StatFunctions1.js` `modiwald` — Agresti–Coull modified Wald + z = √cscrit, displayed as "Taylor series" |
| `calc.two_by_two.crude.odds_ratio.lower` | `epitools::oddsratio(method=\"wald\")` | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.crude.odds_ratio.upper` | `epitools::oddsratio(method=\"wald\")` | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/textbook` |
| `calc.two_by_two.crude.risk_ratio.lower` | `epitools::riskratio(method=\"wald\")` | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.crude.risk_ratio.upper` | `epitools::riskratio(method=\"wald\")` | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`) |
| `calc.two_by_two.crude.risk_difference.lower` | `epiR::epi.2by2` (ARisk; `DescTools::BinomDiffCI(method=\"wald\")` — bit-iden… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/stratified, two_by_two/textbook` |
| `calc.two_by_two.crude.risk_difference.upper` | `epiR::epi.2by2` (ARisk; `DescTools::BinomDiffCI(method=\"wald\")` — bit-iden… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary, two_by_two/zero_cell` |
| `calc.two_by_two.strata[].odds_ratio.lower` | `epitools::oddsratio(method=\"wald\")` | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`) |
| `calc.two_by_two.strata[].odds_ratio.upper` | `epitools::oddsratio(method=\"wald\")` | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`) |
| `calc.two_by_two.strata[].risk_ratio.lower` | `epitools::riskratio(method=\"wald\")` | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`) |
| `calc.two_by_two.strata[].risk_ratio.upper` | `epitools::riskratio(method=\"wald\")` | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`) |
| `calc.two_by_two.strata[].risk_difference.lower` | `epiR::epi.2by2` (ARisk; `DescTools::BinomDiffCI(method=\"wald\")` — bit-iden… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`) |
| `calc.two_by_two.strata[].risk_difference.upper` | `epiR::epi.2by2` (ARisk; `DescTools::BinomDiffCI(method=\"wald\")` — bit-iden… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`) |
| `calc.two_by_two.crude.etiologic.efp.lower` | `epiR::epi.2by2` (PARisk/AFEst) | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/stratified, two_by_two/textbook` |
| `calc.two_by_two.crude.etiologic.efe.lower` | `epiR::epi.2by2` (PARisk/AFEst) | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/textbook` |
| `calc.two_by_two.crude.etiologic.pfp.lower` | `epiR::epi.2by2` (PARisk/AFEst) | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.crude.etiologic.pfe.lower` | `epiR::epi.2by2` (PARisk/AFEst) | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.crude.etiologic_or.efe_or.lower` | fixtures/R/calc.R | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/stratified, two_by_two/textbook` |
| `calc.two_by_two.crude.etiologic_or.pfp_or.lower` | fixtures/R/calc.R | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.crude.etiologic_or.pfe_or.lower` | fixtures/R/calc.R | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.adjusted.direct_rd.lower` | fixtures/R/calc.R | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.adjusted.direct_rd.upper` | fixtures/R/calc.R | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary, two_by_two/support_max, two_by_two/zero_event` |
| `calc.two_by_two.adjusted.direct_rr.upper` | fixtures/R/calc.R | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/stratified` |
| `calc.two_by_two.adjusted.mh_or.upper` | `stats::mantelhaen.test(correct=FALSE)` (uncorrected; epi.mh adds 0.5 to zero… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/stratified` |
| `calc.two_by_two.adjusted.mh_rr.lower` | `fixtures/R/methods/tables.R::mh_rr_2x2xk` (uncorrected; bit-identical to `ep… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.adjusted.mh_rr.upper` | `fixtures/R/methods/tables.R::mh_rr_2x2xk` (uncorrected; bit-identical to `ep… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `two_by_two/support_max` |
| `calc.two_by_two.crude.chisq.uncorrected.p_1t` | `stats::chisq.test(correct=FALSE)` | OpenEpi shows the same statistic's tail via Posten's pchisq approximation | this | OpenEpi `StatFunctions1.js` `pchisq` — Posten's chi-square approximation; capture `two_by_two/one_inside_boundary, two_by_two/zero_cell` |
| `calc.two_by_two.crude.chisq.uncorrected.p_2t` | `stats::chisq.test(correct=FALSE)` | OpenEpi shows the same statistic's tail via Posten's pchisq approximation | this | OpenEpi `StatFunctions1.js` `pchisq` — Posten's chi-square approximation; capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.crude.chisq.mantel_haenszel.p_1t` | `stats::mantelhaen.test(correct=FALSE)` | OpenEpi shows the same statistic's tail via Posten's pchisq approximation | this | OpenEpi `StatFunctions1.js` `pchisq` — Posten's chi-square approximation; capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.crude.chisq.mantel_haenszel.p_2t` | `stats::mantelhaen.test(correct=FALSE)` | OpenEpi shows the same statistic's tail via Posten's pchisq approximation | this | OpenEpi `StatFunctions1.js` `pchisq` — Posten's chi-square approximation; capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.crude.chisq.yates.p_1t` | `stats::chisq.test(correct=TRUE)` | OpenEpi shows its own Yates χ² and Posten tail | this | OpenEpi `TwobyTwo.js` own Yates χ² + `pchisq` Posten tail; capture `two_by_two/one_inside_boundary, two_by_two/zero_cell` |
| `calc.two_by_two.crude.chisq.yates.p_2t` | `stats::chisq.test(correct=TRUE)` | OpenEpi shows its own Yates χ² and Posten tail | this | OpenEpi `TwobyTwo.js` own Yates χ² + `pchisq` Posten tail; capture `two_by_two/one_inside_boundary, two_by_two/zero_cell, two_by_two/zero_event` |
| `calc.two_by_two.strata[].chisq.uncorrected.p_1t` | `stats::chisq.test(correct=FALSE)` | OpenEpi shows the same statistic's tail via Posten's pchisq approximation | this | OpenEpi `StatFunctions1.js` `pchisq` — Posten's chi-square approximation |
| `calc.two_by_two.strata[].chisq.uncorrected.p_2t` | `stats::chisq.test(correct=FALSE)` | OpenEpi shows the same statistic's tail via Posten's pchisq approximation | this | OpenEpi `StatFunctions1.js` `pchisq` — Posten's chi-square approximation |
| `calc.two_by_two.strata[].chisq.mantel_haenszel.p_1t` | `stats::mantelhaen.test(correct=FALSE)` | OpenEpi shows the same statistic's tail via Posten's pchisq approximation | this | OpenEpi `StatFunctions1.js` `pchisq` — Posten's chi-square approximation |
| `calc.two_by_two.strata[].chisq.mantel_haenszel.p_2t` | `stats::mantelhaen.test(correct=FALSE)` | OpenEpi shows the same statistic's tail via Posten's pchisq approximation | this | OpenEpi `StatFunctions1.js` `pchisq` — Posten's chi-square approximation |
| `calc.two_by_two.strata[].chisq.yates.p_1t` | `stats::chisq.test(correct=TRUE)` | OpenEpi shows its own Yates χ² and Posten tail | this | OpenEpi `TwobyTwo.js` own Yates χ² + `pchisq` Posten tail |
| `calc.two_by_two.strata[].chisq.yates.p_2t` | `stats::chisq.test(correct=TRUE)` | OpenEpi shows its own Yates χ² and Posten tail | this | OpenEpi `TwobyTwo.js` own Yates χ² + `pchisq` Posten tail |
| `calc.two_by_two.strata[].chisq.yates.value` | `stats::chisq.test(correct=TRUE)` | OpenEpi shows its own Yates χ² and Posten tail | this | OpenEpi `TwobyTwo.js` own Yates χ² + `pchisq` Posten tail |
| `calc.two_by_two.crude.fisher_exact.p_2t` | `stats::fisher.test` | OpenEpi shows a two-sided Fisher p of 2 × min(one-tailed), capped at 1 | default: midp_exact.p_2t | OpenEpi `MartinStats.js`/`TwobyTwo.js` — two-sided = 2×min(1-tailed), capped at 1; capture `two_by_two/one_inside_boundary, two_by_two/stratified, two_by_two/textbook, two_by_two/zero_cell, two_by_two/zero_event` |
| `calc.two_by_two.strata[].fisher_exact.p_2t` | `stats::fisher.test` | OpenEpi shows a two-sided Fisher p of 2 × min(one-tailed), capped at 1 | default: midp_exact.p_2t | OpenEpi `MartinStats.js`/`TwobyTwo.js` — two-sided = 2×min(1-tailed), capped at 1 |
| `calc.two_by_two.crude.cmle_or.midp.est` | `stats::fisher.test` (est; or.midp's is the median-unbiased, not the CMLE) + … | OpenEpi shows its own CMLE mid-P solver (MartinStats.js) | this | OpenEpi `MartinStats.js` own CMLE/mid-P iteration; capture `two_by_two/textbook` |
| `calc.two_by_two.crude.cmle_or.midp.lower` | `stats::fisher.test` (est; or.midp's is the median-unbiased, not the CMLE) + … | OpenEpi shows its own CMLE mid-P solver (MartinStats.js) | this | OpenEpi `MartinStats.js` own CMLE/mid-P iteration; capture `two_by_two/one_inside_boundary` |
| `calc.two_by_two.strata[].cmle_or.midp.lower` | `stats::fisher.test` (est; or.midp's is the median-unbiased, not the CMLE) + … | OpenEpi shows its own CMLE mid-P solver (MartinStats.js) | this | OpenEpi `MartinStats.js` own CMLE/mid-P iteration |
| `calc.two_by_two.strata[].cmle_or.midp.upper` | `stats::fisher.test` (est; or.midp's is the median-unbiased, not the CMLE) + … | OpenEpi shows its own CMLE mid-P solver (MartinStats.js) | this | OpenEpi `MartinStats.js` own CMLE/mid-P iteration |
| `calc.two_by_two.crude.cmle_or.fisher.lower` | `exact2x2::exact2x2(tsmethod=\"central\")` | OpenEpi shows its own CMLE Fisher-CL solver (MartinStats.js) | default: cmle_or.midp | OpenEpi `MartinStats.js` own Fisher-CL iteration (unmapped "Fisher Limits" column) |
| `calc.two_by_two.crude.cmle_or.fisher.upper` | `exact2x2::exact2x2(tsmethod=\"central\")` | OpenEpi shows its own CMLE Fisher-CL solver (MartinStats.js) | default: cmle_or.midp | OpenEpi `MartinStats.js` own Fisher-CL iteration (unmapped "Fisher Limits" column) |
| `calc.two_by_two.strata[].cmle_or.fisher.lower` | `exact2x2::exact2x2(tsmethod=\"central\")` | OpenEpi shows its own CMLE Fisher-CL solver (MartinStats.js) | default: cmle_or.midp | OpenEpi `MartinStats.js` own Fisher-CL iteration (unmapped "Fisher Limits" column) |
| `calc.two_by_two.strata[].cmle_or.fisher.upper` | `exact2x2::exact2x2(tsmethod=\"central\")` | OpenEpi shows its own CMLE Fisher-CL solver (MartinStats.js) | default: cmle_or.midp | OpenEpi `MartinStats.js` own Fisher-CL iteration (unmapped "Fisher Limits" column) |
| `calc.dose_response.xmh.cc.value` | `fixtures/R/methods/dose_response.R::xmh_trend` — Rosner, Fundamentals of Bio… | OpenEpi shows the corrected extended MH as (Σv1 − 0.5)²/Σv (no |·| on Σv1) | this | OpenEpi `DoseResponse.js` `XMHchisq += ((V1sum − 0.5)²)/Vsum` — subtracts 0.5 without |·|, so the correction can inflate the statistic (V1sum < 0); capture `dose_response/decreasing, dose_response/textbook` |
| `calc.dose_response.xmh.cc.p` | `fixtures/R/methods/dose_response.R::xmh_trend` — Rosner, Fundamentals of Bio… | OpenEpi shows its corrected extended MH's tail via Posten's pchisq approximation | this | OpenEpi `DoseResponse.js` + `pchisq` Posten tail on the corrected XMH; capture `dose_response/textbook` |
| `calc.dose_response.xmh.uncorrected.value` | `fixtures/R/methods/dose_response.R::xmh_trend` — Schlesselman 1982 p.203 | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: xmh.cc | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.dose_response.xmh.uncorrected.p` | `fixtures/R/methods/dose_response.R::xmh_trend` — Schlesselman 1982 p.203 | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: xmh.cc | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.matched_case_control.or.lower` | `fixtures/R/methods/matched.R::matched_or_taylor` — b/c with log CI √(1/b+1/c) | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | this | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `matched_case_control/few_discordant` |
| `calc.matched_case_control.cmle_or.midp.upper` | `exact2x2::exact2x2(paired=TRUE, midp=TRUE)` | OpenEpi shows its own CMLE mid-P solver (MartinStats.js) | this | OpenEpi `MartinStats.js` own CMLE/mid-P iteration; capture `matched_case_control/textbook` |
| `calc.matched_case_control.cmle_or.fisher.lower` | `exact2x2::exact2x2(paired=TRUE, tsmethod=\"central\")` | OpenEpi shows its own CMLE Fisher-CL solver (MartinStats.js) | default: cmle_or.midp | OpenEpi `MartinStats.js` own Fisher-CL iteration (unmapped "Fisher Limits" column) |
| `calc.matched_case_control.cmle_or.fisher.upper` | `exact2x2::exact2x2(paired=TRUE, tsmethod=\"central\")` | OpenEpi shows its own CMLE Fisher-CL solver (MartinStats.js) | default: cmle_or.midp | OpenEpi `MartinStats.js` own Fisher-CL iteration (unmapped "Fisher Limits" column) |
| `calc.mean_ci.z.lower` | `fixtures/R/methods/means.R::mean_ci` — z-CI with √((N−n)/(N−1)) FPC, Cochran… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | default: t | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `mean_ci/se_input, mean_ci/textbook, mean_ci/variance_input` |
| `calc.mean_ci.z.upper` | `fixtures/R/methods/means.R::mean_ci` — z-CI with √((N−n)/(N−1)) FPC, Cochran… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | default: t | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `mean_ci/se_input, mean_ci/textbook, mean_ci/variance_input` |
| `calc.mean_ci.t.lower` | `stats::qt` | OpenEpi shows its own t quantile (AStudT bisection) | this | OpenEpi `StatFunctions1.js` `AStudT` — own t quantile by bisection; capture `mean_ci/se_input, mean_ci/textbook` |
| `calc.median_ci.normal.lower_rank` | `fixtures/R/methods/means.R::percentile_ci` — Conover, Practical Nonparametri… | OpenEpi shows rounded normal-approx ranks (Math.round) | this | OpenEpi `CIMedian.js` `round(np ∓ z√npq)` / `round(1+np+z√npq)` ranks; capture `median_ci/boundary` |
| `calc.median_ci.normal.upper_rank` | `fixtures/R/methods/means.R::percentile_ci` — Conover, Practical Nonparametri… | OpenEpi shows rounded normal-approx ranks (Math.round) | this | OpenEpi `CIMedian.js` `round(np ∓ z√npq)` / `round(1+np+z√npq)` ranks; capture `median_ci/boundary, median_ci/large, median_ci/median, median_ci/p25` |
| `calc.t_test_summary.pooled.lower` | `stats::t.test(var.equal=TRUE)` equivalent | OpenEpi shows its own t quantile (AStudT bisection) | this | OpenEpi `StatFunctions1.js` `AStudT` — own t quantile by bisection; capture `t_test_summary/boundary, t_test_summary/equal_n, t_test_summary/large, t_test_summary/textbook` |
| `calc.t_test_summary.welch.lower` | `stats::t.test(var.equal=FALSE)` | OpenEpi shows its own t quantile at Math.round(Satterthwaite df) (AStudT bisection) | default: pooled | OpenEpi `t_testMean.js` `AStudT` at `Math.round`(Satterthwaite df) — engine uses the fractional df; capture `t_test_summary/boundary, t_test_summary/equal_n, t_test_summary/large, t_test_summary/textbook` |
| `calc.t_test_summary.pooled.upper` | `stats::t.test(var.equal=TRUE)` equivalent | OpenEpi shows its own t quantile (AStudT bisection) | this | OpenEpi `StatFunctions1.js` `AStudT` — own t quantile by bisection; capture `t_test_summary/boundary, t_test_summary/large, t_test_summary/textbook` |
| `calc.t_test_summary.welch.upper` | `stats::t.test(var.equal=FALSE)` | OpenEpi shows its own t quantile at Math.round(Satterthwaite df) (AStudT bisection) | default: pooled | OpenEpi `t_testMean.js` `AStudT` at `Math.round`(Satterthwaite df) — engine uses the fractional df; capture `t_test_summary/boundary, t_test_summary/equal_n, t_test_summary/large` |
| `calc.t_test_summary.welch.p_2t` | `stats::t.test(var.equal=FALSE)` | OpenEpi shows its own t quantile at Math.round(Satterthwaite df) (AStudT bisection) | default: pooled | OpenEpi `t_testMean.js` `AStudT` at `Math.round`(Satterthwaite df) — engine uses the fractional df; capture `t_test_summary/boundary` |
| `calc.anova_summary.anova.p` | `fixtures/R/methods/means.R::anova_from_summary` — ANOVA identities from n/me… | OpenEpi shows its own F tail (FishF) | this | OpenEpi `StatFunctions1.js` `FishF` — own F-distribution tail; capture `anova_summary/large` |
| `calc.proportion.ci.wilson.lower` | `PropCIs::scoreci` | OpenEpi shows a Wilson score CI at fixed z = 1.96 (z² = 3.8416) | this | OpenEpi `StatFunctions1.js` `proportion95CL(…, "Wilson")` — fixed z = 1.96, z² = 3.8416, conf ignored; capture `proportion/textbook` |
| `calc.proportion.ci.wilson.upper` | `PropCIs::scoreci` | OpenEpi shows a Wilson score CI at fixed z = 1.96 (z² = 3.8416) | this | OpenEpi `StatFunctions1.js` `proportion95CL(…, "Wilson")` — fixed z = 1.96, z² = 3.8416, conf ignored; capture `proportion/zero_numerator` |
| `calc.proportion.ci.score_cc.lower` | `fixtures/R/methods/proportion.R::score_cc_ci` — Fleiss, Statistical Methods … | OpenEpi shows the Fleiss score-CC CI at fixed z = 1.96 (z² = 3.8416) | default: ci.wilson | OpenEpi `StatFunctions1.js` `proportion95CL(…, "Fleiss")` — fixed z = 1.96, z² = 3.8416; capture `proportion/textbook` |
| `calc.proportion.ci.midp.lower` | `exactci::binom.exact(midp=TRUE)` | OpenEpi shows mid-P bounds by own-CDF bisection (~1e-5 steps) | default: ci.wilson | OpenEpi `PersonTime1.js`/`StatFunctions1.js` mid-P bounds by own-CDF bisection ~1e-5; capture `proportion/all_positive` |
| `calc.proportion.ci.midp.upper` | `exactci::binom.exact(midp=TRUE)` | OpenEpi shows mid-P bounds by own-CDF bisection (~1e-5 steps) | default: ci.wilson | OpenEpi `PersonTime1.js`/`StatFunctions1.js` mid-P bounds by own-CDF bisection ~1e-5; capture `proportion/zero_numerator` |
| `calc.rate_one.ci.midp.lower` | `exactci::poisson.exact(midp=TRUE)` | OpenEpi shows mid-P bounds by own-CDF bisection (~1e-5 steps) | this | OpenEpi `PersonTime1.js`/`StatFunctions1.js` mid-P bounds by own-CDF bisection ~1e-5; capture `rate_one/textbook` |
| `calc.rate_one.ci.byar.lower` | `epitools::pois.byar` scaled | OpenEpi shows the classic Byar — x for the lower bound, x+1 for the upper — z = √cscrit | default: ci.midp | OpenEpi `PersonTime1.js`/`SMR.js` — Byar with x lower / x+1 upper, z = √cscrit (engine follows `epitools::pois.byar`, x+0.5 both); capture `rate_one/boundary, rate_one/large, rate_one/textbook` |
| `calc.rate_one.ci.normal.lower` | `epitools::pois.approx` scaled | OpenEpi shows a Wald bound (x ± z·√x)/PT clamped at 0, z = √cscrit | default: ci.midp | OpenEpi `PersonTime1.js` normal approx (x ± √cscrit·√x)/PT clamped ≥ 0; capture `rate_one/boundary` |
| `calc.rate_one.ci.rothman_greenland.lower` | `fixtures/R/methods/poisson.R::rg_ci` — Rothman & Greenland, Modern Epidemiol… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | default: ci.midp | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `rate_one/boundary` |
| `calc.rate_one.ci.midp.upper` | `exactci::poisson.exact(midp=TRUE)` | OpenEpi shows mid-P bounds by own-CDF bisection (~1e-5 steps) | this | OpenEpi `PersonTime1.js`/`StatFunctions1.js` mid-P bounds by own-CDF bisection ~1e-5; capture `rate_one/boundary` |
| `calc.rate_one.ci.byar.upper` | `epitools::pois.byar` scaled | OpenEpi shows the classic Byar — x for the lower bound, x+1 for the upper — z = √cscrit | default: ci.midp | OpenEpi `PersonTime1.js`/`SMR.js` — Byar with x lower / x+1 upper, z = √cscrit (engine follows `epitools::pois.byar`, x+0.5 both); capture `rate_one/boundary, rate_one/textbook` |
| `calc.rate_one.ci.normal.upper` | `epitools::pois.approx` scaled | OpenEpi shows a Wald bound (x ± z·√x)/PT clamped at 0, z = √cscrit | default: ci.midp | OpenEpi `PersonTime1.js` normal approx (x ± √cscrit·√x)/PT clamped ≥ 0; capture `rate_one/boundary` |
| `calc.rate_one.ci.rothman_greenland.upper` | `fixtures/R/methods/poisson.R::rg_ci` — Rothman & Greenland, Modern Epidemiol… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | default: ci.midp | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `rate_one/textbook` |
| `calc.smr.ci.byar.lower` | `epitools::pois.byar` scaled | OpenEpi shows the classic Byar — x for the lower bound, x+1 for the upper — z = √cscrit | default: ci.midp | OpenEpi `PersonTime1.js`/`SMR.js` — Byar with x lower / x+1 upper, z = √cscrit (engine follows `epitools::pois.byar`, x+0.5 both); capture `smr/boundary_low, smr/large, smr/textbook` |
| `calc.smr.ci.byar.upper` | `epitools::pois.byar` scaled | OpenEpi shows the classic Byar — x for the lower bound, x+1 for the upper — z = √cscrit | default: ci.midp | OpenEpi `PersonTime1.js`/`SMR.js` — Byar with x lower / x+1 upper, z = √cscrit (engine follows `epitools::pois.byar`, x+0.5 both); capture `smr/boundary_low, smr/large, smr/textbook` |
| `calc.smr.ci.rothman_greenland.upper` | `fixtures/R/methods/poisson.R::rg_ci` — Rothman & Greenland, Modern Epidemiol… | OpenEpi shows the same formula with z = √cscrit (hard-coded chi-square criticals) | default: ci.midp | OpenEpi `StatFunctions1.js` `cscritFromOEConfLevel` — hard-coded χ² criticals (95% → z = √3.841 ≈ 1.95996 vs `qnorm(0.975)`); capture `smr/textbook` |
| `calc.smr.ci.ury_wiggins.lower` | `fixtures/R/methods/poisson.R::ury_wiggins` — Am J Epidemiol 1985;122:197-8 | OpenEpi shows Ury–Wiggins bounds with hard-coded per-conf offsets, z = √cscrit | default: ci.midp | OpenEpi `SMR.js` Ury–Wiggins hard-coded per-conf offsets, z = √cscrit; capture `smr/boundary_low` |
| `calc.screening.lr_pos.lower` | fixtures/R/calc.R | OpenEpi shows bounds exp(ln LR ± 1.96·Σ(1/nᵢ)) — no √ on the SE sum | this | OpenEpi `DiagnosticTest.js` LR bounds `exp(ln LR ± 1.96·Σ(1/nᵢ))` — no √ on the SE sum; capture `screening/textbook` |
| `calc.screening.lr_neg.lower` | fixtures/R/calc.R | OpenEpi shows bounds exp(ln LR ± 1.96·Σ(1/nᵢ)) — no √ on the SE sum | this | OpenEpi `DiagnosticTest.js` LR bounds `exp(ln LR ± 1.96·Σ(1/nᵢ))` — no √ on the SE sum; capture `screening/boundary, screening/textbook` |
| `calc.screening.level_lr[].lower` | `fixtures/R/methods/screening.R::level_lr` — log-method LR CI as in `Diagnost… | OpenEpi shows level-LR bounds with the level's own 2×2 counts, not column totals | this | OpenEpi `DiagnosticTest.js` multi-level LR uses the level's 2×2 counts |
| `calc.screening.lr_pos.upper` | fixtures/R/calc.R | OpenEpi shows bounds exp(ln LR ± 1.96·Σ(1/nᵢ)) — no √ on the SE sum | this | OpenEpi `DiagnosticTest.js` LR bounds `exp(ln LR ± 1.96·Σ(1/nᵢ))` — no √ on the SE sum; capture `screening/textbook` |
| `calc.screening.lr_neg.upper` | fixtures/R/calc.R | OpenEpi shows bounds exp(ln LR ± 1.96·Σ(1/nᵢ)) — no √ on the SE sum | this | OpenEpi `DiagnosticTest.js` LR bounds `exp(ln LR ± 1.96·Σ(1/nᵢ))` — no √ on the SE sum; capture `screening/boundary, screening/textbook` |
| `calc.screening.level_lr[].upper` | `fixtures/R/methods/screening.R::level_lr` — log-method LR CI as in `Diagnost… | OpenEpi shows level-LR bounds with the level's own 2×2 counts, not column totals | this | OpenEpi `DiagnosticTest.js` multi-level LR uses the level's 2×2 counts |
| `calc.screening.ppv.lower` | `PropCIs::scoreci` | OpenEpi shows a Wilson score CI at fixed z = 1.96 (z² = 3.8416) | this | OpenEpi `StatFunctions1.js` `proportion95CL(…, "Wilson")` — fixed z = 1.96, z² = 3.8416, conf ignored; capture `screening/boundary` |
| `calc.ss_cohort.kelsey.exposed` | `fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc` | OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles | this | OpenEpi `SSCohort.js` Kelsey/Fleiss via own `ANorm`/`Norm` quantiles; capture `ss_cohort/percent_input, ss_cohort/risk_difference_input, ss_cohort/risk_ratio_input` |
| `calc.ss_cohort.fleiss.exposed` | `fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc` | OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles | default: kelsey | OpenEpi `SSCohort.js` Kelsey/Fleiss via own `ANorm`/`Norm` quantiles; capture `ss_cohort/percent_input, ss_cohort/risk_difference_input, ss_cohort/risk_ratio_input` |
| `calc.ss_cohort.fleiss_cc.exposed` | `fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc` | OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles | default: kelsey | OpenEpi `SSCohort.js` Kelsey/Fleiss via own `ANorm`/`Norm` quantiles; capture `ss_cohort/percent_input, ss_cohort/risk_difference_input, ss_cohort/risk_ratio_input` |
| `calc.ss_cohort.kelsey.unexposed` | `fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc` | OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles | this | OpenEpi `SSCohort.js` Kelsey/Fleiss via own `ANorm`/`Norm` quantiles; capture `ss_cohort/percent_input, ss_cohort/risk_difference_input, ss_cohort/risk_ratio_input` |
| `calc.ss_cohort.fleiss.unexposed` | `fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc` | OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles | default: kelsey | OpenEpi `SSCohort.js` Kelsey/Fleiss via own `ANorm`/`Norm` quantiles; capture `ss_cohort/percent_input, ss_cohort/risk_difference_input, ss_cohort/risk_ratio_input` |
| `calc.ss_cohort.fleiss_cc.unexposed` | `fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc` | OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles | default: kelsey | OpenEpi `SSCohort.js` Kelsey/Fleiss via own `ANorm`/`Norm` quantiles; capture `ss_cohort/percent_input, ss_cohort/risk_difference_input, ss_cohort/risk_ratio_input` |
| `calc.ss_cohort.kelsey.total` | `fixtures/R/methods/sample_size.R::kelsey` — same source as `ss_unmatched_cc` | OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles | this | OpenEpi `SSCohort.js` Kelsey/Fleiss via own `ANorm`/`Norm` quantiles; capture `ss_cohort/percent_input, ss_cohort/risk_difference_input, ss_cohort/risk_ratio_input` |
| `calc.ss_cohort.fleiss.total` | `fixtures/R/methods/sample_size.R::fleiss` — same source as `ss_unmatched_cc` | OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles | default: kelsey | OpenEpi `SSCohort.js` Kelsey/Fleiss via own `ANorm`/`Norm` quantiles; capture `ss_cohort/percent_input, ss_cohort/risk_difference_input, ss_cohort/risk_ratio_input` |
| `calc.ss_cohort.fleiss_cc.total` | `fixtures/R/methods/sample_size.R::fleiss_cc` — same source as `ss_unmatched_cc` | OpenEpi shows the same Kelsey/Fleiss formula via own ANorm/Norm quantiles | default: kelsey | OpenEpi `SSCohort.js` Kelsey/Fleiss via own `ANorm`/`Norm` quantiles; capture `ss_cohort/percent_input, ss_cohort/risk_difference_input, ss_cohort/risk_ratio_input` |
| `calc.ss_mean_difference.n1` | `fixtures/R/methods/sample_size.R::ss_means` — OpenEpi `SSMean.js` formula | OpenEpi shows n₁ = ⌈(s₁²+s₂²/r)(z_α+z_β)²/Δ²⌉ — a z formula, not power.t.test | this | OpenEpi `SSMean.js` `n₁ = ⌈(s₁²+s₂²/r)(z_α+z_β)²/Δ²⌉` — z formula, not `power.t.test`; capture `ss_mean_difference/high_power, ss_mean_difference/small_effect` |
| `calc.ss_mean_difference.n2` | `fixtures/R/methods/sample_size.R::ss_means` — OpenEpi `SSMean.js` formula | OpenEpi shows n₁ = ⌈(s₁²+s₂²/r)(z_α+z_β)²/Δ²⌉ — a z formula, not power.t.test | this | OpenEpi `SSMean.js` `n₁ = ⌈(s₁²+s₂²/r)(z_α+z_β)²/Δ²⌉` — z formula, not `power.t.test`; capture `ss_mean_difference/high_power, ss_mean_difference/small_effect` |
| `calc.ss_mean_difference.total` | `fixtures/R/methods/sample_size.R::ss_means` — OpenEpi `SSMean.js` formula | OpenEpi shows n₁ = ⌈(s₁²+s₂²/r)(z_α+z_β)²/Δ²⌉ — a z formula, not power.t.test | this | OpenEpi `SSMean.js` `n₁ = ⌈(s₁²+s₂²/r)(z_α+z_β)²/Δ²⌉` — z formula, not `power.t.test`; capture `ss_mean_difference/high_power, ss_mean_difference/small_effect` |
| `calc.power_unmatched_cc.normal` | `fixtures/R/methods/sample_size.R::power_cc` — OpenEpi `PowerCC.js` (Fleiss §… | OpenEpi shows the same normal-approx power formula via own Norm/ANorm and z = √cscrit | this | OpenEpi `PowerCC.js`/`PowerCohort.js` — own `Norm`/`ANorm` approximations + z = √cscrit; capture `power_unmatched_cc/small_effect` |
| `calc.power_unmatched_cc.normal_cc` | `fixtures/R/methods/sample_size.R::power_cc_cc` — same | OpenEpi shows the same normal-approx power formula via own Norm/ANorm and z = √cscrit | default: normal | OpenEpi `PowerCC.js`/`PowerCohort.js` — own `Norm`/`ANorm` approximations + z = √cscrit; capture `power_unmatched_cc/small_effect, power_unmatched_cc/textbook` |
| `calc.power_cohort.normal` | `fixtures/R/methods/sample_size.R::power_cohort` — OpenEpi `PowerCohort.js` (… | OpenEpi shows the same normal-approx power formula via own Norm/ANorm and z = √cscrit | this | OpenEpi `PowerCC.js`/`PowerCohort.js` — own `Norm`/`ANorm` approximations + z = √cscrit; capture `power_cohort/cohort` |
| `calc.power_cohort.normal_cc` | `fixtures/R/methods/sample_size.R::power_cohort_cc` — same source | OpenEpi shows the same normal-approx power formula via own Norm/ANorm and z = √cscrit | default: normal | OpenEpi `PowerCC.js`/`PowerCohort.js` — own `Norm`/`ANorm` approximations + z = √cscrit; capture `power_cohort/cross_sectional` |
| `calc.power_mean_difference.power` | `fixtures/R/methods/sample_size.R::power_means` — OpenEpi `PowerMean.js` z-fo… | OpenEpi shows a z-power 1 − Norm(−z + √n₁·Δ/√(s₁²+s₂²/r)) (own Norm), not power.t.test | this | OpenEpi `PowerMean.js` `power = 1 − Norm(−z + √n₁·Δ/√(s₁²+s₂²/r))` — own Norm, not `power.t.test`; capture `power_mean_difference/small_effect, power_mean_difference/textbook` |
| `calc.ss_proportion.n_openepi.80` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPr… | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi_real.80` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi` | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7.80` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `Populati… | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7_real.80` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.total.80` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size.80` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size_real.80` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi.90` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPr… | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi_real.90` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi` | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7.90` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `Populati… | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7_real.90` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.total.90` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size.90` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size_real.90` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi.95` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPr… | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi_real.95` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi` | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7.95` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `Populati… | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7_real.95` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.total.95` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size.95` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size_real.95` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi.97` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPr… | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi_real.97` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi` | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7.97` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `Populati… | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7_real.97` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.total.97` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size.97` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size_real.97` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi.99` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPr… | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi_real.99` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi` | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7.99` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `Populati… | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7_real.99` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.total.99` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size.99` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size_real.99` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi.999` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPr… | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi_real.999` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi` | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7.999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `Populati… | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7_real.999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.total.999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size.999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size_real.999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi.9999` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — as implemented in `SSPr… | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_openepi_real.9999` | `fixtures/R/methods/sample_size.R::ss_prop_openepi` — same source as `n_openepi` | OpenEpi's own formula (`SSPropor.js`) — engine emits it verbatim (not OpenEpi-mapped) | this | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7.9999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — as implemented in `Populati… | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.n_ei7_real.9999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.total.9999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size.9999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `n_ei7` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |
| `calc.ss_proportion.cluster_size_real.9999` | `fixtures/R/methods/sample_size.R::ss_prop_ei7` — same source as `cluster_size` | Epi Info 7 StatCalc (`PopulationSurvey.xaml.cs` @ 4cd353c); not OpenEpi-mapped | default: n_openepi | not OpenEpi-mapped; Task 9 variant pair — EI7 source `PopulationSurvey.xaml.cs` @ 4cd353c, OpenEpi `SSPropor.js` |

- Files outside the task's Files block:
  - `crates/epiora-core/src/corpus/openepi.rs` — the
    documented-difference classification: `OE_METHOD_MARK` label
    lookup against `calc::definition`, the `method_differences`
    report bucket, and a `normalize_key` fix (a numeric path segment
    folds into the previous one as `[]` — the old code produced
    `strata.[].x`, which never matched a `strata[].x` registry id, so
    `covered_statistics` silently covered nothing). The method
    choices can only take effect in the comparison layer: the plan
    forbids changing fixtures, tolerances, decision rules and result
    values, and the mapping itself is baked into the capture files —
    the same position the v0.2 Task 9 documented-rescale keys occupy
    in `corpus/ei7.rs`.
  - `crates/epiora-core/src/corpus/report/openepi.rs` — reports the
    `method_differences` count and list in the `## OpenEpi check`
    section and reuses the shared `normalize_key`.
  - `crates/epiora-core/src/validation/registry/calc*.rs` — not
    changed: `Coverage` rows carry only `statistic`/`fixtures`/`class`
    (no label field), so the Files block's `calc.rs (labels)` has no
    surface to edit; all method labels live in `calc/**` `OutputDef`s.

### v1.0 Task 14 — calculators from a link in a store-less worker (2026-11-18)

Decision: `/calc` and `/calc/$id` render the registry generically in
the existing app shell; the query string is a plain string map
(`?strata=10,11;4,20`), decoded into raw-text drafts so a malformed
link marks inputs instead of blanking. A complete link computes on
load; calculate pushes the inputs back into the URL. `random_numbers`
without `seed` draws one inside the calc worker via
`crypto.getRandomValues` (53 bits) and writes it into the URL with
replace. The calc worker (`calc.worker.ts`) loads core-wasm only —
no store, no import-wasm, no Web Lock — and stamps
`RunContext.timestamp`; the app's data worker now spawns lazily so a
calculator tab never acquires `epiora-store`.

- Files outside the task's Files block:
  - `app/src/worker/client.ts` — `dataWorker`/`dataApi` were created
    at module load, so importing the module (every screen does, calc
    routes included through the statically imported router tree)
    spawned the worker and took the `epiora-store` Web Lock on
    `/calc`. `dataApi` is now a `Proxy` that spawns the worker on
    first method call (writes shadow it — the e2e
    `window.dataApi.<method>` patches still work) and the Tauri
    bridge attaches inside `dataWorker()` at spawn; `workerName` and
    the tab-lock BroadcastChannel contract are unchanged.
- `app/vite.config.ts` (listed under Modify) is unchanged: Vite
  already emits `new Worker` entries as `assets/calc.worker-*.js`,
  which the unchanged precache globs cover — verified by
  `calc-offline.spec.ts`'s dead-server reload.
- `app/src/main.tsx`: the Tauri `listProjects` startup probe is
  skipped on `/calc` (it would otherwise spawn the store worker via
  the lazy path); the tab-lock channel listener stays unconditional
  because its `workerName` tag makes calc routes a no-op.
- `app/src/ui/screens/calc/calc.module.css` — the Files block lists a
  `.module.css` per calc screen component; the four components share
  one module (`calc.module.css`) because their classes are the shared
  form/table/result layout, and per-component modules would duplicate
  the same token references. No per-component CSS exists.
- `app/scripts/budget.mjs` and `app/scripts/budget.test.mjs` — outside
  the Files block. Step 4 / spec §10 requires the `/calc/$id` first
  load reported in the job summary (not gated); the only reporter is
  the existing budget script, which now prints a second table over
  index.html, the entry JS/CSS, the core .wasm and the manifest's
  calc route/`calc.worker` chunks. The 5,000,000 gate and the exit
  code are unchanged; the test gained a case asserting both.
Supersedes: —
### v1.0 Task 15 — MEANS dialog and Epi Info command line (2026-11-18)

Decision: every result card shows its Epi Info command with a Copy
button. `runs.command` (store schema v2, `ALTER TABLE runs ADD COLUMN
command TEXT`, nullable) is stored at insert for new runs; rows written
before the migration hold NULL and `toRunRecord` generates the command
on load with the worker's `epiInfoCommand` — never written back. MEANS
joins `run()`'s freq/tables dispatch, `RunRecord`'s unions and the
procedure menu; `renderMeans` renders one descriptives table (a row per
group, then overall) plus each test block the engine returned.

- Files outside the task's Files block:
  - `app/src/store/opfs.ts`, `app/src/store/relay.ts`,
    `app/src/store/stdio.ts` — the three backends each hold a
    `MIGRATIONS` list that must name `0002_run_command.sql` or the new
    column never materialises (the plan lists the migration file
    itself; the loaders are the mechanical wiring).
  - `app/src/ui/routes/analysis.tsx` — the screen's dialog state and
    `makeRun` dispatch gain `'means'` → `dataApi.runMeans`; the plan's
    Modify list names the analysis components but not the route that
    wires them.
  - `app/src/worker/client.ts` — `wrapDataApi` mirrors every `DataApi`
    method explicitly; `runMeans` and the test hook below need the
    main-thread twin `api.ts` defines.
  - `app/src/ui/screens/analysis/render-means.tsx` (fix round 1) —
    `renderMeans` moved here so `render-result.tsx` stays under the
    ~400-line global constraint; `render-result.tsx` re-exports it, so
    `render-result.test.tsx` and `ResultCard.tsx` are unchanged.
- `DataApi.__clearRunCommand` (VITE_TEST_HOOKS=1 only): the plan's e2e
  `old_run_gets_generated_command` needs a `runs` row with NULL
  `command`, which no public write path creates — `run()` always stores
  a command now. The hook nulls the column on a stored run, following
  the `__simulateQuotaOnNextWrite` / `__clearSnapshotCache` pattern.
Supersedes: —
### v1.0 Task 16 — Explain panel, MathML templates, formula guard (2026-11-19)

Decision: one reusable `ExplainPanel` serves both surfaces. Every
Analysis result row carries the concrete `explainProcedure` paths of
the leaves it displays (estimate first, then CI bounds and p-value);
when a row covers more than one leaf the panel's Select switches the
traced statistic — labels stay digit-free so the "every number comes
from the trace" rule holds. Words come first, "Show the maths" reveals
the template's MathML with the trace's formatted values substituted,
and `method.*` traces render words plus the inputs/result table only —
no substituted mathematics.

- Files outside the task's Files block:
  - `crates/epiora-core/src/explain/freq.rs` — engine trace fix the
    guard test made visible: the trace passed `table.total` (records
    in scope, including missing) as the `n` denominator for
    `percent`, `cumulative_percent` and both per-category CIs, but
    the procedure divides by `nf = Σ counts`. The formula could not
    reproduce the leaf it claims to explain; `n` is now the summed
    category counts. No leaf values change — only trace inputs.
  - `scripts/check-app.sh` — the guard test needs the Node parity
    build (`pkg-parity`, target `nodejs`, `parity` feature); the app
    gate now builds it next to the release packages so Vitest finds
    it in CI and locally.
  - `app/src/worker/client.ts` — `wrapDataApi` mirrors every
    `DataApi` method explicitly; `explainRun` needs its main-thread
    twin (same pattern as `rerun`/`deleteRun`).
  - `app/src/ui/screens/analysis/ResultCard.tsx` — the plan's Modify
    list names `render-result.tsx` but not the card that owns the
    dialog state and calls `dataApi.explainRun`.
  - `app/src/ui/screens/analysis/render-means.tsx` — `renderMeans`
    lives in its own module (v1.0 Task 15); its descriptives and test
    rows get the same `stats` mapping `render-result.tsx` gained.
  - `app/src/ui/format.ts` — `formatNumber` moved out of
    `render-result.tsx` into a leaf module with no React or stylesheet
    imports: result rows now import the Epiora `Button`, and the e2e
    specs import `formatNumber`, so keeping it in `render-result.tsx`
    pulled a `.module.css` into Playwright's transform (which cannot
    load CSS). Callers updated: `render-result.tsx`,
    `render-result.test.tsx`,
    `CalculatorResult.tsx`, `ExplainPanel.tsx`,
    `e2e/analysis.spec.ts`, `e2e/explain.spec.ts`.
  - Fix-round split under the ~400-line rule (AGENTS.md rule 8) —
    pure move; no behaviour, template id, string or number changed:
    - `app/src/ui/screens/analysis/stats-table.tsx` — `StatsTable`,
      `StatRow`/`ExplainHandler`, `stat`, `ciStats`, `estimate` and
      `interval` moved out of `render-result.tsx`, which re-exports
      the public pieces so callers are unchanged.
    - `app/src/ui/screens/analysis/render-freq.tsx` — the FREQ table
      view and `renderFreq` (same sibling-module precedent as
      `render-means.tsx`).
    - `app/src/explain/templates/chiSquarePooled.ts` — the pooled
      and homogeneity χ² templates; `chiSquare.ts` keeps the
      single-table and paired forms and exports `CHI`, `SQ`, `MH`.
    - `app/src/explain/templates/personTimeAdjusted.ts` — the
      directly adjusted and pooled Mantel–Haenszel person-time
      templates (with `directLn`/`rrLn`); `personTime.ts` exports
      `RB`.
    - `app/src/explain/templates/binomial.ts` — the one-sample z
      test and binomial tail templates; `proportions.ts` keeps the
      interval bounds. `app/src/explain/index.ts` merges
      `CHI_SQUARE_POOLED`, `PERSON_TIME_ADJUSTED` and `BINOMIAL`.
- `freq.*` statistic paths name no table, so the engine resolves them
  against the run's first table only; `renderFreq` therefore offers
  the explain affordance on `tables[0]`'s rows only (later tables get
  no explain button rather than a wrong trace).
Supersedes: —

### Conductor process — review/task gate trimming (2026-11-18, owner-authorised in PR #91)

Decision: the owner approved the conductor's recommendations on PR #91
("do all your recos in the same PR"). This is a process change only — no
app product code, fixture, tolerance, budget or Task 16 template/panel
change.

- Review prompt: CI is the gate. `gh pr checks` green (perf skipped on
  PRs is by design) means the reviewer does not re-run
  `./scripts/check.sh` or the full Playwright suite, does not fight
  sandbox limits, reviews the diff against the plan and runs only
  targeted tests for something it doubts; red/pending CI means review
  the diff only. Escape hatch: the full gate runs only when the
  conductor's extra instructions ask for it. Files:
  `.devin/skills/epiora-review/SKILL.md` (step 5 replaced — it told the
  reviewer to run `./scripts/check.sh`) and
  `scripts/conductor/build-prompt.sh` (review mode prints the same
  paragraph).
- Task prompt: iterate on targeted tests (the task's named tests, tsc,
  lint, the relevant vitest/`cargo -p` test); run the full
  `./scripts/check.sh` at most once, immediately before the push; never
  loop on the whole three-browser Playwright suite locally — CI re-runs
  it. Files: `.devin/skills/epiora-task/SKILL.md` (step 8 — one rule,
  replacing the old iterate-with-quick wording) and
  `scripts/conductor/build-prompt.sh` (task mode prints it).
- `line-count` judge check: flags an added or grown source file
  provably over ~400 lines post-change (`*.md`, `*.json`, lockfiles,
  `fixtures/**`, generated files and snapshots exempt; a file already
  over on main that did not grow is exempt — `run_case.rs` and
  `special/gamma.rs` follow the same rule). A diff does not carry a
  modified file's total size, so the check uses the provable bound
  `max(last shown new-side line, last shown old-side end + net growth)`
  from the hunk headers — it catches every file the diff proves is past
  the limit and cannot prove (never guesses) a mid-file-only growth.
  `checkLineCount` lives in `scripts/conductor/judge-checks-gates.ts`
  (placement deviation from the fix instructions: `judge-checks.ts`
  would exceed the 400-line rule the check enforces). Also in
  `judge-checks.ts`: `files-block` now resolves a
  `` `X.tsx` + `.module.css` `` Files-block entry to `X.module.css`, and
  a `docs/decisions.md` mention by unique path suffix
  (`e2e/analysis.spec.ts` for `app/e2e/analysis.spec.ts`) counts as
  named — both were false FLAGs on this PR. Files:
  `scripts/conductor/judge-checks.ts`, `scripts/conductor/judge.ts`,
  `scripts/conductor/judge.test.ts` (423 lines — the new cases were
  required in this file; the check's bound does not reach it because no
  hunk lands past line 400), `scripts/conductor/fixtures/judge/plan.md`
  (fixture Task 4), `scripts/conductor/fixtures/judge/line-count-new-file.diff`,
  `scripts/conductor/fixtures/judge/line-count-grew-past.diff`,
  `scripts/conductor/fixtures/judge/line-count-long-nogrow.diff`,
  `scripts/conductor/fixtures/judge/line-count-exempt.diff`,
  `scripts/conductor/fixtures/judge/files-block-plus-css.diff`,
  `scripts/conductor/fixtures/judge/decisions-suffix.diff`,
  `docs/superpowers/plans/2026-11-17-epiora-v1.0-conductor-efficiency.md`
  (`line-count` added as check 6, list renumbered — it enumerates check
  ids).
- Sharding the `app` job was tried in this PR and withdrawn by the owner
  (shards failed on the deploy preview; no wall-time gain because
  `app-gate` precedes the shards); it will be its own PR.
- Found, not fixed: `app/playwright.config.ts:29` uses
  `devices['Desktop WebKit']`, which does not exist (key is `Desktop
  Safari`), so the webkit project runs chromium — to be handled by its
  own task.
- Task sizing note in
  `docs/superpowers/plans/2026-11-12-epiora-v1.0-calculators-and-means.md`
  (a UI task over ~12 files, or pairing a new component tree with a new
  three-engine e2e spec, is split into logic+unit tests and UI+e2e; no
  existing task split). And `docs/decisions.md` (this entry).
Supersedes: —

### v1.0 Task 17 — R export: verbatim corpus R, zip download, CI (2026-11-19)

Decision: the exported `analysis.R` does not re-implement the
statistics — it embeds the corpus's own MIT-licensed reference R and
re-runs it. `fixtures/R/methods/*.R` ship verbatim whole-file (SPDX
headers and top-level constants like `ss_levels` ride along), and the
`*_write` drivers from `calc.R`, `twobytwo.R`, `stratified.R`,
`rxc.R`, `freq.R`, `means.R`, `special.R`, `common.R` and `nchyper.R`
are extracted by `name <- function` signature with a transitive
identifier closure. The generator entry points `write_fixture` /
`calc_write` / `pkg_versions` / `write_dataset` are shimmed to print
`EPIORA <path> <value %.17g>` per leaf — `expected.json` and the
script's stdout therefore share the corpus's path scheme and E7
classes, and `scripts/r-export/compare.mjs` checks every line
(`closed_form` 1e-9, `iterative` 1e-6). `random_numbers` refuses with
`export_not_available`: its draws come from a seeded ChaCha stream R
cannot replay. Dataset runs embed a deterministic `data.csv` whose
missing markers are disambiguated per column against real text
values; the corpus's `hostile_text` case round-trips every cell
exactly. Local verification: `export-corpus` wrote 138 cases, all
scripts ran under the renv-pinned R 4.6.2, `compare.mjs` verified
8,879 statistics.

- Files outside the task's Files block:
  - `crates/epiora-core/src/error.rs` — `EngineError::ExportNotAvailable`
    and its stable `export_not_available` code (spec §8's "every
    failure is an `EngineError`"; the plan's `random_numbers` rule).
  - `crates/epiora-core/src/export/mod.rs` — pre-existing Task 11
    module; the plan's Create list names `csv.rs`/`r_script.rs`/
    `r_snippets.rs` but module wiring lands here.
  - `crates/epiora-core/src/export/r_emit.rs` — shared emit helpers
    (the `epi_walk` prelude, JSON/matrix/table literals, the CSV
    reader prelude, the `DATASETS` env builder, the `epitools` attach
    detector).
  - `crates/epiora-core/src/export/r_calc.rs` — the per-calculator
    `<writer>(id, …)` dispatch `calc_call` runs (the Files block's
    `r_script.rs` holds the public API; a single file would be ~800
    lines, past the ~400-line rule).
  - `crates/epiora-core/src/export/r_proc.rs` — the dataset drivers
    (`tables`/`freq`/`means`) both the app export and the corpus
    cases share.
  - `crates/epiora-core/src/export/corpus_export.rs` — the
    `corpus`-feature half of `export-corpus` (case drivers, expected
    flattening); the bin stays a thin CLI.
  - `crates/epiora-core/src/export/corpus_datasets.rs` — the built-in
    `hostile_text` dataset and the dataset-echo case builder, split
    out of `corpus_export.rs` for the ~400-line rule.
  - `crates/epiora-core/src/tabulate.rs` — `categories` is now
    `pub(crate)` so the TABLES driver's R literals reuse the engine's
    level ordering (Yes-first, {0,1}→"1,0") instead of re-implementing
    it.
  - `app/package-lock.json` — mechanical `npm install fflate@0.8.3`
    output (fflate is MIT; plan's Tech Stack).
  - `app/src/worker/client.ts` — `wrapDataApi` mirrors every
    `DataApi` method explicitly; `exportRun` needs its main-thread
    twin.
  - `app/src/worker/errors.ts` — the app's engine-error-code mirror
    gains `export_not_available`; the contract test requires every
    `EngineError::code()` to appear here and in `ui/strings.ts`.
  - `app/src/calc/calc.worker.ts` — the `exportR` endpoint the Files
    block's `app/src/calc/api.ts` declares (the worker is the wasm's
    owner).
  - `app/src/calc/client.ts` — same Comlink twin as `worker/client.ts`.
  - `app/src/calc/strings.ts` — `exportR`/`exportRFailed` labels; the
    calculator UI's strings live here, not in `app/src/ui/strings.ts`.
  - `app/src/ui/routes/calc.tsx` — `CalculatorResult` needs the run's
    `receipt` for `receipt.json` and the zip filename; the route owns
    the `CalcOutput` and now passes it down.
  - `scripts/tests/float-ban.bats` (fix round 1) — `setup()` now also
    copies `fixtures/R/` into `$BATS_TEST_TMPDIR`: `r_snippets.rs`
    `include_str!`s `fixtures/R/*.R` and `fixtures/R/methods/*.R`, so
    clippy in the copied workspace could not compile epiora-core
    without them.
Supersedes: —
### v1.0 Task 18 — means_1m measurement (2026-11-19)

Decision: `perf.spec.ts` (702 lines, past the ~400-line rule the plan's
split targets) is split by scenario under `app/perf/scenarios/`. The
spec keeps the `perfPage` fixture, the per-sample loop and the owner
gate test; every other line moved verbatim — `shared.ts` holds the
environment knobs, the generated-CSV fixtures, `record()` /
`runScenario()` / `skipScenario()` and the page-side failure
diagnostics behind a `SampleCtx`; `store.ts` (`empty_store_opfs`,
`create_project`), `import.ts` (`import_*`, `reimport_*`),
`tables.ts` (`open_tables_*`) and `grid.ts` (both scrolls plus
`scrollGrid`/`waitForGrid`) hold the scenario bodies. Recorded line
order, skip reasons, `scen()` wrapping and the D7 `gatedOnly`
conditionals are unchanged.

`means_1m` (spec §10, plan: "time from `runMeans` call to resolve on
the existing 1M-row perf dataset for `MEANS <Integer age column>
<3-level Text column>`, after clearing the worker's handle LRU") runs
after `open_tables_*`: `variable: 'age'`, `group_by: 'grp'`,
`include_missing: false` (`grp` has no missing values, so the flag is
inert; `false` matches the TABLES options). It is measured only —
`gate-eval.ts`'s `GATES` gains the owner's number in Task 19 — and it
does not run in the D7 throttled pass, which the job comment scopes
to the two gated scenarios.

- Files outside the task's Files block:
  - `app/perf/scenarios/shared.ts`, `store.ts`, `import.ts`,
    `tables.ts`, `grid.ts` — the Files block's `perf.spec.ts`
    parenthetical requires the split into `app/perf/scenarios/*.ts`;
    these files are the existing scenarios and plumbing moved
    verbatim. `means.ts` (the listed Create) holds the new scenario.
- `app/perf/summarise.mjs` (listed under Modify) is unchanged: its
  scenario table is generic over recorded fields, so `means_1m`'s
  `ms` median and p90 print with no code change (verified on a local
  `PERF_ROWS=100000` run: `means_100k ms` 223.9).
Supersedes: —
### v1.0 Task 19 — means_1m gate; DataGrid 1M-row scroll (2026-11-19)

Decision: `means_1m` is gated at the owner-picked **3,173 ms** (median
of successful samples ≤ threshold — the Task 18 proposal, slowest
attempt's p90 2,538.2 × 1.25, chosen on PR #94). The DataGrid no
longer uses React Aria `Table`/`Virtualizer`: RAC materialises the
full collection — all 1,000,000 `items` as Row/Cell elements in the
hidden collection tree — which is what crashed the renderer at 1M
rows (`Target crashed`, React error #520). The grid renders a window
of rows itself (visible ± 1 page) with a roving-tabindex arrow-key
model, and once `total × 48 px` exceeds an 8,000,000 px scroll span
(every engine's element-height cap is ≥ ~17.9M px; Chromium clamps
scrollTop at 2^25 ≈ row 698,900) the scroll space compresses: the
scrollbar maps to a fractional first row and rendered rows re-anchor
to it at real 48 px spacing.

- Files outside the task's Files block, all under the owner's
  2026-11-19 "faster perf measurement runs" instruction
  (docs/conductor/notes/v1.0/task19.md):
  - `app/perf/scenarios/shared.ts`, `import.ts`, `tables.ts`,
    `store.ts`, `app/perf/perf.spec.ts` — the `scenarios` dispatch
    input's `PERF_SCENARIOS` filter (fixture expansion: `import_*`
    still runs when a dependent is listed; a filtered-out scenario
    records nothing) and the `filtered` flag passed to the gate.
  - `app/scripts/perf-gate.test.mjs` — unit tests for the filtered
    (`not run (filtered)`) and unfiltered gate cases the instruction
    requires, plus `means_1m` samples in the existing fixtures.
- `.github/workflows/app.yml` also gains the `scenarios` and `cpu4`
  dispatch inputs; push to main is unchanged (all scenarios + the D7
  pass).
- `grid_scrolls_to_last_row_of_1m` reports a 1,000,000-row dataset
  through a patched `getVersionPage` rather than importing 1M rows:
  an off-the-record e2e context cannot hold a real 1M-row store
  (SQLITE_IOERR — the perf job's persistent profile exists for that
  reason), and the renderer bug is a function of the row count the
  grid materialises. The real-store 1M scroll is `grid_scroll_1m`.
Supersedes: —
### v1.0 Task 23 — which-calculator guide; assumption notes (2026-11-19)

Decision: the guide is app-side data (`app/src/calc/guide.ts`) — a
tree with no cycles whose leaves are registry calculator ids, each
option one letter of `?path=a.b.c` link state, each leaf optionally
carrying `prefill` inputs the calculator's own `decodeInputs` accepts
(`two_by_two.study_type`, `power_cohort.design`). Notes render
through `NOTE_TEXT` in `app/src/explain/notes.ts`: `words` under the
table, `rule` + `source` + trigger values in explain. A note's block
is the first statistic path segment after the run-surface prefix —
calculator notes strip `calc.<id>.`, `strata[]` refines by the note's
`stratum` value, `means.*` refines to its test block — so a note sits
beneath the table it concerns; a block with no table renders once at
the end. `suggests` resolves to an in-page `#note-` anchor on the
row that displays the suggested statistic (calculator `strata[]` maps
to `[0]`), or a `/calc/<id>` link when it names a calculator. All
note wording is marked "pending adviser review", and receipts are
untouched — notes stay the engine side channel Task 22 defined.

Analysis notes persist: `runs.notes` (schema v3, migration
`0003_run_notes.sql`, `ALTER TABLE` like Task 15's `command`). A card
re-rendered from the store needs the notes the run emitted — `ProcOutput.notes`
is otherwise discarded by the worker — so the worker writes
`JSON.stringify(out.notes)` at insert and a pre-v3 row's NULL parses
to `[]` (its card shows no notes: the side channel did not exist when
it ran). NULL is never written back, matching `command`.

Files outside the task's Files block — the analysis-side plumbing the
"notes on result cards" requirement cannot work without (the
decisions entry this requires):
- `store/migrations/0003_run_notes.sql` — the new column (a
  `store/migrations/` file was implied by `command`'s precedent in
  Task 15, whose Files block did carry the store files).
- `app/src/store/repo/runs.ts` — `notes` through the repo row type,
  `COLUMNS`, `toRun` and `insert`.
- `app/src/store/opfs.ts`, `app/src/store/relay.ts`,
  `app/src/store/stdio.ts` — migration 3 registered in all three
  backends' `MIGRATIONS` (schema version now 3).
- `app/src/store/conformance/suite.ts` — schema version 3
  expectations, `notes` on every `runs.insert` call, the 0002 test
  rewound to the true v1 shape, and the new
  `migration_0003_adds_notes_on_both_backends` test (runs on the
  native and every OPFS browser backend).
- `app/src/worker/api.ts` — `RunRecord.notes: Note[]`.
- `app/src/worker/analysis.ts` — persists `out.notes`, reads it back
  (NULL → `[]`).
- `app/src/ui/screens/analysis/ResultCard.tsx` — passes `run.notes`
  to the TABLES/MEANS renderers and to `ExplainPanel`.
- `app/src/ui/screens/analysis/render-means.tsx` — the Files block's
  `render-result.tsx` lists the shared entry point only; the MEANS
  renderer lives in this sibling file (task block placement:
  `means.bartlett`/`means.t_test`/`means.anova`/
  `means.kruskal_wallis`).
- `app/src/ui/screens/analysis/stats-table.tsx` — the `#note-` row
  anchors the suggested-statistic links target (id on the row's first
  explainable stat path).
- `app/src/explain/index.ts` — `et` additions (`note`,
  `noteValues`) — all panel strings live in `et` there.
- `app/src/store/conformance/migrations.ts` (PR #97 fix round) —
  `suite.ts` grew past the ~400-line rule with the new
  `migration_0003_*` test, so the two `migration_*` cases moved here
  behind `migrationTests(ref, open)`, called from `storeConformance`'s
  describe with a `get`/`set` ref sharing its `store` binding. Test
  names, assertions and per-backend count are unchanged.
Supersedes: —

### v1.0 Task 24 — public validation page from the CI report (2026-11-19)

Decision: `/validation` is a store-less, offline dossier fed by
`validation-report --format json`: engine.yml's `validation-report`
job uploads the JSON as the `v1.0-validation-json` artifact alongside
the unchanged `v0.2-validation-md`, and app.yml's deploy build
downloads the latest green engine run's copy into
`app/public/validation.json` (same head SHA when that parallel run
already finished, else newest green main — the document names its own
`commit` and `run_url` either way). `Row` gained `method` plus a
split `r_reference`/`ei7`/`openepi` so the JSON can emit the
structured `references` object while the Markdown `reference`
column — and the whole Markdown report — stays byte-identical; the
`method` label itself (`OutputDef.method` for `calc.*`, the
`fixtures/R/*.R` generator script per procedure family) lives in
`validation::method` because `corpus` is feature-gated out of the
wasm build. A build without the file shows the wasm binding's
`validationRegistry()` entries plus "observed errors not measured in
this build"; every rendered number is a `cases` count or a formatted
`worst_error` from the JSON — the numeric tolerances are never shown,
only the class names. ExplainPanel links the selected statistic to
`/validation#<id>` with concrete `[n]` indices collapsed to the
registry's `[]` anchor ids.

Files outside the task's Files block — the transport the "registry
entries from the wasm bindings" fallback cannot work without (the
main thread never touches wasm, so the entries ride the store-less
calc worker like every other binding):
- `crates/epiora-wasm/src/lib.rs` — the `validationRegistry` export:
  `REGISTRY` as `statistics[]`-shaped `{id, method, class, cases}`
  objects (no `references`/`worst_error` — nothing measured the
  build), serialized via `JSON.parse` like `calcRegistry`.
- `crates/epiora-core/src/validation.rs` — `validation::method`, the
  shared method-label source used by the report's JSON `method`
  field and the wasm fallback (kept out of `corpus`, which is not in
  the shipped binding).
- `app/src/calc/api.ts`, `app/src/calc/client.ts`,
  `app/src/calc/calc.worker.ts` — `validationRegistry()` on the
  store-less `CalcApi` alongside `listCalculators`.
- `app/vite.config.ts` — `validation.json` added to the workbox
  precache glob so a bundled report survives offline like `/calc`.
- `.gitignore` — `app/public/validation.json` is CI-generated; it
  must never be committed (a build without it is a supported state).

Fix round (2026-11-19, PR #98 review + app CI run 44572039484):
- `references.r` is omitted from the JSON whenever no covering
  fixture recorded an `r_version`/`packages` (the plan's `"r"?` is
  optional): `calc.random_numbers.values[]` is covered only by the
  seeded `tests/golden/random_numbers.json` (no `source`, no R
  generator), so it had been emitted as the empty `": "` shell.
  `Row` gained `has_r_reference`; the Markdown `reference` column —
  and so the whole Markdown report — stays byte-identical.
- `app/src/ui/shell.module.css` — outside the task's Files block:
  the new `/validation` nav link pushed the header past 320px
  (scrollWidth 340 locally, 363 on CI), failing every route's
  320px test. The nav now wraps (`flex-wrap: wrap`, row-gap
  `--space-2`); horizontal spacing is unchanged.
- `app/src/ui/screens/validation/{ValidationTable,ValidationEntry}.tsx`
  and `app/src/ui/screens/validation/validation.module.css` are named
  in the Files block by its brace shorthand; they now render a plain
  `<table role="grid">` instead of the interactive Epiora Table —
  ~690 React Aria collection rows made every row/cell tabbable
  (4,824 `tabindex` nodes) and axe's `analyze()` exceeded the 30s
  test timeout on all three engines. The implicit
  columnheader/rowheader/gridcell roles keep the e2e role contract,
  the `#<id>` anchor moved onto the `<th scope="row">` (no extra
  element), and the scroll wrapper is a focusable `region` named by
  the caption (`scrollable-region-focusable`). Measured locally on
  the production build: axe `analyze()` 21,551 ms → 5,746 ms,
  5,547 → 4,855 elements, 4,824 → 2 `tabindex` nodes, 0 violations.

### v1.0 Task 20 — Stata export deferred (2026-11-20)

Decision: deferred. The owner has no Stata licence yet ("i dont have
stata yet so defer that", `/conduct v1.0 20`, 2026-11-20) and
`docs/decisions.md` records no FAU answer — the task's Step 1 deferral
path. Deferred: `export_stata_procedure`, `scripts/stata/verify.sh`,
`fixtures/stata/results.json`, the Stata CI job, `exportStata` and the
ResultCard control. Resume trigger: the owner has Stata; re-run Task 20
from Step 2. Carried forward as an open decision for Task 21 ("Stata
status: deferred").
Supersedes: —

### v1.0 Task 21 — v1.0 closed (2026-11-20)

v1.0 closed as a pre-release. Every gate of the v1.0 spec's decision
table is realised: G1 (OpenEpi replacement plus the start of Classic
Analysis parity — every registry calculator and MEANS merged), G2
(two releases: regression and data management are v1.1), G3
(calculators open from a link, inputs in the URL), G4 (the equivalent
Epi Info command on every Analysis card), G5 (ships as a pre-release;
epidemiologist sign-off and explain-text review recorded "pending"),
G6 (R export verified in CI; Stata deferred — see the Task 20 entry),
G7 (`openepi_display` added to platform spec §12.3) and G8 (the
"which calculator?" guide, engine-computed assumption notes and the
public validation page merged).

The validation report: `app.yml`'s `validation-report` job now renders
`docs/validation/v1.0.md` through `app/scripts/validation-report-v1.mjs`
from a source app run (default the `9448ab6` main-push run
https://github.com/dorrabbkhan/Epiora/actions/runs/44715067404,
`validation_run` input overrides) plus the sibling `engine` run's
`v1.0-validation-json` artifact for per-statistic worst errors; the
`report_version` input keeps v0.3 reproducible from run 43208388072.
`docs/validation/v1.0.md` is the unchanged artifact of run 44726281061
(PR head 376743b), sha256
`c9cf09f9755218a74c4ae811020dc4dc22f86dd175b7ec7e883754a5dcf94dd3`,
rendered from source run 44715067404 (9448ab6, attempt 2).

Versions: workspace and app bumped to `1.0.0`; `receipt_v1.json`
regenerated (only `engine.version` and `receipt_sha256` differ).

Owner decision (option A, `/conduct v1.0 21`, 2026-11-20): the 1.0.0
bump changed every receipt-bearing `parity_sha` in
`corpus_parity_main.tsv` because `normalize_receipt` pinned only
`git_sha`. `parity_norm.rs` now also pins `engine.version` to
`"unknown"` (compile-time build metadata, spec §4.3 — the D1 reason),
with a unit test showing two receipts differing only in
`engine.version` normalize to the same parity line.
`corpus_parity_main.tsv` regenerated by
`cargo run -p epiora-core --features corpus --bin corpus-runner -- --hash-golden fixtures`:
127 → 137 rows (ten `means_*` rows added by the MEANS task since the
golden was generated); on the 127 common rows only `parity_sha` cells
changed (100 receipt-bearing rows; the 27 receipt-less rows —
inline-table 2×2/stratified, `rxc`, `special.*` — are byte-identical);
no `actual`/result value, fixture hash or dataset hash changed. This
regeneration is owner-approved for this task only.

Files outside the Files block: `app/scripts/validation-report-v1.mjs`
(new: the v1.0 renderer, imported by `validation-report.mjs`);
`Cargo.lock` and `app/package-lock.json` (the version bump's
mechanical edits); `scripts/check-app.sh` (the licence check's
`--excludePackages 'epiora-app@…'` is version-pinned — bumped with
the app version); `crates/epiora-core/tests/{notes,receipt,
receipt_project,receipt_v3}.rs` (pinned `EngineStamp.version`
`0.2.0` → `1.0.0`, required by the bump);
`crates/epiora-core/src/corpus/parity_norm.rs` and
`crates/epiora-core/src/bin/corpus-runner.rs` (owner option A and its
doc comment); `crates/epiora-core/tests/explain.rs` (the
`results_unchanged` doc comment for option A);
`crates/epiora-core/tests/golden/corpus_parity_main.tsv` (the
owner-approved regeneration above). `crates/epiora-wasm/bindings/*.ts`
needed no edit — they embed no version.

Fix round 1: `app/src-tauri/tauri.conf.json`,
`app/src-tauri/Cargo.toml` and `app/src-tauri/Cargo.lock` — the
desktop app version followed `app/package.json` to `1.0.0` (the
release checkpoint checks all four app-version files; the lock diff
is version-only — `epiora-app` and `epiora-store-native`);
`app/scripts/validation-report-v1.fixtures.mjs` (new: the v1.0
sample inputs and `buildV1Input`, extracted from
`validation-report.test.mjs` so both files stay under ~400 lines;
assertions, test names and expected values unchanged).

Open decisions carried forward: epidemiologist adviser and sign-off
(G5, pending); Stata export (deferred, Task 20 — resume when the
owner has Stata); D10 SAS verification route (before v1.2); D11 DAG
adjustment-set helper route (v1.2).
Supersedes: —

### Tab take-over releases the OPFS pool (2026-11-22)

- Bug: "Use here" on the lock screen took the Web Lock and reloaded
  the new tab, but its project list stayed "Loading…" forever — the
  console repeated `opfs-sahpool` `NoModificationAllowedError`
  (`createSyncAccessHandle` refuses while another context holds a sync
  access handle). Reproduced by the conductor on the deployed build
  (Chromium, 2026-11-22). The `use_here_takes_over` e2e missed it: it
  only waited for the "Projects" heading, which renders while the list
  is still loading.
- Cause: on take-over the old tab's data worker ran `closeStore()`,
  which closes the sqlite database — but the installed `opfs-sahpool`
  VFS keeps a sync access handle on every pool file until
  `SAHPoolUtil.pauseVfs()` runs or the worker is terminated. Nothing
  did either, so the new tab's worker could never open the pool.
- Fix: bundled `@sqlite.org/sqlite-wasm` 3.54.1-build1 supports
  `pauseVfs`/`unpauseVfs`/`isPaused`, so no worker termination or new
  API is needed. `closeStore` (`app/src/worker/store-backend.ts`) now
  calls the new `releaseOpfsPool()` (`app/src/store/opfs.ts`) after the
  store closes; `openOpfsStore` re-arms a paused pool via
  `unpauseVfs()` so a release is never fatal to a later open. The old
  tab still reports 'lost' and stays on the lock screen, and "Use
  here" there works again (take-over is symmetric). No lock-protocol
  messages changed; no dependency added.
- Tests: `lock_takeover_releases_pool_for_next_worker` in
  `app/src/worker/tab-lock.browser.test.ts` drives a real data
  worker, posts the LockScreen's 'take-over' through `takeOver()`,
  waits for the worker's 'lost' broadcast, then opens the same
  database from a second worker and reads the project the first wrote
  — with the first worker never terminated (fails before the fix:
  "harness store did not open" on chromium, firefox and webkit).
  `app/e2e/tab-lock.spec.ts` `use_here_takes_over` now creates the
  'Oswego' project in the first tab, requires its rowheader and no
  'Loading…' in the tab that took over (proves the same database
  opened), and adds the A→B→A round trip back through "Use here" —
  all three engines (fails before the fix: the rowheader never
  renders).
- Files outside the Files block (`app/src/store`, `app/src/worker`,
  `app/e2e/tab-lock.spec.ts`): `docs/decisions.md` (this entry,
  required by the fix brief).
- Follow-up fix (2026-11-22): RUSTSEC-2026-0592 "Stack overflow in
  `Url::origin()` on nested `blob:` URLs", published the same day,
  failed `cargo deny check` in the native-store and check-app CI jobs
  on `url 2.5.8` (transitive, `app/src-tauri/Cargo.lock`). Bumped to
  `url 2.5.9` via `cargo update -p url`; the only other lock changes
  are `form_urlencoded` 1.2.2 → 1.2.3 (a direct dep of `url`, required
  by cargo) and a re-resolved `syn` edge inside `cssparser-macros`
  0.7.1 (version unchanged). No `deny.toml` ignore was added. It was
  pushed to #102's branch after #102 merged, so it lands as its own PR;
  the advisory — not #102's code — blocked CI and the v1.0.0 release.
Supersedes: —

### v1.0.0 released (2026-11-22)

- Release: <https://github.com/dorrabbkhan/Epiora/releases/tag/v1.0.0>,
  `Epiora v1.0.0 — Calculators and MEANS`, a pre-release published
  2026-11-22 by the owner. Tag `v1.0.0` points at
  `4de761f388ff7c168dce30aea18ef529694d1f92` (the #103 merge). It is a
  pre-release because adviser sign-off and the explain-text review are
  still pending (platform spec §1, v1.0 spec G5) and the desktop
  builds are unsigned. To make it a full release later:
  `gh release edit v1.0.0 --prerelease=false --latest` (no new tag).
- Pre-release checkpoint: `scripts/release/checkpoint.sh 1.0.0` on
  `4de761f` reported 0 FAIL (`collect-assets.sh` refuses on any FAIL).
- Evidence (main push runs on `4de761f`): app
  [45039095330](https://github.com/dorrabbkhan/Epiora/actions/runs/45039095330),
  perf gate: `import_1m` median 53,050.9 ms ≤ 89,550,
  `open_tables_1m` median 1,637.6 ms ≤ 2,874, `means_1m` median
  1,365.3 ms ≤ 3,173; desktop
  [45039095398](https://github.com/dorrabbkhan/Epiora/actions/runs/45039095398);
  spikes
  [45039095281](https://github.com/dorrabbkhan/Epiora/actions/runs/45039095281).
  Earlier main pushes after the version bump hit the slow perf-runner
  mode (`import_1m` about 99–101 s) and needed re-runs; the release
  evidence is the `4de761f` run above.
- Assets: the six unsigned desktop bundles from desktop run 45039095398
  (AppImage, deb, rpm, NSIS setup exe, msi, dmg) plus `SHA256SUMS.txt`,
  collected by `scripts/release/collect-assets.sh`.
- Desktop hand checks: the owner checked the Linux bundles and the
  Windows bundles (in a VM). The macOS `.dmg` is built in CI but **not
  hand-tested**: no Mac is available.
- Fixes merged between closing v1.0 (#100) and the release: #102 (the
  tab-lock "Use here" take-over hung at "Loading…" because the old
  worker kept the OPFS SAH-pool handles; `closeStore` now pauses the
  pool) and #103 (`url` 2.5.9 for RUSTSEC-2026-0592).
- Production deploy: release run
  [45049416918](https://github.com/dorrabbkhan/Epiora/actions/runs/45049416918)
  (success, 11m39s), Netlify deploy `6b03285b22cce3cc028c34f9`
  (<https://6b03285b22cce3cc028c34f9--epiora-dev.netlify.app>), live at
  <https://epiora-dev.netlify.app>.
- Post-release check: `scripts/release/verify-deploy.py
  https://epiora-dev.netlify.app` reported 38 checks, 0 FAIL. The owner
  did the manual pass on production: first load, offline reload,
  import, FREQ and TABLES.
- `docs/releasing.md`: the pre-release reason ("0.x internal
  foundation") and the rollback note ("for the first release none
  exists") were left over from v0.3.0; both are corrected in this PR.
Supersedes: —

### v1.1 Task 1 — command register (2026-11-22)

Decision: `docs/v1.1-command-register.md` + `crates/epiora-lang/register.json`
enumerate every Classic Analysis command (84 grammar-derived, plus spec-§5.3's
non-grammar CANCEL READ and KEYVARS), 55 dispatch-listed functions, all 28
`MetaFieldType` members and 20 module features from the Epi Info 7 source at
4cd353c — each supported at a version or planned ≤ v1.6, or not_planned with a
reason. Counts: commands — supported 37 (v1.0: 3, v1.1: 34), planned 37
(v1.2: 2, v1.3: 24, v1.4: 2, v1.5: 2, v1.6: 7), not_planned 12; functions —
supported 46 (v1.1), planned 3 (v1.3), not_planned 6; field types — 28 all at
v1.1; module features — v1.0: 1, v1.3: 11, v1.4: 7, v1.5: 1.
Roadmap gaps (not_planned, reasons in the register): EXECUTE/WAITFOR*,
PYTHON/R blocks, SQLEXEC/RECORDSET/CONNECTION, COMMANDLINE, GEOCODE/
GETCOORDINATES, IOCODE, SENDSMS, SYSBARCODE/SYSLATITUDE/SYSLONGITUDE/
SYSALTITUDE, DEFINE DLLOBJECT/NETOBJECT, DELETE File/TABLES, READ file-spec
forms. Proposed v1.1 additions beyond spec §5.3: UNDELETE, RUNPGM, QUIT, EXIT,
SUMMARIZE, KEYVARS, CANCEL READ, plus the §6.1 note-only set — all subject to
this OWNER REVIEW.
Ambiguities resolved and recorded in the register doc: `version` is the
Classic Analysis target for `grammar: "both"` entries (check-code targets in
reason); COMMANDLINE, EXIT, FILEDIALOG, MAP, NEWPAGE, SYSINFO, WAITFOR* parse
but have no Rule class at 4cd353c; DATEDIFF/DATEPART are undispatched helper
classes, not callable functions; `working-menu.grm` is the .mnu grammar,
covered by COMMANDLINE; `List` (MetaFieldType 27) takes spec §4.2's catch-all.
Evidence: register-check `node scripts/legacy/register-check.mjs` → exit 0
(86 commands, 55 functions, 28 field types, 20 module features).
Files outside the Files block:
- scripts/legacy/register-check.mjs — the check Step 3 creates and runs.
- crates/epiora-lang/reference/ (5 files + NOTICE) — verbatim Epi Info 7
  grammar/dispatch/enum copies under D2 (Apache-2.0) so the check is hermetic;
  provenance recorded in reference/NOTICE.
Supersedes: —

### v1.1 Task 2 — legacy samples (D12) (2026-11-22)

Decision: `fixtures/legacy/samples/ei7/` holds every `.prj`, `.mdb` and `.db`
under `Epi.Core/Projects/` @ `4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6`
(46 files, 20 folders — KML_Example and Rates have no such files, listed in
`fixtures/legacy/samples/README.md`; Apache-2.0, `ei7/NOTICE`). All project
databases are JET4; `BeefPrevention.db` and `Sample/SampleDB.db` are SQLite.
The 21 `.prj` of the D2 inventory are 19 project files + 2 ESRI shapefile
projections in CommunityHealthAssessment (copied, flagged in the README).

D12 verdict — Epi Info 3.5.4 tutorial samples: **redistributable** (OWNER
REVIEW). Evidence: `fixtures/legacy/samples/ei3/` commits the contents of the
CDC-hosted `EIHATProjectFiles.zip`
(`https://www.cdc.gov/epiinfo/zipfiles/eihat/EIHATProjectFiles.zip`, live
2026-11-22; SHA-256 9954794cc6d721c6c50030f284e7f3d0a4f53cbacee8cff0667a039b37a17dee;
page `https://www.cdc.gov/epiinfo/support/downloads/eihat.html`, mirrored at
restoredcdc.org). The ZIP contains no licence file. The companion tutorial
document (EIHATFull.pdf, HHS/CDC/NCPHI, document version 2.0, October 2005,
`https://www.cdc.gov/epiinfo/pdfs/eihat/EIHATFull.pdf` — fetched via the
restoredcdc.org mirror; the cdc.gov path now serves a 404 page) states on its
Copyright Information page: "Epi Info is a trademark of CDC. Epi Info™
programs are provided in the public domain to promote public health.
Programs may be freely translated, copied, or distributed. No warranty is
made or implied for use of the software for any particular purpose." The
files are a U.S.-government work product of HHS/CDC (17 U.S.C. §105);
survey data "were created for the tutorial". Caveat recorded for the owner:
the document credits "the collaborative efforts of the CDC, the Assessment
Initiative (AI), and the New York State Department of Health (NYSDOH)" and
"asthma data provided to the CDC by the NYSDOH" — a state-agency
collaboration; if the owner prefers caution the fallback is
`ei3/fetch.json` + `scripts/legacy/fetch-ei3.sh` (download-only), and if the
files turn out unobtainable spec §8.2 ships EI3 "unvalidated".
`ei3/censusage_files/` (saved-web-page CSS/JS/GIF chrome, not tutorial data)
is the only ZIP member not committed.

Views/records per project are inventoried in
`fixtures/legacy/samples/README.md` (metaViews + mdbtools 1.0.1 native
build, `mdb-tables`/`mdb-export`/`mdb-count`; SQLite via python3 sqlite3).
`mojibake`/JET3 code-page coverage: six of the seven EI3 `.mdb`s are JET3
(`albany_demogz.mdb` is JET4).

Files outside the Files block:
- fixtures/legacy/samples/SHA256SUMS — the step-1 "record SHA-256 of every
  file" made machine-checkable (70 files, `sha256sum -c` verifiable);
  keeping it out of the README table keeps the inventory readable.
Supersedes: —

### v1.1 Task 3 — EI7 reference corpus II (2026-11-23)

Decision: `fixtures/ei7/v11/` commits seven Analysis programs and their
workflow-produced outputs: `select_sort` (29 steps), `define_assign_if`
(40), `recode` (35), `merge_relate` (20), `delete_set` (21),
`functions_date` (34), `functions_text_math` (27). Each `<topic>.pgm`
READs a scratch copy under `ei7-out/work/v11-data` (Epi Info MERGE writes
into the destination file on disk), ROUTEOUTs to `<topic>.html`, and is
parsed by `fixtures/ei7/parse.mjs --steps` into `{source, steps}` json —
one step per non-comment source line; `rows` holds the first output table
(LIST rows verbatim, header included), `results` holds registry-path
numerics (positional `freq.*`, `means.*`, `two_by_two.*` via labels.json,
`selection.record_count`, `selection.deleted_records_included`,
`delete.records_deleted`, `undelete.records_restored`). Generating run:
`45161983082` (all seven `run: OK`, jsons schema-valid, drift check
green); development runs `45151708069`, `45156679205`, `45158293708`,
`45159694424`, `45160531102` each produced measured failures that shaped
the corpus (all recorded in `fixtures/ei7/v11/README.md`).

Measured Epi Info 7.2.7.0 deviations from grammar/docs (each pinned or
worked around in the corpus): `SET` requires `END-SET`; `DELETE`/`UNDELETE`
need `RUNSILENT` headless; `UNDELETE` is a no-op on raw `.mdb` READs ("The
current Read must be an Epi7 project", `records_restored: 0`, FREQ total
stays 46 after `DELETE ILL = 0` removed 29 of 75); `MERGE`'s
`APPEND`/`UPDATE`/`RELATE` option crashes in `Rule_Merge.SetMergeOpt`
(IndexOutOfRange) — only bare `MERGE file:table key` match-or-insert is
pinned (Nutrition1 grew 3 -> 13 rows across two merges); `DEFINE x = f()`
rejects function calls as column lookups; `MINUTES(a,b)` and
`NUMTODATE(y,m,d)`/`NUMTOTIME(h,m,s)` arities differ from classic Epi Info;
`ABS`/`TRUNC` throw NullReferenceException on missing input (corpus uses
the fully populated `HeightInches`); `LOG` is a reserved but unparseable
token; FREQ excludes missing values including from Total
(`SELECT MASHEDPOTA = (.)` -> record_count 1, FREQ Total 0); RELATE
expands to 11 rows MATCHING / 13 ALL (all 11 PatientVisits2 belong to
patient "1"); `SET MISSING=ON` adds a `Missing` FREQ category (24 of 46
undeleted rows for TIMESUPPER). EI7 uppercases the ROUTEOUT filename
(`DEFINE_ASSIGN_IF.HTML`); the committed corpus normalises names to
lowercase, bytes unchanged.

EI3 upgrade probe (nonblocking, `ei3_probe` dispatch input): run
`45177072437` completes unattended — all 800 source rows land in each of
the four upgraded page tables (probe.log verbatim tail):

    STATUS Started updating image fields in [PreInterventionSurvey]...
    STATUS No image fields found to update in [PreInterventionSurvey].
    STATUS Converting Form Relations...
    STAGE3 ImportEnded
    STAGE3 destination mdb: D:\a\Epiora\Epiora\ei3-probe\UpgradedProbe\UpgradedProbe.mdb
    COUNT PreInterventionSurvey 800
    COUNT PreInterventionSurvey1 800
    COUNT PreInterventionSurvey2 800
    COUNT PreInterventionSurvey3 800
    COUNT source PreInterventionSurvey 800

The earlier hang was the modal `ImportMessagesDialog`, not the upgrade:
`ProjectUpgradeManager.Import()` calls `RaiseEventImportEnded()` and
then `new ImportMessagesDialog().ShowDialog()`
(`Epi.ImportExport/ProjectUpgradeManager.cs` ~lines 346-352 @ `4cd353c`),
which blocks forever headless — run `45159694424`'s artifact already
held the fully upgraded `UpgradedProbe\UpgradedProbe.mdb`. The probe now
subscribes `ImportStatus` (logged as `STATUS`) and `ImportEnded`, counts
rows over OleDb inside the handler and calls `Environment.Exit(0)`
before the dialog opens; the probe step is capped at
`timeout-minutes: 20`. Earlier attempts: `45156679205` reached stage 1
before PowerShell's native-stderr handling killed the step;
`45151708069` died at probe start on the anycpu build and uninitialised
configuration. Note: run `45177072437`'s `ei7` corpus job failed "No
drift in fixtures/ei7" — the only diff is the `<i>Date:</i>` timestamp
Epi Info stamps into each `.html` at generation (`11:08:08 AM` ->
`12:44:51 PM`); the `.json` steps data is unchanged. The drift is
pre-existing (the generating run `45161983082` passed because the v11
outputs were still untracked). Resolved in fix round 3 (owner-approved
option A): `ei7.yml` rewrites the banner to the literal token
`<i>Date:</i>&nbsp;&nbsp;<b>[run timestamp]</b>` after the v11
copy/rename and before parse.mjs (raw bytes, UTF-8 no BOM, CRLF kept);
run `45182109352` proved the drift was only that stamp (its artifact
re-committed the two html verbatim) and run `45182452028` went green
including "No drift in fixtures/ei7".

Files outside the Files block:
- `fixtures/ei7/steps.mjs` — fix round 1: the `--steps` corpus half of
  `parse.mjs` (program-line <-> <h3> correlation, step building and the
  `--pgm`/`--input` source block) split out verbatim so both files stay
  under the ~400-line rule (plan Global Constraints). Pure move: the
  ei7.yml invocation, flags and produced json are unchanged.
Supersedes: —
### v1.1 Task 5 — mdbtools wasm module and EPTB batches (2026-11-23)

Decision: `crates/epiora-mdb` compiles vendored mdbtools 1.0.1 libmdb
(verbatim from the pinned tarball) plus `c/harness.c`/`c/eptb.c`/
`c/codepages.c` natively via `cc` in build.rs and to wasm via
`build-wasm.sh` (pinned emsdk 6.0.13). `build.rs` generates
`mdbtools.h` from `mdbtools.h.in` (no autoconf: iconv and xlocale off,
fake-glib in, `LOCALE_T=locale_t`, `TLS=_Thread_local` plus the
configure DEFS). `epiora-legacy::batch::decode_batch` decodes EPTB v1;
`app/src/legacy/mdb.ts` wraps the committed module in
`app/public/mdb/` (LGPL notice + §6 source-offer beside it).

Ambiguities resolved:
- JET3 code-page test file: the task text names `testV1997.mdb` for
  `jet3_cp1252_text_decodes`, but that file's four user tables contain
  ASCII only. `testIndexCodesV1997.mdb` (same jackcess commit,
  JET3/cp1252) holds `AA<byte>AA` cells covering the high half of the
  page, including the five cp1252-undefined bytes stored raw — used
  instead; provenance in `crates/epiora-mdb/tests/data/NOTICE`.
- JET3 text decoding: the iconv-less build returns latin1-mapped UTF-8
  for cp1252 files and raw bytes otherwise; `eptb.c` recovers the byte
  stream and decodes with the caller's single-byte page
  (windows-874/1250..1258, default windows-1252 per plan), undefined
  bytes → U+FFFD counted in the batch's `decode_errors` extension.
- `Mdb::batch` returns `Result<Vec<u8>, MdbError>` — the plan's
  shorthand `-> Vec<u8>` omits the Result; spec rule that no input may
  panic requires typed errors. Added internal codes
  `mdb_no_table`/`mdb_encode`/`mdb_code_page_unsupported`/
  `mdb_tables_json` alongside the four plan codes.
- Password detection is a header sniff (jackcess JetFormat offsets:
  JET3 20 bytes at 0x42; JET4 40 bytes at 0x42 masked by (int) of the
  f64 header date at 0x72) because JET4 database passwords do not
  encrypt pages — mdbtools reads such files happily.
- ACE/`.accdb` files open as `mdb_unsupported_version` by header
  version byte (mdbtools would treat them as JET4).
- The five `data.c`/`mdbtools.h.in` patches (a `MdbColumn.is_null`
  field and the one-line `_mdb_attempt_bind` assignment setting it)
  are required for the EPTB missing bitmap — upstream discards
  null-ness inside `_mdb_attempt_bind`; recorded in
  `crates/epiora-mdb/NOTICE`. For MDB_BOOL the JET null bit is the
  value, never missingness.
- Thread-safety: the harness's last_error/last_format state is
  `_Thread_local` and handle-slot allocation is CAS — parallel cargo
  tests open several databases at once; `Mdb` is `!Send`/`!Sync`.
- `epiora-legacy` scaffolding (Cargo.toml, `src/lib.rs`, `src/error.rs`
  with `LegacyError`) is implied by the Files block's `src/batch.rs`;
  mappers arrive in Tasks 7–8.
- `mdb.test.ts` resolves fixtures via `process.cwd()` (vitest cwd =
  app/) — `import.meta.url` is a dead dev-server URL under vitest.

Files outside the Files block:
- `crates/epiora-mdb/c/epiora_mdb.h`, `c/codepages.c` — shared
  interface header and the generated single-byte code-page tables.
- `crates/epiora-mdb/src/bin/dump-batches.rs` — regenerates the
  native oracle batches under `tests/data/batches/sample/`.
- `crates/epiora-mdb/vendor/mdbtools-1.0.1/` — the vendored libmdb
  (the Files block's crate contents imply it; no registry crate for
  libmdb exists).
- `crates/epiora-mdb/fuzz/Cargo.toml`, `fuzz/rust-toolchain.toml` —
  the `mdb_any` target needs the standalone fuzz crate
  (same pattern as epiora-core/epiora-import).
- `.github/workflows/engine.yml`, `fuzz-nightly.yml` — `mdb_any`
  added to both fuzz lists (spec §6.5's "fuzz targets run in the
  same PR" requires them registered).
- `app/eslint.config.js` — `public/mdb/**` added to the ignore list:
  the generated wasm glue must not be linted.
- `deny.toml` — a comment only: the LGPL C dep is outside
  cargo-deny's scope; all new Rust deps already hold allow-list
  licences, so no allow-list change was needed.
- `crates/epiora-legacy/src/batch/tests.rs` — the batch decoder's
  test module, split out of `batch.rs` in fix round 1 to keep the
  file under the ~400-line rule; assertions unchanged.
- `crates/epiora-mdb/tests/data/batches/chunk7/` — native chunked
  oracle batches (`--chunk` mode of `dump-batches`, fix round 1) for
  the wasm `batches(table, 7)` parity check.

Evidence: `cargo test -p epiora-mdb` 7/7 pass (incl. proptest
`random_bytes_never_panic`); `cargo test -p epiora-legacy` 2/2;
`npx vitest run src/legacy/mdb.test.ts` green — the committed wasm
module's batches are byte-identical to the native encoder's for all
93 Sample.mdb tables.
Supersedes: —
### v1.1 Task 6 — dataset model v2: Time/DateTime, meta, labels, links, snapshot v2 (2026-11-23)

Hash rule (the OWNER REVIEW item; recorded verbatim as the plan states
it, then the encoding details): *canonical v1 bytes remain unchanged
for legacy datasets; when any column uses `Time`/`DateTime` or any
`label_set` is set, a trailing `LBLS` section is appended; label sets
are serialized in id order; each label entry is canonical value bytes
plus UTF-8 label text; prompts, legacy types and legacy names never
enter the hash.* In code (`dataset/hash.rs`): the `LBLS` section is
`"LBLS"` + u32 label-set count + per-set (u32 id-len + id bytes +
u32 entry count + per entry (value type tag + canonical value bytes +
u32 label-len + label UTF-8)). Reason: labels are user-visible data —
two datasets differing only in labels must not collide — while prompts
and legacy names are provenance/report text that must never change a
dataset's identity; the suffix keeps every v1 fixture hash
byte-identical (v1 datasets carry no v2 section).
Decision: `Dataset` gains `ColumnData::Time` (i32 seconds since
midnight, `0..=86_399` else `EngineError::InvalidTime`,
`invalid_time`), `ColumnData::DateTime` (i64 seconds since
1970-01-01, no zone), matching `Value`/`ValueType` variants (tags 6,
7); `meta`/`label_sets` attach only through `with_meta` (meta length,
`label_set` references and Time label entries are validated,
`LabelSetMissing` = `label_set_missing`). Snapshot v2 = v1 layout +
per-column meta + trailing label sets; `datasetToSnapshot` writes v1
when no v2 feature is present, so existing snapshot bytes are
unchanged. `dataset_links` live in the store (migration 0004), never
inside `Dataset`.
Ambiguities resolved: `app/src/store/types.ts` is unchanged — repo
files hold their own row interfaces (see `repo/datasets.ts`), so the
five new row types live in `repo/legacy.ts`. `FreqTable`/`Labels2x2`/
`TablesResult` gain `value_labels` gated by `skip_serializing_if` so
unlabelled (legacy) results keep their JSON byte-for-byte. Label
entries preserve source order; duplicated entries keep first-match
lookup. `-0.0` equals `0.0` in label lookup and canonical bytes.
Dataset-JSON v2 writes `time`/`date_time` columns as `hh:mm:ss` /
`YYYY-MM-DD hh:mm:ss` strings — same convention as `date`'s ISO
strings — and label entries as `[{type, value}, label]` pairs.
Evidence: `cargo test -p epiora-core --all-features` all green incl.
the 10-task-test file (`v1_datasets_hash_unchanged` pins all 11
`fixtures/data/*.dataset.json` hashes) and `cargo test -p
epiora-import --all-features` all green — command run this session.
Files outside the Files block (new `Value`/`ColumnData`/`ValueType`
variants force every exhaustive match workspace-wide):
- `crates/epiora-core/src/corpus/dataset_json.rs` — v2 JSON read
  (meta, label_sets, time/date_time strings); a loader only, no
  writer.
- `crates/epiora-core/src/export/command.rs` — Time/DateTime cells in
  the corpus/R export command path.
- `crates/epiora-core/src/export/corpus_datasets.rs` — Time/DateTime
  cells in corpus dataset export.
- `crates/epiora-core/src/export/csv.rs` — Time/DateTime cells in CSV
  export.
- `crates/epiora-core/src/export/r_emit.rs` — Time/DateTime cells in
  the R export emitter.
- `crates/epiora-core/src/tabulate/categories.rs` — category keys and
  `hh:mm:ss`/`YYYY-MM-DD hh:mm:ss` display.
- `crates/epiora-core/src/tabulate/groups.rs` — MEANS numerics for
  Time/DateTime.
- `crates/epiora-core/src/procedures/mod.rs` — `value_labels` field
  sits beside the result structs it documents.
- `crates/epiora-import/src/coerce.rs` — Time/DateTime coercion.
- `crates/epiora-import/src/excel_date.rs` — Time/DateTime in the
  Excel date path.
- `crates/epiora-import/src/infer.rs` — Time/DateTime inference.
- `crates/epiora-import/src/keys.rs` — Time/DateTime in key columns.
- `crates/epiora-import/src/records.rs` — Time/DateTime in record
  paths.
- `crates/epiora-import/tests/diff_property.rs` — new enum variants
  force the exhaustive match here too.
- `crates/epiora-wasm/bindings/ColumnMeta.ts` — regenerated by the
  pinned `export_bindings` gate.
- `crates/epiora-wasm/bindings/DatasetLink.ts` — regenerated by the
  pinned `export_bindings` gate.
- `crates/epiora-wasm/bindings/FreqTable.ts` — regenerated by the
  pinned `export_bindings` gate.
- `crates/epiora-wasm/bindings/Labels2x2.ts` — regenerated by the
  pinned `export_bindings` gate.
- `crates/epiora-wasm/bindings/LabelSet.ts` — regenerated by the
  pinned `export_bindings` gate.
- `crates/epiora-wasm/bindings/LinkKind.ts` — regenerated by the
  pinned `export_bindings` gate.
- `crates/epiora-wasm/bindings/TablesResult.ts` — regenerated by the
  pinned `export_bindings` gate.
- `crates/epiora-wasm/bindings/Value.ts` — regenerated by the pinned
  `export_bindings` gate.
- `crates/epiora-import-wasm/bindings/Value.ts` — regenerated by the
  pinned `export_bindings` gate.
- `crates/epiora-import-wasm/bindings/ValueType.ts` — regenerated by
  the pinned `export_bindings` gate.
- `app/src/worker/snapshots.ts` — the snapshot scanner must decode
  format 2 (tags 6/7, meta blocks) to show stored datasets.
- `app/src/worker/errors.ts` — the two new error codes are surfaced
  to the UI.
- `app/src/ui/strings.ts` — every engine error code requires a user
  message (`contract.test.ts` enforces it); `invalid_time` and
  `label_set_missing` got theirs.
- `app/src/store/opfs.ts` — the OPFS backend registers migration
  0004 (the Files block's `repo/*`/`conformance/*` are dead code
  without it).
- `app/src/store/relay.ts` — the relay backend registers migration
  0004 (the Files block's `repo/*`/`conformance/*` are dead code
  without it).
- `app/src/store/stdio.ts` — the stdio backend registers migration
  0004 (the Files block's `repo/*`/`conformance/*` are dead code
  without it).
- `crates/epiora-core/tests/dataset_v2_procedures.rs` — the FREQ /
  TABLES / MEANS dataset-v2 tests, split out of `dataset_v2.rs`
  verbatim to keep both files under the ~400-line cap.
Supersedes: —

### v1.1 Task 4 — golden imports (2026-11-23)

Decision: `fixtures/legacy/golden/` commits the expected-import tree for
every legacy sample, produced by `fixtures/legacy/golden.mjs` in the
`legacy-golden` workflow and committed byte-identical to the workflow
artifact. Producing run (all steps green, including the in-run
second-run no-diff proof):
https://github.com/dorrabbkhan/Epiora/actions/runs/45201754092
Toolchain on the runner: mdbtools 1.0.1-0.1build1, sqlite3
3.46.1-9ubuntu0.3, Node 22.23.4 (versions printed by the "Install
mdbtools and sqlite3" step of that run).

Per-project shape in the produced tree (views = `views[]` in
project.json, links = grid/relate datasets, records = rows across all
emitted datasets):

| project | source_format | views | links | records |
| --- | --- | ---: | ---: | ---: |
| fixtures/legacy/golden/ei3/Asthma Final | ei3_access | 1 | 0 | 800 |
| fixtures/legacy/golden/ei3/Asthma Survey 2005 | ei3_access | 1 | 0 | 10 |
| fixtures/legacy/golden/ei3/Asthma Survey 2005 Sample | ei3_access | 1 | 0 | 800 |
| fixtures/legacy/golden/ei3/HospitalPoints | ei3_access | 0 | 0 | 0 |
| fixtures/legacy/golden/ei3/SchoolPoints | ei3_access | 0 | 0 | 0 |
| fixtures/legacy/golden/ei3/SymptomsbyAgeSample | ei3_access | 0 | 0 | 0 |
| fixtures/legacy/golden/ei3/albany_demogz | ei3_access | 0 | 0 | 0 |
| fixtures/legacy/golden/ei7/BeefPrevention/BeefPrevention | ei7_sqlite | 1 | 0 | 0 |
| fixtures/legacy/golden/ei7/Cholera/Advanced Analysis | ei3_access | 0 | 0 | 0 |
| fixtures/legacy/golden/ei7/Cholera/Cholera_in_Rwenshama_2 | ei7_access | 3 | 0 | 66 |
| fixtures/legacy/golden/ei7/Cholera/EpiChol | ei3_access | 0 | 0 | 0 |
| fixtures/legacy/golden/ei7/CommunityHealthAssessment/CommunityHealthAssessment | ei7_access | 3 | 0 | 800 |
| fixtures/legacy/golden/ei7/CommunityHealthAssessment/albany_demogz | ei3_access | 0 | 0 | 0 |
| fixtures/legacy/golden/ei7/EColi/EColi | ei7_access | 2 | 0 | 359 |
| fixtures/legacy/golden/ei7/H1N1ClinicalLongForm/H1N1ClinicalLongForm | ei7_access | 8 | 14 | 0 |
| fixtures/legacy/golden/ei7/H1N1ClinicalShortForm/H1N1ShortForm | ei7_access | 3 | 8 | 2 |
| fixtures/legacy/golden/ei7/H1N1ShortForm/H1N1ShortForm | ei7_access | 3 | 8 | 2 |
| fixtures/legacy/golden/ei7/HIV/HIV | ei7_access | 1 | 0 | 359 |
| fixtures/legacy/golden/ei7/HansensDisease/HansensDisease | ei7_access | 4 | 11 | 1 |
| fixtures/legacy/golden/ei7/HuttYouthSurvey/HuttYouthSurvey | ei7_access | 1 | 0 | 0 |
| fixtures/legacy/golden/ei7/JohnSnow/JohnSnow | ei7_access | 1 | 0 | 452 |
| fixtures/legacy/golden/ei7/LandmineStudy/LandmineStudy | ei7_access | 1 | 0 | 0 |
| fixtures/legacy/golden/ei7/Legionellosis/Legionellosis | ei7_access | 1 | 0 | 40 |
| fixtures/legacy/golden/ei7/Lyme/Lyme | ei7_access | 1 | 0 | 1159 |
| fixtures/legacy/golden/ei7/Mumps/Mumps | ei7_access | 1 | 0 | 45 |
| fixtures/legacy/golden/ei7/Nutrition/Nutrition | ei7_access | 2 | 1 | 14 |
| fixtures/legacy/golden/ei7/PHIN/PHINVS | ei3_access | 0 | 0 | 0 |
| fixtures/legacy/golden/ei7/STEPS/STEPS_EI7 | ei7_access | 1 | 0 | 0 |
| fixtures/legacy/golden/ei7/STEPS/Steps_EpiInfo | ei3_access | 0 | 0 | 0 |
| fixtures/legacy/golden/ei7/Salmonellosis/Salmonellosis | ei7_access | 2 | 0 | 309 |
| fixtures/legacy/golden/ei7/Sample/Sample | ei7_access | 18 | 3 | 4088 |
| fixtures/legacy/golden/ei7/Sample/SampleDB | ei7_sqlite | 19 | 3 | 4585 |
| fixtures/legacy/golden/reimport | ei7_sqlite | 19 | 3 | 4587 |

33 projects, 98 views, 51 links, 18478 records (the reimport copy is
SampleDB minus 1 removed record plus 3 added records: 4585 − 1 + 3 =
4587).

Layout readings where the format left room for choice:

- EI7 view → base table: `metaFields.DataTableName` (first non-empty
  value per ViewId), matching `Epi.Core`'s `GetDataTableName`. Page
  tables are `<base><PageId>`, grid tables `<page table><FieldName>`;
  `metaGridColumns` supplies grid columns, ordered by `GridColumnId`,
  with `UniqueKey` prepended when the physical table has it (metadata
  omits it in some projects).
- Codes fields store display text in the field column plus a companion
  code column; `SourceTableName`/`CodeColumnName`/`TextColumnName`
  (trimmed — mdb cells can carry a leading space) or the inline `List`
  expression become dataset `label_sets`.
- EI3 `Lists` expressions (`C`, `W`, `L` forms) resolve through the
  `SOURCE` rows (`CODE*` names → datatables), not bare table names.
- EI3 `MIRROR` fields import the source field's values and are reported
  `field_imported_changed`. Deleted rows are kept, not dropped, in both
  mappers: rows with `RECSTATUS`/`RecStatus` = 0 stay in the emitted
  dataset, the status column travels with them (EI3 as a verbatim
  `RECSTATUS` system column, EI7 as the `RecStatus` metaField's integer
  column), `deleted_records` counts them, and `records` counts every
  emitted row, deleted rows included.
- Sources with no view-definition tables (a JET4 `.mdb` without
  `metaViews`, a `.db` without it, an EI3 file without `view*` tables)
  report `legacy_layout_unknown` plus `table_not_imported` per plain
  table; report `item` fields name the project stem or table, never a
  path, so the Access and SQLite-copy goldens compare byte-for-byte
  (`source`, `source_format` are the only provenance fields excluded by
  `fixtures/legacy/compare-golden.mjs`).
- `copy-tables.mjs --sqlite` emits text cells as `CAST(X'<hex>' AS
  TEXT)`: the sqlite3 shell strips `\r` from scripts read line-wise, and
  the copy contract is verbatim bytes (a `\r\n` prompt in Access equals
  `\r\n` in the copy). `--tsql` keeps plain literals; Task 9 runs it.
- The reimport fixture is fixed, not random: the first three Oswego
  `GlobalRecordId`s (lexical) give 1 removed + 2 changed rows, three
  fixed GUIDs (`a0000000-…-001`..`003`) are added, one `ReimportFlag`
  column is added; `produced-ids.json` records which GUIDs were picked
  and `expected-diff.json` (authored, verified identical to the run's
  output) states the expected diff vocabulary.

Files outside the Files block:
- `fixtures/legacy/csv.mjs` — the `mdb-export` CSV reader (quoted NULL
  vs empty string, embedded newlines) shared by the readers.
- `fixtures/legacy/legacy-read.mjs` — the uniform table/schema/row
  interface over `mdb-export`/`mdb-schema` (JET3/JET4) and `sqlite3
  -json`.
- `fixtures/legacy/legacy-values.mjs` — cell conversion, case-
  insensitive lookup, check-code block counting, and
  `unknownLayoutEntries` shared by both mappers.
- `fixtures/legacy/ei7-project.mjs` — the EI7 mapper `golden.mjs`
  orchestrates (kept separate to stay under the ~400-line cap).
- `fixtures/legacy/ei7-grid.mjs` — the EI7 field-type tables and the
  grid child-dataset builder, extracted from `ei7-project.mjs` (same
  ~400-line cap).
- `fixtures/legacy/ei3-project.mjs` — the EI3 mapper, same reason.
- `fixtures/legacy/emit.mjs` — dataset.json v2 writing and the
  canonical dataset hash mirroring `epiora-core`'s encoding.
- `fixtures/legacy/golden.test.mjs` — the Step-1 unit tests
  (`oswego_view_has_expected_columns`,
  `code_table_becomes_label_set`).
- `fixtures/legacy/compare-golden.mjs` — the cross-backend comparator
  the workflow runs over every EI7 sample.
- `fixtures/legacy/golden/reimport/SampleDB.db` and
  `fixtures/legacy/golden/reimport/produced-ids.json` — workflow
  outputs inside the Files block's `reimport/` tree, listed because
  they are binary/generated.
Supersedes: —

### v1.1 Task 7 — EI7 mapper, refresh diff, report entries (2026-11-23)

Decision: `epiora-legacy` maps Epi Info 7 Access/SQLite projects to a
`LegacyProject` (views as dataset-v2 `Dataset`s, grid/relate children
under `datasets`+`links`, programs and check code verbatim, plus the
ordered report) whose structure is byte-comparable to the Task 4
golden fixtures. `refresh_plan` diffs a remapped project against the
committed view datasets on `GlobalRecordId` via the new
`epiora_import::diff_datasets(previous, next, key)`.

Interface notes where the golden oracle superseded the Interfaces
block sketch:

- The plan's hidden `__recstatus` column is not emitted: the golden
  keeps the `RecStatus` metaField as a normal integer column in every
  view dataset, so the mapper does the same; `deleted_records` counts
  its `0` rows and they stay in the dataset.
- `ReportKind` carries the golden's full emitted vocabulary
  (`field_not_imported`, `field_imported_changed`,
  `field_column_missing`, `column_not_imported`, `table_not_imported`,
  `code_table_missing`, `view_data_missing`,
  `record_links_not_imported`, `legacy_layout_unknown`,
  `code_page_assumed`, `text_decoding`, `view_failed`) rather than the
  sketch's subset — the golden report JSON is compared verbatim, so
  every emitted kind must round-trip through `as_str()`.
- `detect` is layout detection only: it returns
  `SourceFormat::Ei7Access` when a `metaViews` table is present (the
  caller supplies the true backend format to `map_ei7`),
  `SourceFormat::Ei3Access` when `view*` tables exist without
  `metaViews`, and otherwise the `legacy_layout_unknown` error
  carrying the plain table list (Review Focus 2). The plan's enum has
  no `Plain` variant — a file with no view-definition tables is the
  `LayoutUnknown` error, never a mapped format.
- `LegacyError::ViewBroken` (`legacy_view_broken`) is never
  constructed by the mapper: a view that cannot map is one
  `view_failed` report entry while the other views keep mapping (the
  plan §7 error contract). The variant stays because the plan lists
  the code — `legacy_view_broken` is reserved for the caller.
- mdbtools' `mdb_date_to_tm` early-returns a zeroed `tm` for serials
  `< 0` or `> 1e6`, which strftime renders as the literal
  `"1900-01-00 00:00:00"`; the golden stores that text and normalizes
  it to `days_from_civil(1900,1,0) = -25568`. The mapper reproduces
  both halves, so out-of-range serials map to the degenerate date,
  never to missing.

Two `epiora-mdb` encoder behaviours track how the golden oracle is
produced:

- `harness.c` keeps the product bind size at 4 MiB
  (`mdb_set_bind_size(mdb, 4 * 1024 * 1024)`): stock `mdb-export`
  truncates bound text at `EXPORT_BIND_SIZE` (200000), so the golden
  was regenerated in #115 with an `mdb-export` built at the same
  4 MiB bind size (owner decision 1A, 2026-11-23). HDCaseForm's
  `CheckCode` reports the full 164 blocks and EPTB values stay
  byte-identical to the golden's CSV route.
- `eptb.c` emits the tag-2 code-page extension only for JET3 files:
  the extension documents which JET3 code page text was decoded with,
  JET4 text is UTF-8 by definition, and the golden only emits
  `code_page_assumed` for JET3.

Files outside the Files block:
- `crates/epiora-legacy/src/ei7/{emit,page}.rs` — `ei7/` splits to
  stay under the ~400-line cap: `emit.rs` is the `OutCol` → `Dataset`
  emitter (`out_columns`/`finish_dataset`), `page.rs` the page-table
  row indexes and `readField` port.
- `crates/epiora-legacy/src/ei7/fuzz.rs` — the `legacy_meta_any`
  harness (arbitrary bytes → generated meta/data tables → `detect` +
  `map_ei7`, `Ok` or typed `Err`), feature-gated as `fuzzing`; the
  fuzz target under `fuzz/` is a one-line call into it.
- `crates/epiora-legacy/src/batch.rs` — the EPTB decoder's `unwrap`s
  became `legacy_batch_invalid` so malformed batches are a typed
  error, never a panic (the fuzz target exercises them).
- `crates/epiora-legacy/tests/common/{mod,sources,encode,compare}.rs`
  — the shared test harness: the `mdb_source` `BatchSource` builder
  (Access via `epiora-mdb`), `SqliteSource`/`VecSource`
  `LegacySource` impls, the verbatim SQLite copy writer, the test
  EPTB encoder + meta-source builders and the golden-JSON compare
  helpers (split for the same ~400-line cap).
- `crates/epiora-legacy/fuzz/{Cargo.toml,Cargo.lock,.gitignore,
  rust-toolchain.toml,regressions/}` — the cargo-fuzz crate, same
  shape as the other three fuzz crates (corpus/target/artifacts
  ignored, `regressions/<target>/.gitkeep` committed, nightly
  2026-10-15 pin).
- `crates/epiora-import/src/lib.rs` — re-exports `diff_datasets`,
  `Diff`, `SchemaChanges` and `FieldChange` for `refresh_plan` and the
  refresh tests.
- `crates/epiora-mdb/c/harness.c` and `crates/epiora-mdb/c/eptb.c` —
  the two encoder behaviours above.
- `app/public/mdb/mdb.wasm` — the committed wasm module rebuilt by
  `crates/epiora-mdb/build-wasm.sh` (pinned emsdk 6.0.13) so it
  carries the eptb.c JET3-only tag-2 gate (`mdb.js` is
  byte-identical); the `legacy` workflow's rebuild-and-compare job
  requires it unchanged.
- `crates/epiora-mdb/tests/data/batches/{sample,chunk7}/` — the
  wasm↔native parity oracle regenerated by `cargo run -p epiora-mdb
  --bin dump-batches` (the command `app/src/legacy/mdb.test.ts`
  documents): the eptb.c JET3-only code-page change means the JET4
  Sample batches no longer carry the tag-2 `windows-1252` extension,
  so the committed references must come from the same encoder build
  as the committed module.
- `.github/workflows/engine.yml` — `legacy_meta_any` added to the
  60-second PR fuzz loop; `.github/workflows/fuzz-nightly.yml` — a
  `fuzz-legacy` job running it for 1800 s (Step 3's PR and nightly
  fuzz lists live in these workflows).
- `Cargo.lock` — resolves `arbitrary` (the `fuzzing` feature) and
  `rusqlite 0.41` (dev-only test SQLite source; 0.41 bundles
  `libsqlite3-sys` 0.39 / SQLite 3.54, which RUSTSEC-2026-0459
  requires — 0.34 resolved to the affected 0.32).
Supersedes: —

### v1.1 Task 4 — golden regenerated at the product 4 MiB bind size (2026-11-23)

Decision: owner decision 1A — fix the oracle, not the product.
Upstream `mdb-export` binds text/MEMO columns at its compile-time
`EXPORT_BIND_SIZE` = 200000 (`src/util/mdb-export.c`), so every bound
value over 199,999 bytes (bind size minus the NUL) was silently
truncated in the committed golden. `legacy-golden.yml` now rebuilds
`mdb-export` from the upstream mdbtools 1.0.1 release tarball (URL
pinned, SHA-256
`ff9c425a88bc20bf9318a332eec50b17e77896eef65a0e69415ccb4e396d1812`,
licence read from the tarball itself at run time: GPL-2.0 utilities
`COPYING`, LGPL-2.0 libmdb `COPYING.LIB`), patched to
`EXPORT_BIND_SIZE` 4194304 — the product reader's bind size
(`crates/epiora-mdb/c/harness.c`: `mdb_set_bind_size(mdb, 4 * 1024 *
1024)`) — and exports `MDB_EXPORT` into `$GITHUB_ENV` so
`golden.mjs`, `copy-tables.mjs` (the Access→SQLite copies) and
`golden.test.mjs` all call the patched binary via `legacy-read.mjs`.
`mdb-tables`/`mdb-schema`/`mdb-ver` still come from apt — they never
bind column data. Nothing under `crates/` is built or modified; the
golden stays independent of the product.

Producing run (all steps green, including the in-run second-run
byte-identical proof):
https://github.com/dorrabbkhan/Epiora/actions/runs/45240078404

Golden files changed by the regeneration (`git diff --stat` verbatim):

     fixtures/legacy/golden/ei7/HansensDisease/HansensDisease/project.json | 2 +-
     1 file changed, 1 insertion(+), 1 deletion(-)

Changed values, before → after:
- `check_code` block count for view `HDCaseForm`: 137 → 164. The
  underlying `metaViews.CheckCode` MEMO was truncated at 199,999
  bytes; a full read yields 271,820 bytes as exported.
- Nothing else: every dataset file, program `text_sha256`, label set
  and the `reimport/` tree are byte-identical. The only bound values
  over 200 KB in the corpus were this MEMO and OLE blobs, which
  `mdb-export` reads via `mdb_ole_read_full`, never the bound buffer.
- New-bound truncation check: the largest field the patched binary
  exports across every `.mdb` sample is 2,204,672 bytes (hex OLE in
  `HansensDisease` `metaImages`); no golden value is still truncated
  at 4,194,303 bytes (bind 4194304 minus NUL).

Ambiguities resolved:
- None — the owner notes scope this PR to the oracle fix only;
  `RecStatus` stays a normal integer column (owner item 3), and no
  `crates/`, `app/` or `spikes/` file is touched (fixture guard).

Files outside the Files block:
- None. `.github/workflows/legacy-golden.yml`,
  `fixtures/legacy/golden/**` and `docs/decisions.md` are all inside
  the Task 4 Files block.
### RUSTSEC-2026-0594 `cookie` 0.18.3 (2026-11-23)

Decision: bump `cookie` 0.18.2 → 0.18.3 in `app/src-tauri/Cargo.lock` via `cargo update -p cookie` (transitive through
`tauri`, `tauri-runtime` and `wry`). RUSTSEC-2026-0594 (private-cookie key reuse after fork) failed `cargo deny check`
in the `app` and `native-store` CI jobs on main and on every open PR (first seen on #113). It is the only lock change. No
`deny.toml` ignore was added. Owner chose a separate PR (#113 escalation, 2026-11-23), as with #103.
Supersedes: —

### v1.1 Task 8 — EI3 (Epi Info 3.5.x) mapper (2026-11-23)

Decision: `epiora-legacy::map_ei3` maps an Epi Info 3.5.x Access
project — one `view<Name>` field-definition table per view, `SOURCE`
rows naming the data table (`DATA\d*`) and code tables (`CODE*`),
fields with `PageNumber > 0`, `UniqueKey` record identity and
`RECSTATUS = 0` deleted rows — to a `LegacyProject` whose structure
equals the Task 4 golden `project.json` files for all twelve
`ei3_access` fixtures (seven under `golden/ei3/`, five JET4 `.mdb`s
under `golden/ei7/`). The module is a port of the independently
written `fixtures/legacy/ei3-project.mjs`; the Epi Info upgrade code
`Epi.ImportExport/Epi2000/` (`{Project,View,Page,MetadataDbProvider,
CollectedDataProvider,ProjectAnalyzer}.cs`) and
`Epi.ImportExport/ProjectUpgradeManager.cs` at
`4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6` (Apache-2.0, D2) is cited
in `ei3/mod.rs` as the plan's method source.

Interface notes where the golden oracle superseded the Interfaces
block sketch:

- `map_ei3` is `map_ei3(source, name: &str)`, not `map_ei3(source)`:
  the golden burns `opts.stem` into `project.name` and the
  `code_page_assumed`/`legacy_layout_unknown` report items, and an
  EI3 file carries no project-name table — viewless files (9 of the
  12 goldens) offer no in-file name at all. The caller passes the
  file stem, mirroring `map_ei7`'s caller-supplied `format`.
- `ei3::EI3_VALIDATED` (`pub const bool = true`) is the product-side
  record of the D12 verdict (Task 2: redistributable). The plan's
  conditional — D12 `unavailable` → every EI3 project's report gets
  `ReportEntry { Unvalidated, <project>, "Epi Info 3.5.x support is
  not validated against sample projects (D12)" }` — is implemented
  behind it; the test's dormant branch reads the verdict from
  `fixtures/legacy/samples/README.md` and asserts that entry instead
  of the golden comparison.
- `detect` moved from `ei7/mod.rs` to `lib.rs` (the Files block's
  `lib.rs` (`detect`) change): it routes between both mappers, so it
  lives in the crate root rather than under `ei7/`; behaviour is
  unchanged (`metaViews` → `Ei7Access`, else `view*` → `Ei3Access`,
  else `LayoutUnknown` carrying the plain table list).
- `deleted_records` counts `Number(RECSTATUS) === 0` with JS
  semantics: a missing, empty or whitespace `RECSTATUS` cell is `0`
  (`Number(null)`/`Number('')`), hence deleted — matching the
  golden's `data.filter(...)` verbatim.
- `check_code.text` stores the non-blank `Checkcode` field texts
  joined by a blank line (the golden pins only the `blocks` count —
  the number of field rows with a non-blank `Checkcode`, not the
  EI7 `BLOCK_RE` header count — so `text` keeps the raw corpus for
  v1.3's translator like Task 7 did).
- `legacy_id` is `""` (golden `null`): EI3 projects have no project
  GUID. `key_column` is `UniqueKey` (the plan's key column; EI3 has
  no `GlobalRecordId`).
- Mirror fields read their source through the *untrimmed* `Name`
  cell (`String(ci(srcField,'Name'))` in the golden), so a trailing
  space in a field name mirrors the same lookup miss the golden has;
  `MIRROR` columns land as `text` with the `field_imported_changed`
  report entry.
- The EI3 report emits none of `program_imported`,
  `check_code_stored`, `records_deleted` or `text_decoding`: the
  golden's report vocabulary for EI3 is `code_page_assumed`,
  `field_not_imported`, `field_imported_changed`,
  `field_column_missing`, `view_data_missing`, `code_table_missing`,
  `table_not_imported`, `legacy_layout_unknown` and (when D12 is
  unavailable) `unvalidated`, and the report JSON is compared
  verbatim.

Files outside the Files block:
- `crates/epiora-legacy/src/ei7/mod.rs` — `MapCtx::new` and
  `any_code_page` widened to `pub(crate)` so `ei3` reuses the shared
  map context; `detect`/`plain_tables` moved out to `lib.rs` (above)
  and `map_ei7` now calls `crate::plain_tables`. No behaviour change.
Supersedes: —

### v1.1 Task 9 — SQL Server one-time import on desktop (2026-11-23)

Decision: the desktop shell gains a `tiberius` SQL Server reader
(`app/src-tauri/src/legacy_sqlserver.rs`) that turns every project
table into EPTB v1 batches — the same intermediate form `epiora-mdb`
and the sqlite-wasm worker emit — so `map_ei7` maps a SQL Server
project once, as `SourceFormat::Ei7SqlServer`. Two Tauri commands
(`legacy_sqlserver_tables`, `legacy_sqlserver_batches`) take
`SqlServerConn` per call; credentials live only for that call, and
`SqlServerError` carries just a code (`sqlserver_connect` /
`sqlserver_auth` / `sqlserver_query`), so a rejected login cannot
echo host, user or password (spec §4.5, §10, §15.7). `SqlServerConn`
and `SqlServerAuth` deliberately derive neither `Debug` nor `Display`.
CI gate: `legacy.yml`'s `sqlserver` job runs SQL Server 2022
Developer edition as a service container (spec §13 — CI only, never
distributed), loads the verbatim `copy-tables.mjs --tsql` copy of
`Sample.mdb` with a pinned go-sqlcmd (v1.10.0, SHA-256 verified in
the job; MIT per the release's licence), and runs
`app/src-tauri/tests/sqlserver.rs`, which is env-gated so the
`native-store` job and local runs skip it without `#[ignore]`; the
job also sets `EPIORA_SQLSERVER_REQUIRED=1` so a missing
`EPIORA_SQLSERVER_HOST` fails the tests instead of passing silently
(fix round 1).

Interface notes where implementation superseded the Interfaces
block sketch:

- `tiberius` is **0.13** (0.13.0, crates.io 2026-11-20 — the latest
  release; the owner prefers the latest version of a tool over an
  older pin. Fix round 1 moved off the task-time 0.12.3 pin).
  Features `tds73` + `rustls` + `winauth` (no OpenSSL; `winauth` is
  `cfg(windows)` inside tiberius). The `rustls` feature resolves to
  tokio-rustls 0.26 / rustls 0.23 / rustls-webpki 0.103, replacing
  0.12's tokio-rustls 0.24 / rustls 0.21 / rustls-webpki 0.101 and
  its RUSTSEC-2026-0104 / RUSTSEC-2026-0382 advisories. 0.13's
  `trust_cert`, auth constructors, row streaming, `ColumnData` and
  the time/numeric types are API-compatible for this reader —
  `legacy_sqlserver.rs`/`eptb.rs` needed no changes; the new
  `handshake_timeout` (15 s) / `command_timeout` (30 s) defaults
  bound only a stalled server round-trip, not a lazy row stream.
  Resolved (fix round 2, owner decision A — PR #119 comment
  7118845674): `rand 0.7.3` via `winauth 0.0.5` (`cfg(windows)`-only,
  required for `AuthMethod::Integrated` on Windows builds, used for
  the NTLM client nonce) is flagged by RUSTSEC-2026-0097 (unsound;
  the trigger needs a custom `log` logger, which the app does not
  install). `winauth` pins `rand = "0.7"` and 0.0.5 is the latest
  release — no safe upgrade exists, so the owner approved adding
  RUSTSEC-2026-0097 to `app/src-tauri/deny.toml`'s `ignore` list
  (the recorded v0.3 Task 14 pattern) and keeping Windows
  integrated auth.
- `SqlServerAuth::Windows` resolves to `AuthMethod::Integrated` on
  Windows builds only; elsewhere it is a typed `sqlserver_auth`
  failure (tiberius gates `Integrated` on `all(windows, "winauth")`).
- The batch `source_type` byte carries the TDS `ColumnType`
  discriminant (the sqlite test source uses `0`; the mapper compares
  it only for cross-chunk consistency, never for mapping decisions).
- T-SQL type → EPTB kind: ints `int1/2/4/8`, `bit` → bool,
  float/money/decimal → real, the `datetime` family → Access
  serial-days (`datetime`/`smalldatetime` +2, `date`/`datetime2`
  −693593; `smalldatetime`'s wire time part is *minutes* — the
  tiberius field name is misleading), char/guid/xml → text,
  binary/image/udt/variant → binary-skipped. Verbatim T-SQL copies
  hold datetimes as `NVARCHAR` text, the SQLite-proven path.
- `sqlserver_tables` lists `sys.tables WHERE is_ms_shipped = 0 ORDER
  BY object_id` — user tables in creation order, mirroring
  `mdb-tables`/`sqlite3 .tables`.
- Row order is split by table kind, both halves verified against a
  local `mcr.microsoft.com/mssql/server:2022-CU28-ubuntu-22.04`
  container loaded with the verbatim `copy-tables.mjs --tsql` copy
  of `Sample.mdb` (93 tables). The mapper emits rows in source
  order and JET's physical order is not key order (`epiora-mdb`
  shows `ADDFull.UniqueKey` starting 247, 277, 61…), so ordering by
  any key column permutes joined cells and breaks
  `sqlserver_copy_matches_access_hashes`.
  - `meta*` tables: `ORDER BY` their first column. In the mdb every
    `meta*` table's first column is a monotonic id in file order
    (`metaViews.ViewId` 1…18, `metaFields.FieldId` 1…534,
    `metaPages.PageId` 1…27; verified via `epiora-mdb` on all 17
    `meta*` tables), and for a real EI7 SQL Server project the
    first column is the id EI7 enumerates by. Physical order cannot
    be trusted here: `meta*` rows are variable-size (CheckCode up
    to ~13 KB) and SQL Server's heap backfills earlier pages — the
    container shows `metaViews` rows 15, 17, 18 backfilling
    page 896 while rows 14 and 16 each occupy later pages — so
    physical order ≠ insert order for these tables, and insert
    order is unrecoverable from a heap (`%%xact_seqno%%` is not
    valid syntax; `%%seqno%%` is per-page).
  - every other table: `CROSS APPLY
    sys.fn_PhysLocCracker(%%physloc%%) ORDER BY plc.file_id,
    plc.page_id, plc.slot_id` — physical slot order, which is
    insert order for the verbatim heap copy (verified on all 76
    non-`meta*` tables: 58 with a unique first column — including
    `EvansCounty7`, `hmohiv`, `hmohiv8`, `LEUKEM210` re-checked
    after a parse artifact — 15 by a unique secondary column,
    `Addicts`/`Anderson` all-REAL fixed-size, `SohoDead` by a
    full-row `ADDRESS` diff), the `mdb-export` dump order the
    golden's rows follow, and leaf order on a clustered real
    project — no key-column assumption.
    A bare `ORDER BY %%physloc%%` is wrong: the rid's little-endian
    byte packing scrambles tables spanning multiple pages.
  Verified offline: the pinned mdbtools `db.columns`/`db.rows`
  copy of `Sample.mdb`, run through the same declared-type kind
  mapping and this split ordering, maps to all 18 golden view
  hashes and all grid hashes identical.
- SQL Server projects are one-time imports (not refreshable): the
  reader emits no snapshots and no refresh machinery is wired —
  refreshability arrives with Task 15's import flow, which calls the
  same `map_ei7` API the tests exercise.

Dependencies added beyond the plan's named `tiberius, tokio-util`:
`tokio` (`net` — `TcpStream` for `Client::connect`),
`futures-util` (`std` only — `TryStreamExt` to iterate tiberius's
row stream), `serde` (`derive` — the `SqlServerConn`/`SqlServerAuth`
IPC shape), all licences inside the existing `deny.toml` allow list.
`app/src-tauri/deny.toml` gains the owner-approved RUSTSEC-2026-0097
ignore recorded above (fix round 2); `cargo deny check` passes.
Licences recorded: tiberius MIT/Apache-2.0; tokio/tokio-util MIT;
futures-util MIT OR Apache-2.0; serde MIT OR Apache-2.0; go-sqlcmd
MIT.

Files outside the Files block:

- `app/src-tauri/Cargo.lock` — the mechanical lockfile update for the
  dependency additions above.
- `app/src-tauri/src/lib.rs` — **created** (the Files block says
  "Modify"; no lib target existed). It exists so integration tests
  can reach `legacy_sqlserver`; it carries the workspace's usual
  `unwrap`/`expect`/`panic` clippy denies.
- `app/src-tauri/src/legacy_sqlserver/eptb.rs` — the EPTB encoder half
  of the reader, split out so `legacy_sqlserver.rs` stays inside the
  ~400-line file-size rule; the Files-block file is the module root.
- `app/src-tauri/src/commands.rs`, `app/src-tauri/src/main.rs` —
  the Files block's "register command" lands here: `main.rs` owns the
  `invoke_handler!` list, `commands.rs` holds the two `#[tauri::command]`
  wrappers that translate `SqlServerError` to the bare code string.
- `app/src/tauri/sqlserver.ts` — new file under the "Modify:
  `app/src/tauri/*`" glob (the per-command bridge wrapper).
- `.github/workflows/legacy.yml` — the `pull_request` `paths:` list
  also gained `app/src-tauri/**` and `app/src/tauri/**` so the job
  gates the files it covers.
- `app/src-tauri/deny.toml` — the owner-approved RUSTSEC-2026-0097
  ignore above (fix round 2); the Files block's `deny.toml` is the
  root file, which stays unchanged.
Supersedes: —

### v1.1 Task 10 — Epi Info 7 expression language (`epiora-lang`) (2026-11-23)

Decision: `parse_expr`/`type_check`/`eval_column` port
`Epi.Core.Interpreter/Rules/` at
`4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6` (Apache-2.0, D2) — the
grammar's `Expression → … → Value` precedence chain as a hand-rolled
lexer + recursive-descent parser, the interpreter's eval domain as
`EV`, and all 46 `supported@1.1` register functions in
`src/functions/`. Every number below matches a `Rule_*.cs` branch or a
`fixtures/ei7/v11/*` pin.

Semantic choices where the source was pinned (quirks ported, not
"fixed"):

- Two missing kinds: a missing *cell* reads as `""` (the
  `CurrentDataRow` → `DBNull` → `""` path in `Rule_Value`), the `(.)`
  literal is `null`; `Util.IsEmpty` covers both. `select_sort.json`
  pins the result: `<> 1` selects the missing row (38 records),
  `= (.)` selects it (1 record).
- Comparisons port `Rule_CompareExp` branch-for-branch, including the
  reversed `TryValue.CompareTo(LHSO)` on the `RHSO is string` arm
  (`5 < "10"` is false in EI7), the `RHSO is Boolean && =` numeric
  arm, `CurrentCultureIgnoreCase` approximated by case-folded
  ordinal, and `Time` as `TimeSpan` (same-type only with `Time`;
  `Date`/`DateTime` share the `DateTime` family).
- Logic operators are string-truthiness (`ToString() == "true"`); our
  results stay `Bool` — the `"true"`/`"True"` spelling differs only
  inside EI7's own `ToString()`, never in a comparison.
- `AddExp` has no `TimeSpan` arms: `time ± x` is missing; `MultExp`
  measures `Time` in `TotalDays`; `PowExp`/`NegateExp` run
  `double.Parse`/`TryParse` on `ToString()` (text parses, dates are
  missing). `type_check` is strict where EI7's branch set leaves
  `null` (dates beside `+`, `Time` beside `±`, non-numeric under
  `* / MOD`): an `expr_type` error naming the column, per the plan's
  "type errors name the column" test — an early error EI7 only finds
  at eval.
- `YEARS` is the birthday-age formula, not `MONTHS/12`. `HOURS` is
  `Math.Round(TotalHours)` (half-even), `MINUTES`/`SECONDS` raw
  doubles. `EPIWEEK` ports `GetMMWRStart` over three candidate years.
- `TXTTONUM` ports the two-step quirk: a `"true"/"false"` `ToString()`
  is overwritten to 1/0 *before* `TryParse` runs, then `TryParse`
  writes the parsed value or **0.0** — `TXTTONUM("true")` is `1`,
  `TXTTONUM("false")` is `0`, `TXTTONUM("abc")` is `0`
  (`Rule_TxtToNum.Execute` @ 4cd353c, lines 32–44; corrected in fix
  round 1). `SUBSTRING` is 1-based with
  `int.Parse` (reals fail). `FINDTEXT` is
  `IndexOf(OrdinalIgnoreCase)+1`. `STRLEN` requires a real string.
- `RND` draws from a `ChaCha20Rng` stream seeded by the recipe as-of
  plus the expression's canonical text, using the same u32 Lemire
  `draw_bounded` as `epiora-core`'s `random_numbers` (hard rule 7 —
  EI7's `random.Next` replayed deterministically).
- `ENVIRON`/`EXISTS`/`FILEDATE` resolve through `EvalEnv.globals`
  under the reserved keys `environ:<name>`, `exists:<path>`,
  `filedate:<path>` — the register's "map captured into the recipe at
  session start, never live I/O" method choice. Missing keys give
  missing values.
- `PFROMZ`/`ZSCORE` are in the table but evaluate to EI7's own
  `ArgumentException` ("no longer supported by Epi Info"), surfaced
  as `unsupported_function` — the register marks them
  `supported@1.1` (they parse), which is what the register-iteration
  test asserts.
- `FORMAT` calls `Microsoft.VisualBasic.Strings.Format` in EI7; the
  v1.1 corpus pins `FORMAT(x, "0.00")` → `x.ToString()` ("54",
  "46.1"), so non-date patterns return the `ToString()`. The named
  date/time formats (`GeneralDate`, `LongDate`, `ShortDate`,
  `LongTime`, `ShortTime`) are applied as a method choice — the
  corpus pins none of them.
- `GROUPROWINDEX`'s carry is the previous row's *expression result*
  (`PreceedingDataRow[qualifiedId]`): equal consecutive argument
  `ToString()`s increment the previous output's `int.TryParse`,
  else 1. `LAGVALUE` is the previous row's argument `ToString()`.
- `AND`/`OR`/`XOR`/`NOT` produce `Bool`, not EI7's `"true"/"false"`
  strings — the eval domain keeps the truth value; `ei_to_string`
  renders `True`/`False` (en-US .NET spelling).

Approximations (recorded, not silent): `CurrentCultureIgnoreCase` ≈
case-folded ordinal; `CurrentCulture` ≈ ordinal on trimmed strings;
the `*`→`.*` LIKE glob is hand-rolled (no regex dep); `TXTTODATE`'s
`DateTime.TryParse` accepts the en-US shapes the corpus exercises
(`M/D/YYYY`, `M-D-YYYY`, ISO `YYYY-MM-DD`, each with an optional
`h:mm[:ss] [AM|PM]` tail; a bare `h:mm` lands on the as-of date where
EI7 uses `DateTime.Today`); `G15` formatting is an en-US
approximation of `double.ToString()`.

Files outside the Files block:
- `crates/epiora-lang/src/expr.rs` — module root beside
  `src/expr/{lexer,parser,ast,types,eval,missing}.rs`; the Files
  block's `src/expr/*.rs` list lands the same files under this
  layout.
- `crates/epiora-lang/src/expr/{lit,ops}.rs` — the ~400-line rule
  splits: `lit.rs` takes the digit-run scanners producing the
  `Date`/`Time`/`Dec`/`Real` literal tokens out of `lexer.rs`;
  `ops.rs` takes the `Rule_CompareExp`/`AddExp`/`MultExp`/`ConcatExp`
  value-operator ports out of `eval.rs`, which keeps the node
  dispatch. No behaviour change.
- `crates/epiora-lang/src/functions/spec.rs` — the static `FnSpec`
  table/`ArgKind`/`function_spec` split out of `functions/mod.rs`
  (same rule); `types.rs` reads it through `mod.rs`'s re-export.
- `crates/epiora-lang/Cargo.toml` dev-deps add `proptest` and
  `serde_json` (register.json iteration test); `rand_chacha` +
  `rand_core` match epiora-core's versions (MIT OR Apache-2.0,
  inside `deny.toml`'s allow list).
- `Cargo.lock` — the mechanical lockfile update for the new
  workspace member.
- `crates/epiora-lang/tests/expr.proptest-regressions` — proptest's
  own saved-seed file (it asks to be checked in); the recorded seed
  predates the non-negative-literal generator fix and now replays
  harmless cases.
- `crates/epiora-lang/src/functions/calendar.rs` — fix round 1: the
  civil-calendar (`days_from_civil`/`civil_from_*`/
  `secs_from_civil_checked`) and MMWR (`mmwr_start`, `day_of_week`,
  `days_in_month`) helpers out of `date.rs` for the ~400-line rule;
  `epiweek` itself stays in `date.rs`. No behaviour change.
- `crates/epiora-lang/tests/expr_props.rs` — fix round 1: the two
  proptests and their strategies out of `expr.rs` (same rule);
  `sample()`/`env()` are duplicated so each test file is
  self-contained. No test removed or weakened.
- `crates/epiora-lang/tests/expr_props.proptest-regressions` — fix
  round 1: the saved-seed file renamed to follow its test target
  (proptest names it after the test file).
Supersedes: —
### v1.1 Task 11 — cleaning commands, working view, recipes, receipt v4 (2026-11-23)

Decision: `src/manage/*` ports EI7's Analysis cleaning commands
against the Task 3 corpus (`fixtures/ei7/v11/*.json`); `view.rs` is
the `WorkingView` (dataset + selection + order + settings) the
Interfaces pin; `recipe.rs` canonicalises a `Command` sequence into
stable text + SHA-256 for the receipt's `WorkingViewStamp`; receipt
v4 stamps `working_view` iff one is supplied.

Semantic choices where the corpus or EI7 source pinned the answer:

- `Command::DefineExpr { name, expr, scope }` — an extra variant
  beside the pinned `Define{name,ty,scope}`: EI7's `DEFINE x = e`
  form declares a column and immediately assigns, and the corpus pins
  it producing a zero-initialised column (record-path assign, not the
  TEXTINPUT path). `Define{name,ty}` keeps the declared-type-only
  form.
- `ASSIGN` into a `DEFINE`d column coerces the eval result into the
  declared type (`retype_into`): EI7 cannot change a declared
  variable's type, so `DEFINE P NUMERIC` + `ASSIGN P = 42` stores
  Real `42.0`, not Integer.
- TEXTINPUT + immediate ASSIGN uppercases (EI7's "Program Specific
  Feature": an ASSIGN whose RHS has no field reference runs on the
  immediate path that applies the input format); record-path ASSIGNs
  (field refs present) keep the value's case. Corpus:
  `functions_text_math` `SICKTXT` → `YES` (immediate) vs `SUBNM` →
  `Doe` (record-path).
- MERGE name-copies shared columns into the *current* dataset,
  last-wins on duplicate keys, inserts leave dest-only columns
  missing; a re-READ sees the merged table. UPDATE/APPEND modes are
  implemented but NOT corpus-pinned (EI7 crashes on the option word;
  owner decision 2026-11-23, method choice deferred to Task 14) —
  covered by hand-computed unit tests.
- `deleted_records_included` is a sticky view flag: SET PROCESS
  DELETED/BOTH toggles which rows the view counts, surviving CANCEL
  SELECT (delete_set.json pins both directions).
- Corpus-comparator adaptations (test-side only, `ei7_v11.rs`): EI7's
  pinned `freq.total` is the displayed non-missing total while the
  engine's `total` counts in-scope rows; and EI7 lists the missing
  category first where the engine appends it — the runner reorders
  and recomputes the cumulative percent in the new order.

Files outside the Files block:

- `crates/epiora-lang/src/fuzz_harness.rs` + the `fuzzing` feature and
  `arbitrary` dep in `crates/epiora-lang/Cargo.toml` (arbitrary 1.5,
  MIT OR Apache-2.0, inside deny.toml's allow list) — the harness the
  Files-block `fuzz_targets/recipe_any.rs` calls, matching the
  epiora-mdb fuzz crate's layout.
- `crates/epiora-lang/fuzz/{Cargo.toml,Cargo.lock,.gitignore,
  rust-toolchain.toml}` — cargo-fuzz crate boilerplate pinned like
  the mdb crate (nightly-2026-10-15).
- `crates/epiora-lang/src/lib.rs` — module wiring, the Interfaces'
  `merge_key_type`/`relate_no_link`/`recipe_replay_mismatch`
  `LangError` variants, and the public re-exports.
- `Cargo.toml` (root) — `crates/epiora-lang/fuzz` in `exclude`;
  `Cargo.lock` — the mechanical lockfile update for `arbitrary`.
- `crates/epiora-wasm/bindings/{WorkingViewStamp.ts,Receipt.ts,
  RunContext.ts}` — regenerated by `cargo test -p epiora-core
  --features ts export_bindings`; the ts-rs output of the receipt
  v4/`RunContext` changes.
- `app/src/store/{opfs,relay,stdio}.ts` — migration 0005 wired into
  the three MIGRATIONS arrays; `app/src/store/repo/index.ts` —
  `sessions.ts` export (the repo module itself is the `repo/*`
  create the glob covers).
- `app/src/store/conformance/{migrations.ts,suite.ts}` — the
  `schema_version` assertions move 4→5 alongside the new
  `migration_0005_on_both_backends` test.
- `.github/workflows/{engine.yml,fuzz-nightly.yml}` — Step 3's "add
  `recipe_any` to PR and nightly fuzz lists": the PR fuzz loop and a
  `fuzz-lang` nightly matrix job, both on the pinned nightly.
- ~28 `RunContext` literals across `crates/epiora-core/{src,tests,
  benches}` (`explain/*`, `corpus/*`, `fuzz_harness.rs`, every test
  file) — each gained `working_view: None` for the Interfaces-pinned
  new field. No behaviour change; `receipts_v1_v2_v3_golden_unchanged`
  verifies the v1–v3 canonical bytes.
- `crates/epiora-core/src/receipt/canonical.rs` is listed in Files
  but unmodified: version 4 flows through `receipt/mod.rs` and the
  `skip_serializing_if` field keeps v1–v3 byte-identical.

Interface and manifest notes (fix round 1, 2026-11-24):

- `crates/epiora-lang/Cargo.toml` `[dependencies]` additions:
  `serde` 1.0.229 and `serde_json` 1.0.154 with `float_roundtrip`
  (both MIT OR Apache-2.0) — the crate's public types serialize for
  the session store and the worker boundary (Task 12's
  `Statement::Procedure { options: serde_json::Value }` /
  `Action` variants and Task 13's lang-wasm `*Json` surface are
  already pinned to `serde_json::Value`); `sha2` 0.10.9 (MIT OR
  Apache-2.0) — `Recipe::sha256_hex` hashes the canonical text into
  the receipt stamp's `recipe_sha256`. The same `serde`/`serde_json`
  pairs also sit in `[dev-dependencies]` for the corpus runner's
  fixture deserialization.
- `[dev-dependencies]` additions `epiora-legacy` and `epiora-mdb`
  (path crates, workspace AGPL-3.0-only): `tests/common/corpus`
  opens the real `Sample.mdb`/`Nutrition.mdb` and decodes their raw
  batches for `every_reference_program_step_matches`.
- `WorkingView.key_vars: Vec<String>` is a `pub` field beyond the
  Interfaces' pinned 4-field struct `{dataset, selected, order,
  settings}`: `Command::KeyVars` declares the view's key columns and
  EI7 keeps that declaration on the view for later keyless
  MERGE/RELATE to consult, so the view has to carry it (the pinned
  `settings` field is `ViewSettings`, not a place for key names).
- `crates/epiora-lang/clippy.toml` (new): a copy of
  `crates/epiora-core/clippy.toml`'s `disallowed-methods` list — the
  libm-only float-maths rule (spec §3.4, E1) applies to the lang
  crate as it does to core. It drove the `var.sqrt()` →
  `libm::sqrt(var)` fix in `manage/summarize.rs`; the corpus
  runner's `check_results` keeps `10f64.powi` under a scoped
  `#[allow(clippy::disallowed_methods)]` because it computes the
  comparison's expected tolerance.
- Test-file splits (every named test kept verbatim; the ~400-line
  file constraint): `tests/ei7_v11.rs` keeps only
  `every_reference_program_step_matches`; its runner moved to
  `tests/common/corpus/{mod,fixture,session,parse,run,compare}.rs`.
  `tests/manage.rs` (804 lines) became
  `tests/manage_{define_recode,merge_relate,view}.rs` by command
  group; `tests/recipe.rs` (504) shed the receipt tests into
  `tests/receipt.rs`. Shared builders (`ds`, `cells`, `Links`,
  `recipe`, `ctx` …) live in `tests/common/{mod.rs,ds.rs}`.
Supersedes: —

### v1.1 Task 12 — `.pgm` parser, planner and register (2026-11-24)

Decision: `src/pgm/*` parses Analysis `.pgm` text into `Program` (one
`Spanned<Statement>` per content line), embeds the Task 1
`register.json` as `pgm::register()`, and `plan`/`plan_with` map
statements to pinned `Action`s — RUNPGM resolves through a
`ProgramSource`, inlining the callee's actions with their origin
lines; `print_command` is the canonical Command printer used by
recipes and dialogs.

Semantic choices where the grammar or the pinned interfaces left a
gap:

- `WriteTarget`/`DisplayWhat`/`ProgramRef` (the Interfaces'
  "implementer's own"): `WriteTarget { mode: WriteMode, format:
  String, target: String, vars: WriteVars }` — `WriteMode` is
  `Replace`/`Append` (REPLACE is the register's `*` default output
  method), `format` keeps the spelled output form (`"Epi 2000"`,
  …), `target` is `table` or `file:table`, and `WriteVars` is
  `All` / `Except(vars)` / `Only(vars)`. `DisplayWhat` is the
  register's displayable views: `DbVariables { fields }`,
  `DbViews`, `Tables`, `Recipe`. `ProgramRef` keeps RUNPGM's three
  spellings: `Named(name)`, `File(path)`, `FileProgram { file,
  name }`.
- A found RUNPGM emits `Action::Note "RUNPGM <key>"` at the caller's
  line then the inlined actions; a missing program emits only
  `Action::Unsupported{command:"RUNPGM <key>", target_version:None}`
  (the Interfaces' "a missing program is an Unsupported action naming
  it"). A cycle is `LangError::RunPgmCycle` (`pgm_runpgm_cycle`).
- `may_differ` becomes true at the first `Unsupported` action whose
  register entry has `affects_data: true` and resets on `Action::Read`
  or `Action::Write` (WRITE re-reads its target — a new working view
  re-establishes what procedures report on); `List` does not reset it.
- QUIT/EXIT end the plan with a `Note` at their line; statements
  after them are parsed but never planned. Output verbs the register
  marks implemented-but-display-only (TYPEOUT/HEADER/ROUTEOUT/
  CLOSEOUT/NEWPAGE/PRINTOUT/SYSINFO/ABOUT/BEEP, `DISPLAY TABLES`) —
  actually every display-only verb — become `Note` actions carrying
  the line text; `display_options`/`output`-family verbs that would
  route results stay `Unsupported`.
- `DEFINE x = e` produces `Command::DefineExpr` (Task 11's added
  variant); `DEFINE x <TYPE>` stays `Command::Define`. `SET` clauses
  named `(+)`, `(-)`, `(.)` and the display-only options are `Note`
  actions, not `SetOption`s.
- Input decoding is UTF-8 else Windows-1252 (`decode_pgm_text`);
  CRLF and bare CR both split lines. `*` as the first non-blank char
  is a comment; `***` mid-line ends the statement's text. Literals
  reuse `expr::lit::{lex_number,number,real}` so the date/number
  grammar is identical to expressions.
- SUMMARIZE aggregates accept both `VAR` and `VARIANCE` spellings
  (the register prints `Variance`; the corpus's reference `.pgm`s
  use both) — both map to `AggFunc::Var`.

Files outside the Files block:

- `crates/epiora-lang/src/recipe.rs` — the canonical printer moved
  to `src/pgm/print.rs` (the Interfaces pin `print_command` in the
  pgm module "used by recipes and dialogs"); `recipe.rs` delegates.
  `canonical_text_is_stable`/`replay_reproduces_view_hash` still pin
  the byte-exact text.
- `crates/epiora-lang/src/fuzz_harness.rs` — `pgm_any` harness
  (decode → parse → plan; panic-freedom, `pgm_syntax` codes only,
  non-empty unsupported names).
- `crates/epiora-lang/fuzz/Cargo.toml` — the `[[bin]]` for
  `pgm_any` (conductor note: record explicitly).
- `.github/workflows/{engine.yml,fuzz-nightly.yml}` — Step 3's "add
  `pgm_any` to PR and nightly fuzz lists": `pgm_any` joined the
  existing `recipe_any` matrix entries on the pinned nightly.
- `src/pgm/parser.rs` is the Files-block entry point (line infra,
  dispatch, shared helpers); the per-command parsers needed their
  own modules under the ~400-line rule:
  `src/pgm/parser/{commands,manage_stmt,helpers,blocks}.rs` plus
  `src/pgm/parser/blocks/{recode,summarize}.rs` and
  `src/pgm/parser/manage_stmt/set.rs`. Largest file: commands.rs at
  392 lines.
- `tests/pgm/{plan,print}.rs` — the ~400-line split of
  `tests/pgm.rs` (all 11 named tests kept verbatim: the fixture /
  decoding / syntax-error tests in the root, planner tests in
  `plan.rs`, `print_parse_roundtrip` proptest in `print.rs`).
- `tests/pgm/print.rs` proptest domain note (test-side only):
  `Value::Date` is generated with days ≥ -683003 — civil years < 100
  are unspellable through the 2-digit-year pivot in
  `parse_date_text` (`99` → 1999), so round-trip generation is
  restricted to representable dates.
Supersedes: —
