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
| `merge_relate.pgm` | RELATE one-to-many (Nutrition1 → PatientVisits2, 11 visits for patient "1") MATCHING then ALL; MERGE match-or-insert on a scratch copy: duplicate source keys update the destination row, a no-match key inserts. |
| `delete_set.pgm` | DELETE/UNDELETE (RUNSILENT — both prompt otherwise) in the working view, SET PROCESS=BOTH/UNDELETED, SET MISSING=ON/OFF, SET DELETED. |
| `functions_date.pgm` | YEAR/MONTH/DAY/HOUR over real Access datetimes with missing cells, MINUTES/DAYS/YEARS/MONTHS two-argument differences, TXTTODATE, NUMTODATE(y,m,d), NUMTOTIME(h,m,s). |
| `functions_text_math.pgm` | UPPERCASE, SUBSTRING, STRLEN, TXTTONUM, FINDTEXT, FORMAT; ROUND, TRUNC, ABS, EXP, LN, SQRT; ROUND propagates missing (AgeMonths has one missing cell). |

## Runtime findings (7.2.7.0, measured)

- `SET <clause> END-SET` — the `END-SET` terminator is required.
- `DEFINE x = <expr>` accepts column expressions (`AGE + AGE`) but a
  function call is resolved as a column name ("Column 'X' does not belong
  to table"); use a typed `DEFINE` plus `ASSIGN` for function results.
- `MERGE`'s `APPEND`/`UPDATE`/`RELATE` option word crashes inside
  `Rule_Merge.SetMergeOpt` (IndexOutOfRangeException — the option token's
  child list is read past its end). Only the bare `MERGE file:table key`
  form (default match-or-insert) runs headless; that is what the corpus
  pins.
- `DELETE <expr>` and `UNDELETE *` show a "Delete Records?" modal without
  `RUNSILENT` — fatal headless.
- `ABS`, `TRUNC` (and presumably the same family of numeric functions)
  throw `NullReferenceException` on a missing input cell instead of
  propagating missing; `ROUND`, `YEAR`/`MONTH`/`DAY`/`HOUR`, `MINUTES`,
  `DAYS`, `YEARS`, `MONTHS`, `UPPERCASE`, `SUBSTRING`, `STRLEN`,
  `TXTTONUM`, `FORMAT`, `FINDTEXT` propagate missing cleanly. The corpus
  therefore exercises the crash-prone functions only on the fully
  populated `HeightInches` column.
- `MINUTES(a, b)` is a two-argument difference function, not a part
  extractor; `NUMTODATE`/`NUMTOTIME` take three integer arguments.
- `LOG` is reserved but rejected by the expression parser ("Unexpected
  Token: LOG"); `LN` is the usable natural log.
- FREQ excludes missing values from both the categories and the Total
  (`SELECT MASHEDPOTA = (.)` selects the one missing row; FREQ then
  reports Total 0).
- Epi Info writes the ROUTEOUT target with the path's own casing —
  `define_assign_if.html` is produced as `DEFINE_ASSIGN_IF.HTML`; the
  committed corpus is normalised to lowercase names (bytes unchanged).

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
