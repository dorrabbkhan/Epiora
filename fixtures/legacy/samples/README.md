# Legacy sample projects (v1.1)

Sample Epi Info projects used by the v1.1 legacy-import tasks (golden imports
in `fixtures/legacy/golden/`, the `epiora-legacy` mappers, the re-import diff
and the migration report). Nothing here is generated — every file is a
verbatim, unmodified copy; SHA-256 of each file is in `SHA256SUMS` and the
licence/provenance for each source is in the `NOTICE` next to it.

## Sources

- `ei7/` — every `.prj`, `.mdb` and `.db` file under `Epi.Core/Projects/` in
  `github.com/Epi-Info/Epi-Info-Community-Edition` @
  `4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6` (Apache-2.0, decision D2;
  see `ei7/NOTICE`). 46 files in 20 folders.
- `ei3/` — the Epi Info 3.5.4 Community Health Assessment Tutorial project
  files (`EIHATProjectFiles.zip` from
  `https://www.cdc.gov/epiinfo/zipfiles/eihat/EIHATProjectFiles.zip`, SHA-256
  `9954794cc6d721c6c50030f284e7f3d0a4f53cbacee8cff0667a039b37a17dee`;
  decision D12 — public domain per the tutorial's copyright page; see
  `ei3/NOTICE` and `docs/decisions.md`). 24 files; `censusage_files/` (web-page
  chrome) omitted.

## Inventory — Epi Info 7 (`ei7/`)

Views are rows of each project database's `metaViews` table; records are the
row count of the view's principal data table(s) as readable by mdbtools 1.0.1
(native build, mdb-tables/mdb-export/mdb-count). `.prj` files are XML project
descriptors; their encrypted connection strings do not decrypt outside
Epi Info 7 (spike `docs/spikes/ei7-windows.md`), so the data file next to
each `.prj` is the project database.

| Project | Source path (`Epi.Core/Projects/…`) | Format | JET | Views | Records |
|---|---|---|---|---|---|
| BeefPrevention | `BeefPrevention/` | `.prj` + SQLite `.db` | — | 1 (Investigation) | 0 |
| Cholera | `Cholera/Cholera_in_Rwenshama_2` | `.prj` + Access `.mdb` | JET4 | 3 | Cholera 66; CholeraCases 33; Questionnaire_DataEntry 0 |
| Cholera (extra DBs) | `Cholera/` | Access `.mdb` ×2, no `.prj` | JET4 | — | EpiChol.mdb SURVEY 66; Advanced Analysis.mdb Mortality 489 |
| CommunityHealthAssessment | `CommunityHealthAssessment/` | `.prj` + Access `.mdb` | JET4 | 3 | PreInterventionSurvey 800 |
| CHA extra DB | `CommunityHealthAssessment/albany_demogz.mdb` | Access `.mdb` | JET4 | — | INCOME 38; RACEETHNICITY 38 |
| EColi | `EColi/` | `.prj` + Access `.mdb` | JET4 | 2 | FoodHistory 359 |
| H1N1ClinicalLongForm | `H1N1ClinicalLongForm/` | `.prj` + Access `.mdb` | JET4 | 8 | ReportCaseID 0 |
| H1N1ClinicalShortForm | `H1N1ClinicalShortForm/` | `.prj` + Access `.mdb` (`H1N1ShortForm.mdb`) | JET4 | 3 | CaseReport 1; Household 1; Laboratory 0 |
| H1N1ShortForm | `H1N1ShortForm/` | `.prj` + Access `.mdb` | JET4 | 3 | same as H1N1ClinicalShortForm (duplicate copy) |
| HIV | `HIV/` | `.prj` + Access `.mdb` | JET4 | 1 (Case) | HIV 359 |
| HansensDisease | `HansensDisease/` | `.prj` + Access `.mdb` | JET4 | 4 | HDCaseForm 1 |
| HuttYouthSurvey | `HuttYouthSurvey/` | `.prj` + Access `.mdb` | JET4 | 1 | Questionnaire 0 |
| JohnSnow | `JohnSnow/` | `.prj` + Access `.mdb` | JET4 | 1 | Survey 452 |
| LandmineStudy | `LandmineStudy/` | `.prj` + Access `.mdb` | JET4 | 1 | COMPLETECASUALTYFORM 0 |
| Legionellosis | `Legionellosis/` | `.prj` + Access `.mdb` | JET4 | 1 | CaseReport 40 |
| Lyme | `Lyme/` | `.prj` + Access `.mdb` | JET4 | 1 | CaseReport 1159 |
| Mumps | `Mumps/` | `.prj` + Access `.mdb` | JET4 | 1 | Survey 45 |
| Nutrition | `Nutrition/` | `.prj` + Access `.mdb` | JET4 | 2 | Nutrition 3; PatientVisits 11 |
| PHIN | `PHIN/PHINVS.mdb` | Access `.mdb`, no `.prj` | JET4 | — | PHVS vocabulary store, 425 tables |
| STEPS | `STEPS/STEPS_EI7` | `.prj` + Access `.mdb` | JET4 | 1 | SurveyInformation 0 |
| STEPS extra DB | `STEPS/Steps_EpiInfo.mdb` | Access `.mdb` | JET4 | — | JOSE/temp/MasterDataSet 3516 |
| Salmonellosis | `Salmonellosis/` | `.prj` + Access `.mdb` | JET4 | 2 | Survey_Final 309 |
| Sample | `Sample/` | `.prj` + Access `.mdb` + SQLite `.db` | JET4 | 18 (mdb) / 19 (db) | Oswego 75; Epi10 2152; EvansCounty 609; SohoCholera 497 (db only); others in metaViews |

