# Epi Info 7 v1.1 reference corpus

Epiora v1.1 Task 3 (spec §8.1). Seven reference programs, run by
`.github/workflows/ei7.yml` through the official Epi Info 7.2.7.0
`Analysis.exe` on `windows-latest`. Each `<topic>.pgm` is rendered into a
`.pgm7` by `fixtures/ei7/run.ps1` with two substitutions:

- `@WORK_DIR@` — `ei7-out/work/v11-data`, holding **scratch copies** of the
  committed sample `.mdb` fixtures. MERGE writes into the destination
  table's file, so the committed fixtures are never touched.
- `@OUTPUT_HTML@` — `ei7-out/v11/<topic>.html`.

The generated `<topic>.html` is committed verbatim and parsed by
`fixtures/ei7/parse.mjs --steps` into `<topic>.json` — the steps corpus
Task 11 replays. Never edit either output by hand; regenerate via
`workflow_dispatch` of `ei7.yml`.

## Programs

| Program | Covers (spec §15.2) |
| --- | --- |
| `select_sort.pgm` | SELECT missing-value comparisons, repeated SELECT AND-combining, CANCEL SELECT; SORT asc/desc/multi-key, sort order of missing (TIMESUPPER), CANCEL SORT. |
| `define_assign_if.pgm` | DEFINE typed and define-expression forms, ASSIGN, IF/THEN/ELSE/END, UNDEFINE; missing propagation in arithmetic (BIGAGE). |
| `recode.pgm` | RECODE LOVALUE/HIVALUE and inner boundaries (AGE rows at 17/64/65), ELSE catching a missing source (MASHEDPOTA), numeric-to-text recode. |
| `merge_relate.pgm` | RELATE one-to-many (Nutrition1 → PatientVisits2, 11 visits for patient "1") MATCHING then ALL; MERGE UPDATE and APPEND on a scratch copy, duplicate source keys. |
| `delete_set.pgm` | DELETE/UNDELETE in the working view, SET PROCESS=DELETED/BOTH/UNDELETED, SET MISSING=ON/OFF, SET DELETED. |
| `functions_date.pgm` | YEAR/MONTH/DAY/HOUR/MINUTES over real Access datetimes with missing cells, DAYS/YEARS/MONTHS, TXTTODATE, NUMTODATE, NUMTOTIME. |
| `functions_text_math.pgm` | UPPERCASE, SUBSTRING, STRLEN, TXTTONUM, FINDTEXT, FORMAT; ROUND, TRUNC, ABS, EXP, LN, LOG, SQRT; missing propagation. |

## Runs

_Generating runs and measurements are recorded here and in
`docs/decisions.md` (`### v1.1 Task 3`) once the workflow_dispatch runs
complete._

## EI3 upgrade probe

`ei7.yml`'s `ei3-upgrade-probe` job (dispatch input `ei3_probe`) drives
`Epi.ImportExport.ProjectUpgradeManager` — the class behind MakeView's
interactive Epi Info 3.x upgrade path — on the source-built route against
`fixtures/legacy/samples/ei3/Asthma Survey 2005 Sample.mdb`. Result and
run link are recorded in `docs/decisions.md`.
