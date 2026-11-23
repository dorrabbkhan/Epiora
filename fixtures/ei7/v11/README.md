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
- `UNDELETE` is a no-op on a raw `.mdb` READ: Epi Info prints "The current
  Read must be an Epi7 project. 0 items marked as deleted." and deleted
  records stay deleted (pinned: `undelete.records_restored: 0`, FREQ
  totals unchanged).
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
- Programs that print a data-source banner stamp a wall-clock
  `<i>Date:</i>&nbsp;&nbsp;<b>M/d/yyyy h:mm:ss tt</b>` into the html
  (merge_relate, select_sort). It is the only per-run byte difference,
  so `ei7.yml` normalises it to the literal token `[run timestamp]`
  before parsing and before the drift check — proven by runs
  [`45182109352`](https://github.com/dorrabbkhan/Epiora/actions/runs/45182109352)
  (drift step showed only the stamp) and
  [`45182452028`](https://github.com/dorrabbkhan/Epiora/actions/runs/45182452028)
  (ei7 job green, "No drift in fixtures/ei7" clean).

## Runs

Generating run:
[`45161983082`](https://github.com/dorrabbkhan/Epiora/actions/runs/45161983082)
(ei7.yml `workflow_dispatch`, all seven programs `run: OK`, every json
schema-valid, `git diff --exit-code fixtures/ei7` clean). Development
runs that produced the pinned runtime findings above: `45151708069`,
`45156679205`, `45158293708`, `45159694424`, `45160531102` — a failed or
partial run is a measurement; the program fixes each one motivated are
in `docs/decisions.md` (`### v1.1 Task 3`).

| Program | Steps | Output-producing steps |
| --- | --- | --- |
| `select_sort` | 29 | 5 FREQ, 6 LIST, 9 SELECT/CANCEL SELECT, 4 SORT, CANCEL SORT |
| `define_assign_if` | 40 | 3 MEANS, 4 FREQ, DISPLAY DBVARIABLES |
| `recode` | 35 | 5 FREQ |
| `merge_relate` | 20 | DISPLAY TABLES, 4 LIST, 3 FREQ, 2 RELATE, 4 READ, 2 MERGE |
| `delete_set` | 21 | 7 FREQ, 1 MEANS, 2 LIST, DELETE, UNDELETE |
| `functions_date` | 34 | 7 FREQ, 1 MEANS, 2 LIST |
| `functions_text_math` | 27 | 1 LIST |

## EI3 upgrade probe

`ei7.yml`'s `ei3-upgrade-probe` job (dispatch input `ei3_probe`) drives
`Epi.ImportExport.ProjectUpgradeManager` — the class behind MakeView's
interactive Epi Info 3.x upgrade path — on the source-built route against
`fixtures/legacy/samples/ei3/Asthma Survey 2005 Sample.mdb`. **Result:
completes unattended once the probe exits on `ImportEnded`.** Run
[`45177072437`](https://github.com/dorrabbkhan/Epiora/actions/runs/45177072437)
upgraded all 800 source rows into four page tables — probe.log verbatim
head and tail (the elided middle is per-field `STATUS` lines plus
`Processed 1..800 of 800 unique record identifiers in
PreInterventionSurvey`):

    STAGE1 ei3 project loaded: views=1
      view viewPreInterventionSurvey table=PreInterventionSurvey
    STAGE2 configuration loaded: D:\a\Epiora\Epiora\ei7\build\Release\Configuration\EpiInfo.Config.xml
    STAGE2 target project created: D:\a\Epiora\Epiora\ei3-probe\UpgradedProbe\UpgradedProbe.prj
    STATUS viewPreInterventionSurvey...
    ...
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

The run-`45159694424` hang was not the upgrade: `Import()` raises
`ImportEnded` and *then* shows the modal `ImportMessagesDialog`
(`Epi.ImportExport/ProjectUpgradeManager.cs` ~lines 346-352 @ `4cd353c`),
which blocks forever on a headless desktop — that run's artifact already
held the fully upgraded `UpgradedProbe\UpgradedProbe.mdb`. The probe now
subscribes `ImportStatus` (logged as `STATUS`) and `ImportEnded`, prints
the row counts inside the handler and calls `Environment.Exit(0)` before
the dialog opens; the step is capped at `timeout-minutes: 20` so a hang
can never again use the whole 90-minute job. Kept opt-in and
nonblocking; full history in `docs/decisions.md` (`### v1.1 Task 3`).
