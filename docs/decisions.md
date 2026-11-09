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
