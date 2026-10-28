# v0.1 Task 4 — per-task notes

- Owner decision D2 (PR #2, Option 1, 2026-10-28): Epi Info 7 is Apache-2.0; reuse is permitted with notice.
- If you copy `Sample.mdb` (or any other Epi Info file) into `spikes/mdb-wasm/`, put a `NOTICE` file next to
  it. The notice names the origin (`github.com/Epi-Info/Epi-Info-Community-Edition` @
  `4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6`, path `Epi.Core/Projects/Sample/Sample.mdb`) and its licence
  (Apache-2.0), and includes or links a copy of the Apache-2.0 text. Record it in the report's Licences section.
- Do not modify the copied `.mdb`. If a modified or truncated copy is needed (Review Focus 1), give it a new name
  and state in the NOTICE that it was changed (Apache §4.2).
- Do not reuse the CC BY-ND icons in `EpiDashboard/`.

## Learned in Task 3 (PR #4, docs/spikes/ei7-windows.md)

- The Oswego data in `Sample.mdb` is the table `Oswego12` (22 fields, 75 rows). The table `Oswego` holds only 4
  system columns (`RecStatus`, `UniqueKey`, `GlobalRecordId`, `FKEY`). Use `Oswego12` as the reader's reference
  table.
- Reference values for the reader test: `Oswego12` has 75 rows; the VANILLA × ILL 2×2 is 43/11/3/18 (Epi Info 7,
  run https://github.com/dorrabbkhan/Epiora/actions/runs/40754370590). Your measurements must come from your own
  runs; cite these only as the cross-check.
- Connection strings in `Sample.prj` are encrypted and did not decrypt with either the source-built or the
  official 7.2.7.0 Epi Info binary. Read `Sample.mdb` directly; do not depend on `.prj` files.
