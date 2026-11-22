# Releasing Epiora

This is the release checklist. Follow it top to bottom; every box must be
ticked before you publish. The helpers under `scripts/release/` automate
everything that can be automated — they are **read-only** towards GitHub
and Netlify: they never create a tag or release, never push, never
upload assets and never change settings. They print the commands; only
you (the owner) run the ones that publish anything.

Rules that apply throughout:

- **Only the owner** creates tags, publishes releases, deploys to
  production, merges PRs, changes rulesets/secrets or edits baselines
  (fixtures, thresholds, golden receipts).
- **The engine version is not the release version.** The release is the
  app version (`app/package.json` etc.). The root `Cargo.toml` workspace
  version (currently 0.2.x) is stamped into receipts via
  `env!("CARGO_PKG_VERSION")` and is covered by golden-receipt tests —
  do NOT bump it for a release; that is an owner-only baseline change.
- The `main` ruleset has no required status checks, so **CI green is
  verified by the checkpoint script**, not by GitHub.
- Everything the scripts write lands in `target/release/v<version>/`
  (gitignored). Never commit it.

---

## Phase 0 — one-time setup

Do this once, before the first release.

- [ ] **Tag ruleset bypass.** Settings → Rules → Rulesets → `release
  tags` → *Bypass list* must include an actor that covers you (e.g.
  Repository admin / Maintain). The ruleset restricts creation, update
  and deletion of `refs/tags/v*`; without a bypass the tag push that a
  published release performs is refused.
- [ ] **Netlify secrets exist.** Settings → Secrets and variables →
  Actions must contain `NETLIFY_AUTH_TOKEN` and `NETLIFY_SITE_ID`.
- [ ] **Netlify token is valid.** The token must not be expired — a dead
  token fails the deploy step *after* the release is already public.
  Mint a fresh token before release day if in doubt.
- [ ] **Netlify site settings** (site `epiora-dev`): *Build & deploy →
  Build settings* — builds are stopped/not linked to a repo (GitHub
  Actions deploys the finished `app/dist`); *Domain management → HTTPS*
  — HTTPS enforced.
- [ ] **Know who does what.** Owner: merges, tags, publishes, deploys,
  bypasses. Conductor: everything else, including writing the
  post-release `docs/decisions.md` entry via a PR.

## Phase 1 — automated pre-flight

- [ ] `git checkout main && git pull --ff-only`
- [ ] Run `./scripts/release/checkpoint.sh 1.0.0` and read
  `target/release/v1.0.0/checkpoint.md`. **0 FAIL is required to
  continue.** Every row means:

  | row | what it means | fix |
  |---|---|---|
  | `app version` | one of `app/package.json`, `app/package-lock.json` (top **and** `packages[""]`), `app/src-tauri/tauri.conf.json`, `app/src-tauri/Cargo.toml` `[workspace.package]` ≠ the version | fix the field in a PR; do not touch the root `Cargo.toml` (engine version, see above) |
  | `on main` / `clean tree` / `up to date` | wrong branch, dirty tree or behind origin | checkout/pull/commit |
  | `tag free` / `release free` / `draft` | `v1.0.0` already exists as tag or release | pick a higher version (e.g. `1.0.1`) or — owner decision — delete the draft |
  | `owner gates merged` | an `[OWNER GATE]`/`[OWNER REVIEW]` PR is still open | merge it (owner) |
  | `validation report` / `closing record` | the version's closing task is not merged | merge the closing PR |
  | `* CI` | the named workflow's latest green main run is stale — its path-filtered files changed since | wait for the run on HEAD; if it ran and is red, the release is blocked |
  | `perf gate` | the perf job failed | see the rerun policy below |
  | `desktop bundles` / `desktop smoke` | a desktop artifact is missing/expired or the Linux smoke failed | fix CI; artifacts expire after 90 days |
  | `secret NETLIFY_*` | the secret is missing (only reported when the token can list secrets) | add it (owner) |
  | `MANUAL …` | something only you can confirm | confirm it and move on |

