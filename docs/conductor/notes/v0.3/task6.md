# v0.3 Task 6 — conductor notes

## Add: replay test for `epiora-import` fuzz regressions (from Task 5 follow-up, PR #40)

`.github/workflows/fuzz-nightly.yml` says a nightly crash input is committed under
`fuzz/regressions/<target>/` so that `tests/fuzz_regressions.rs` replays it forever. That is true for
`epiora-core` (`crates/epiora-core/tests/fuzz_regressions.rs`) but `epiora-import` has no replay test:
`crates/epiora-import/fuzz/regressions/` holds only `text_import/.gitkeep`, so a committed import crash input
would never be re-run. Task 6 touches the import fuzz wiring (`import.yml`, `fuzz-nightly.yml` import
targets), so it closes the gap:

- Create `crates/epiora-import/tests/fuzz_regressions.rs`, mirroring the core test: for each target in
  `text_import`, `sheet_import`, `diff_any`, replay every file under
  `crates/epiora-import/fuzz/regressions/<target>/` (ignore `.gitkeep`; a missing or empty folder is a pass)
  through the same library function the cargo-fuzz target calls (`epiora_import::text_import`,
  `epiora_import::sheet_import`, `epiora_import::diff_any`), behind the same feature gate the fuzz harness
  functions use.
- Add a smoke test that runs all three harnesses on empty input and 64 bytes of `0xff`.
- Add `.gitkeep` files for `regressions/sheet_import/` and `regressions/diff_any/`.
- Make sure the test runs in CI (the `cargo test -p epiora-import` step in `import.yml` with the feature
  enabled, or `--all-features`); paste the CI line showing it ran.
- Record these files under "Files outside the Files block" in the Task 6 decisions entry, citing this note.

Do not commit any crash input that is not a real crash; do not change any existing fuzz target.

## Task 5 interpretations (accepted, PR #40)

Task 5's decisions entry fixes these behaviours; Task 6's bindings must keep them, not reinterpret them:
absent field in records JSON → missing; malformed record → `invalid_settings`; `record_json` past the end →
`{}`; added columns emit `FieldChange`s with `old: None`, removed columns only in `schema.removed_columns`;
`unchanged` ignores row reordering under a key column; `FieldChange.row` is the row index in `next`.

## Environment

Locally `cargo deny` is installed under `target/bin`, not as a cargo subcommand: run the full gate as
`PATH="$PWD/target/bin:$PATH" ./scripts/check.sh` and say so in the PR.
