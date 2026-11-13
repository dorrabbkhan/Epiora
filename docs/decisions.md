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
- Files outside the Files block: `docs/decisions.md` (this entry, as the
  owner note requires); `.github/workflows/app.yml` (CI repair — the
  `app` job pinned `ubuntu-latest` → `ubuntu-24.04` because
  ubuntu-latest moved to 26.04 resolute on 2026-11-13 and Playwright's
  WebKit build needs `libicudata.so.74`, absent on resolute; the
  store-opfs suite and Playwright e2e fail at browser launch on every
  PR until reverted or Playwright supports resolute).