- [ ] **Red perf policy (Task 17 owner decision).** A single red `perf`
  job on a main push is rerun twice (job rerun or
  `gh workflow run app.yml --ref main`); it counts as a real failure —
  and blocks the release — only if a rerun fails again. Runner variance
  alone (`import_1m` swung ~70 s vs ~100 s on identical code) is not a
  blocker.

The script also wrote `release-notes.md` (draft release body) and
`ci.env` (the verified run ids `collect-assets.sh` reads).

## Phase 2 — manual checks on the exact release content

Automated gates can't judge feel. Do this once per release on a preview
of the *exact* commit the tag will point at (the checkpoint's HEAD).

- [ ] **Get a preview of the release commit.** Once `main` HEAD is the
  release commit:
  `gh workflow run app.yml --ref main -f runs=1`
  (dispatch deploys a preview aliased from the branch name —
  `https://main--epiora-dev.netlify.app`; `-f runs=1` keeps the
  mandatory perf pass short — its verdict is irrelevant here). Or use
  the last merged PR's `pr-<N>--epiora-dev.netlify.app` preview if it is
  the same tree.
- [ ] **Sanity:** `python3 scripts/release/verify-deploy.py <preview
  URL>` — all PASS (the loopback-only redirect check reports SKIP on
  `127.0.0.1`).

### Browser (Chromium, then Firefox, then Safari/WebKit)

- [ ] First load renders; install as PWA works (install icon/prompt).
- [ ] Reload while offline (DevTools → Network → Offline) — the app
  still loads.
- [ ] Create a project; import `fixtures/import/utf8_comma/input.csv`
  through the wizard (preview → confirm).
- [ ] Run **FREQ** on a variable, then **TABLES** (e.g. AGE × NAME,
  levels alice/bob). Open the result's receipt — it shows the project
  and the engine version (`1.0.0` — the same number as the app this release, but a separate field).
- [ ] Reload — the output log survives.
- [ ] Open a second tab — the lock message shows.
- [ ] Switch themes, including high contrast; tab through the app
  keyboard-only to run a procedure.

### Desktop (install the bundles `collect-assets.sh` collected — unsigned)

- [ ] **Linux:** `chmod +x Epiora_1.0.0_amd64.AppImage && ./Epiora_1.0.0_amd64.AppImage`
  (or `sudo apt install ./Epiora_1.0.0_amd64.deb`).
- [ ] **Windows:** `Epiora_1.0.0_x64-setup.exe` → SmartScreen "Windows
  protected your PC" → **More info → Run anyway**.
- [ ] **macOS (arm64):** open `Epiora_1.0.0_aarch64.dmg`, drag to
  Applications → Gatekeeper warns (unsigned) → right-click → **Open**,
  or `xattr -dr com.apple.quarantine /Applications/Epiora.app`.
- [ ] On each: import the same fixture, run FREQ + TABLES, **restart the
  app — the data is still there**.
- [ ] **Encrypted at rest:** the store file `epiora.db` (Linux
  `~/.local/share/org.epiora.app/epiora.db`, macOS
  `~/Library/Application Support/org.epiora.app/epiora.db`, Windows
  `%APPDATA%\org.epiora.app\epiora.db`) must NOT start with the
  plaintext header `SQLite format 3\0`:
  `head -c 16 <path>/epiora.db | od -c | head -1` — ciphertext is
  expected.

## Phase 3 — create the release

- [ ] `./scripts/release/collect-assets.sh 1.0.0` — downloads the three
  desktop artifacts of the verified run into
  `target/release/v1.0.0/upload/` and writes `SHA256SUMS.txt`. It
  refuses if the checkpoint had FAILs or a bundle's filename version ≠
  `1.0.0`.
- [ ] **GitHub → Releases → Draft a new release**, values:

  | field | value |
  |---|---|
  | Choose a tag | `v1.0.0` — **Create new tag: v1.0.0 on publish** |
  | Target | `main` — confirm the commit shown is the checkpoint's `HEAD_SHA` (in `target/release/v1.0.0/ci.env`) |
  | Release title | `Epiora v1.0.0 — Calculators and MEANS` |
  | Description | paste `target/release/v1.0.0/release-notes.md` verbatim |
  | Attach binaries | everything in `target/release/v1.0.0/upload/` (6 bundles + `SHA256SUMS.txt`) |
  | Set as a pre-release | **CHECKED** — adviser sign-off pending (v1.0 spec G5), unsigned desktop builds; a published pre-release still fires `published` and deploys |
  | Set as the latest release | n/a — GitHub does not offer it for a pre-release |
  | Create a discussion | unchecked |

- [ ] **Save draft** → attach the assets → re-read the draft →
  **Publish release**.

CLI equivalent (prints the same thing — also printed by
`collect-assets.sh`):

```bash
gh release create v1.0.0 --draft --prerelease \
  --target "$(grep '^HEAD_SHA' target/release/v1.0.0/ci.env | cut -d= -f2)" \
  --title "Epiora v1.0.0 — Calculators and MEANS" \
  --notes-file target/release/v1.0.0/release-notes.md \
  target/release/v1.0.0/upload/*
gh release edit v1.0.0 --draft=false   # publish
```

## Phase 4 — watch the deploy run

Publishing `v1.0.0` fires `app.yml` on `release: [published]`; the `app`
job re-runs the full gate on the tagged tree, rebuilds without
`VITE_TEST_HOOKS`, asserts the canary, then `netlify deploy --prod`.
(The `perf` and desktop-bundle jobs do NOT run on release — desktop
bundles already shipped as the artifacts you attached.)

- [ ] `gh run list --workflow app.yml --event release` →
  `gh run watch <run-id>`
- [ ] If it fails: `gh run rerun <run-id> --failed`. The release stays
  published but **production is not deployed until this run is green**.
  A perf-style flake does not apply here — there is no perf job on
  release; a red gate is real.

## Phase 5 — verify production

- [ ] `python3 scripts/release/verify-deploy.py https://epiora-dev.netlify.app`
  — all PASS (redirect, SPA fallback, every `_headers` rule, all assets
  incl. wasm content types, no `VITE_TEST_HOOKS`, test-hook canary
  present).
- [ ] Short manual pass on `https://epiora-dev.netlify.app`: first load,
  offline reload, import, FREQ + TABLES.

## Phase 6 — record it

- [ ] Tell the conductor: the release URL, the release-event `app.yml`
  run URL, and the Netlify deploy id (Netlify dashboard → site
  `epiora-dev` → Deploys → top deploy; the id is on its detail page).
  The conductor writes the `### v1.0.0 released` entry in
  `docs/decisions.md` via a PR — you don't edit it by hand.

## Rollback

- **Web app:** Netlify dashboard → site `epiora-dev` → Deploys →
  *Publish deploy* on the previous production deploy (v1.0.0's is
  `6b03285b22cce3cc028c34f9`), or fix forward as the next patch release.
- **GitHub release:** edit it back to a draft
  (`gh release edit v1.0.0 --draft=true`) or leave it marked
  pre-release. **Never delete the tag to "retry"** — deletion needs the
  ruleset bypass, and `deployed` history/trust is preserved by keeping
  it.

## Values for v1.0.0

| field | value |
|---|---|
| Version / tag | `1.0.0` / `v1.0.0` (create on publish) |
| Target | `main` at the checkpoint `HEAD_SHA` |
| Title | `Epiora v1.0.0 — Calculators and MEANS` |
| Type | pre-release (`--draft --prerelease`, publish via `--draft=false` or the web form) |
| Latest | n/a (pre-release) |
| Discussion | none |
| Description | `target/release/v1.0.0/release-notes.md` |
| Assets | `target/release/v1.0.0/upload/`: `Epiora_1.0.0_amd64.AppImage`, `Epiora_1.0.0_amd64.deb`, `Epiora-1.0.0-1.x86_64.rpm`, `Epiora_1.0.0_x64-setup.exe`, `Epiora_1.0.0_x64_en-US.msi`, `Epiora_1.0.0_aarch64.dmg`, `SHA256SUMS.txt` |
| Production URL | `https://epiora-dev.netlify.app` |
| Preview of release commit | `https://main--epiora-dev.netlify.app` (dispatch `app.yml` on `main`, `-f runs=1`) |
| Engine version | `1.0.0` — bumped by v1.0 Task 21 (#100); appears in receipts |