Not copied — project folders with no `.prj`/`.mdb`/`.db` at the D2 commit
(listed here so the 22-folder count is complete):

| Folder | Contents | Reason |
|---|---|---|
| KML_Example | `Maryland_Counties.kml`, `ACS_11_1YR_HealthInsurance.xls` | no project/data file |
| Rates | `.cvs7`/`.xlsx`/`.docx` reports | no project/data file |

Counts reconcile with D2 (`docs/spikes/d2-epi-info-licence.md`): 22 folders,
21 `.prj` files total — 19 project `.prj` plus the two ESRI shapefile
projection files in `CommunityHealthAssessment/` (`ALBZCTA_region.prj`,
`Albany County Boundary_region.prj`), which are copied here verbatim and are
shapefile metadata, not Epi Info projects.

## Inventory — Epi Info 3.5.4 (`ei3/`)

Epi Info 3.5.x projects are single `.mdb` files (JET3/Access 97 era) that
hold both data tables and `view<Name>` view-definition tables — no `.prj`.

| File | Format | JET | Views | Records (principal tables) |
|---|---|---|---|---|
| `Asthma Final.mdb` | Access `.mdb` | JET3 | 1 (PreInterventionSurvey) | PreInterventionSurvey 800; BRFSS2003 5544; 25 more data tables |
| `Asthma Survey 2005.mdb` | Access `.mdb` | JET3 | 1 (PreInterventionSurvey) | PreInterventionSurvey 10 |
| `Asthma Survey 2005 Sample.mdb` | Access `.mdb` | JET3 | 1 (PreInterventionSurvey) | PreInterventionSurvey 800; ageandsymptom 369 |
| `albany_demogz.mdb` | Access `.mdb` | JET4 | — | INCOME 38; RACEETHNICITY 38 |
| `HospitalPoints.mdb` | Access `.mdb` | JET3 | — | hosppts 12 |
| `SchoolPoints.mdb` | Access `.mdb` | JET3 | — | pttable 128 |
| `SymptomsbyAgeSample.mdb` | Access `.mdb` | JET3 | — | AllSymptoms 800; ageandsymptom 369 |

Non-database tutorial files kept with the samples: `AsthmaTable.xls`,
`censusage.htm`, `Frequencies.htm`, `Frequencies.xml`, `My Epi Report2.EPT`
/`.epx`, `WMonth Report2.EPT`/`.epx`, `WMonthPieGraph2.bmp`, and the two
Albany shapefile sets (`.dbf`/`.prj`/`.shp`/`.shx`).

## Notes for later tasks

- All EI7 project databases are JET4; the EI3 tutorial `.mdb`s are JET3
  (except `albany_demogz.mdb`, JET4) — JET3 Windows-code-page text is covered
  by `Asthma Final.mdb` (the review-focus file for `jet3_cp1252_text_decodes`).
- `H1N1ClinicalShortForm/` and `H1N1ShortForm/` hold identical
  `H1N1ShortForm.mdb` copies (same SHA-256, see `SHA256SUMS`); their `.prj`
  files differ. The duplicate is upstream, kept verbatim.
- `Sample/SampleDB.db` is the SQLite twin of `Sample.mdb` (19 views incl.
  `SohoCholera`, which the `.mdb` lacks) — the pair exercises the
  cross-back-end equality criterion (spec §8.2).
- `PHIN/PHINVS.mdb` is a vocabulary store with no `.prj` — the
  `plain_access_db_is_layout_unknown_with_tables` review-focus case.
