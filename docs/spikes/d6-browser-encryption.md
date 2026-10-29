# D6 — Encryption in the browser

**Status:** go

## Question

Plan Task 7, resolving spec §17 open decision D6 ("Whole-database
encryption in the browser — open SQLCipher wasm build or field-level
only") and exercising Review Focus 2: does WebCrypto field-level
encryption work as §11.1 specifies, can an open SQLCipher wasm build
keep a whole database encrypted on OPFS (marker bytes absent from raw
file, wrong key rejected), and is a browser without OPFS reported
cleanly rather than thrown?

## Method

- `spikes/browser-crypto/page.html` implements both produced interfaces:
  - `window.fieldCrypto.encrypt(passphrase, plaintext)` /
    `.decrypt(passphrase, blob)` — WebCrypto AES-GCM 256, key derived
    per field with PBKDF2-SHA256 at 600,000 iterations and a fresh
    random 16-byte salt; blob = base64(salt‖iv‖ciphertext+tag) with a
    12-byte random IV (spec §11.1, plan Task 7 interface).
  - `window.dbCrypto.probe()` — opens a SQLCipher database
    (`epiora-d6.db`) in the wasm module's in-memory FS, writes the marker
    `EPIORA_PLAINTEXT_MARKER`, exports the encrypted file bytes and
    persists them to OPFS; returns `{ opfs, encrypted, error }`. When
    `navigator.storage.getDirectory` is missing or rejects, it reports
    `opfs:false` with an `error` naming the fallback instead of throwing.
    `dbCrypto.reopen(key)` is a spike-internal helper used by the
    "reopen with wrong key fails" test (not part of the produced
    interface).
- SQLCipher wasm build: `@7mind.io/sqlcipher-wasm` v1.4.0
  (`7mind/sqlcipher-wasm`), downloaded from the GitHub release asset
  `sqlcipher-wasm.zip` pinned by SHA-256
  (`a2bc6de7…ce10cb7`) plus `lib/sqlite-api.mjs` and `LICENSE` from tag
  `v1.4.0`, both SHA-256-pinned — `vendor/fetch-vendor.sh` verifies all
  three. The ES-module build embeds SQLCipher 4.9.0 + OpenSSL 3.3.2
  (per its README; AES-256-CBC pages, PBKDF2-HMAC-SHA512, HMAC-SHA512).
  The build has no OPFS VFS — the database lives in Emscripten MEMFS
  and is exported to OPFS per persist, so every write copies the whole
  encrypted file.
- `spikes/browser-crypto/crypto.spec.mjs` (Playwright,
  `channel: 'chrome'`) serves the repo over loopback HTTP and runs the
  five plan checks plus a `?bench=1` measurements test that writes
  `crypto-results.json`: 1,000 sequential field encrypts and decrypts,
  and 10,000-row `INSERT` + serialize + OPFS-write for sqlcipher-wasm
  and for plain `@sqlite.org/sqlite-wasm` (3 reps each, medians).
- Dependencies install from the committed `package-lock.json` with
  `npm ci` inside `spikes/browser-crypto`.

## Environment

- Local: Ubuntu resolute x86-64 (kernel 7.0.0-1015-oem), Node.js
  v24.22.0, Google Chrome (stable channel — no Playwright browser
  download), Playwright 1.64.1.
- CI: `ubuntu-latest`, `actions/setup-node` `node-version: '22'` and the
  runner image's preinstalled `google-chrome-stable`; versions printed
  by the "Environment versions" step of
  `.github/workflows/spike-browser-crypto.yml`.
- Packages (from `spikes/browser-crypto/package-lock.json`):
  `@sqlite.org/sqlite-wasm` 3.54.1-build1 (SQLite 3.54.1),
  `@playwright/test` 1.64.1.
- RNG: none in measured code (rows and field values are deterministic);
  fieldCrypto salts/IVs come from `crypto.getRandomValues` — the CSPRNG
  the algorithm requires, not a data RNG (spec §7.4).

## Measurements

Every number links to the run that produced it. CI numbers are from the
`browser-crypto` workflow run cited below (step "Playwright checks +
measurements" output and the `crypto-results.json` artifact /
`crypto-results.json` block in the step summary). The identical local
command is:

    cd spikes/browser-crypto && ./vendor/fetch-vendor.sh && npx playwright test

Functional checks (CI run 40907412732, step "Playwright checks +
measurements"; the identical local command above passes all six):

- `field round trip`, `wrong passphrase rejects`,
  `encrypted db hides plaintext` (raw OPFS bytes of `epiora-d6.db`
  contain no `EPIORA_PLAINTEXT_MARKER` and no `SQLite format 3\0`
  header), `reopen with wrong key fails` (`file is not a database`,
  right key still opens), `no OPFS is reported, not thrown`
  (`{ opfs:false, error:"OPFS unavailable … fallback: keep the exported
  encrypted bytes in memory or IndexedDB" }`): all pass.

Measured numbers (CI run
[40907412732](https://github.com/dorrabbkhan/Epiora/actions/runs/40907412732)
on `ubuntu-latest` — `crypto-results.json` printed in the "Report
crypto-results.json" step and uploaded as the `browser-crypto-results`
artifact; all six checks passed in 3.0 m):

- Field crypto: 1,000 sequential encrypts = 85,783.5 ms total,
  85.3 ms/field median; 1,000 sequential decrypts = 85,157.8 ms total,
  85.1 ms/field median — i.e. ~85 s per 1,000 fields each way, dominated
  by the 600,000-iteration PBKDF2 derivation.
- Whole-DB write throughput (10,000 rows, one txn, + serialize + OPFS
  write, median of 3 reps): plain sqlite-wasm 14.4 ms
  (insert 9.6 / export 0.3 / persist 4.7), sqlcipher-wasm 13.6 ms
  (insert 9.4 / export 0.2 / persist 4.4); db bytes 188,416 vs 184,320.
- sqlcipher-wasm payload: `vendor/dist/sqlcipher.mjs` 4,271,260 bytes
  raw, 1,658,937 bytes gzipped (measured in-page via
  `CompressionStream`).

For comparison, the local `E2E=1 ./scripts/check.sh` run measured
74,984.3 ms / 75,294.1 ms per 1,000 fields and 11.2 / 12.2 ms db medians
on the machine above.

## Decision

field-level only — one of the two options the plan offers ("whole-DB in
the browser + field-level, or field-level only").

Reasoning, from the measurements: whole-DB encryption *works* end to
end (encrypted OPFS file, wrong key rejected, ~0 ms write overhead at
10k rows), but (a) the only ready-made open build keeps the database in
MEMFS and rewrites the entire encrypted file on every persist — no
page-level OPFS VFS, so durability between commits is a copy of the
last export and write cost is expected to grow with file size — an
inference from the MEMFS whole-file-export design, not measured beyond
10k rows); (b) it adds 1.66 MB gz
to first load (a third of the §12.8 5 MB budget) for a layer §11.1 does
not require — field-level encryption of identifying fields is already
mandated; and (c) shipping a third-party crypto binary raises the
supply-chain bar — the build is MIT and reproducible via its nix flake,
but a v0.3+ whole-DB requirement should instead weigh building
SQLCipher + an OPFS VFS from the upstream `sqlcipher/sqlcipher`
`ext/wasm` source, which this spike did not test.

## Consequences

- v0.3+ browser store: WebCrypto field-level encryption of identifying
  fields per §11.1 (PBKDF2-SHA256, 600k iterations, AES-GCM 256,
  salt‖iv‖ct blob — the interface this spike produced). Key-derivation
  cost (~75 ms/field here) means batch-derive or cache keys per
  passphrase/session where safe; the spec should state whether the
  600k-iteration cost is per-field or amortised.
- OPFS absence is a supported degraded mode: report `opfs:false` and
  fall back to in-memory/IndexedDB storage, never throw (Review
  Focus 2).
- If whole-DB encryption in the browser is ever required, prefer a
  self-built upstream `sqlcipher/sqlcipher` `ext/wasm` artifact (real
  OPFS VFS, same code path as `@sqlite.org/sqlite-wasm`) over a
  third-party MEMFS-export build; re-measure then.
- Tauri whole-database encryption stays with SQLite3MultipleCiphers per
  §11.1 — untouched by this decision.

## Licences

- `7mind/sqlcipher-wasm` v1.4.0 wrapper and release asset: MIT
  (`vendor/LICENSE`, fetched SHA-256-pinned from tag `v1.4.0`;
  `package.json` at the tag declares `MIT`). The downloaded zip states
  no licence itself, so the licence is taken from the tagged source the
  release was built from.
- SQLCipher 4.9.0 inside the build: BSD-3-Clause (Zetetic), per the
  upstream `sqlcipher/sqlcipher` LICENSE.
- OpenSSL 3.3.2 inside the build: Apache-2.0, per the project's README
  ("OpenSSL 3.3.2 compiled to WebAssembly").
- `@sqlite.org/sqlite-wasm` 3.54.1-build1: Apache-2.0, per the pinned
  package's `license` field in `package-lock.json`; the embedded SQLite
  core itself is public domain. Linked as an ES module; used only as the
  unencrypted comparison.
- `@playwright/test` 1.64.1: Apache-2.0 (devDependency, test only).
- Rejected licences respected: no GADM data, no SQLite Encryption
  Extension (SEE), no PowerSync (spec A4, A6, A7). The
  `@journeyapps/wa-sqlite` SQLite3MultipleCiphers wasm build was
  considered and passed over in favour of an actual SQLCipher build
  (spec §11.1 names SQLCipher for the browser) from a non-PowerSync
  vendor.
