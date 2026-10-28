# Epi Info 7 on Windows CI — can it run a .pgm headless as a stats reference?

**Status:** go-with-changes

## Question

Spec §12.2 wants `.pgm` scripts run through Epi Info 7 on a Windows CI runner as
a reference oracle for the compatibility interpreter — "building or obtaining
Epi Info 7 now that CDC removed installers" is the risky bet (plan Task 3).
Can `Epi Info 7.sln` be built with MSBuild on `windows-latest`, and can
`Analysis.exe` execute a `.pgm` (`READ` the Oswego sample, `TABLES VANILLA
ILL`, `ROUTEOUT` to HTML) unattended, ending in a CI artifact?

## Method

Workflow `.github/workflows/spike-ei7.yml` (windows-latest): checks out
`Epi-Info/Epi-Info-Community-Edition` at the D2 commit, `nuget restore`,
`msbuild "Epi Info 7.sln" /p:Configuration=Release`, downloads and extracts
the official signed release ZIP `7.2.7.0` (the "archived installer" — CDC no
longer hosts downloads, but the GitHub release of 2025-03-09 remains), then
runs `spikes/ei7-windows/run-pgm.ps1`.

`run-pgm.ps1` renders `oswego-tables.pgm` with absolute paths into a `*.pgm7`
file (Analysis.exe's command-line scan only accepts arguments containing
`.pgm7`), launches Analysis.exe, and waits up to 300 s; the program's `QUIT`
command closes the app (the only exit route — `Application.Run` otherwise
idles forever). On timeout it dumps the window's UI-Automation tree plus
`Configuration\`, `Output\` and `Logs\` contents before killing the process.
The check asserts `ei7-output/*.html` contains `VANILLA`; the folder is
uploaded as a workflow artifact.

## Environment

- Runner: `windows-latest` (Windows Server with Visual Studio; MSBuild,
  NuGet, .NET Framework 4.8 targeting pack).
- Epi Info source: `Epi-Info/Epi-Info-Community-Edition` @
  `4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6` (D2 commit).
- Official binary: GitHub release `7.2.7.0`,
  `2025.3.9.Epi.Info.7.2.7.0.signed.dual.keys.en-US.es-ES.fr-FR.pt-BR.zip`
  (85 MB, extracted and run in place — no installation or elevation).
- No RNG used.

## Measurements

### Iteration 1 — run [40722364407](https://github.com/dorrabbkhan/Epiora/actions/runs/40722364407)

- `nuget restore` + `msbuild "Epi Info 7.sln" /p:Configuration=Release
  /p:Platform="Mixed Platforms"`: **SUCCESS** (warnings only; `Setup.vdproj`
  and `SilverlightControls.Web` have no `Build.0` in Release so are skipped).
- `Analysis.exe oswego-tables.pgm7` (`READ {Sample.prj}:Oswego`): process
  stayed alive > 300 s → killed, no html (expected Step-2 failure).

### Iteration 2 — run [40724089856](https://github.com/dorrabbkhan/Epiora/actions/runs/40724089856)

- Timeout again. UIA dump: one window "Epi Info 7 - Analysis",
  `Responding=True`, status bar "Ready" — **no modal dialog**; headless GUI
  works. `Configuration\EpiInfo.Config.xml` (10,698 B) and
  `Output\output1.html` (2,990 B, template only) created → the program never
  executed past the first command.

### Iteration 3 — run [40740143056](https://github.com/dorrabbkhan/Epiora/actions/runs/40740143056)

- Program-editor text captured: `CryptographicException: Padding is invalid
  and cannot be removed` in `Epi.Configuration.Decrypt` ←
  `Project.get_CollectedDataConnectionString` ← `Rule_Read.Execute`. A `.prj`
  read fails because the shipped `.prj` connection strings cannot be
  decrypted by the compiled-in key (verified locally: AES-CBC with the
  `passPhrase`/`saltValue`/`initVector` constants in `Configuration_Static.cs`
  yields garbage on `Sample.prj`).

### Iteration 4 — run [40742051799](https://github.com/dorrabbkhan/Epiora/actions/runs/40742051799)

- `READ 'Sample.mdb':Oswego` **worked** — "Current Data Source … Oswego
  Record Count: 75". `TABLES VANILLA ILL` then threw
  `NullReferenceException` in `cDataSetHelper.SelectDistinct` (column not in
  the loaded table).

### Iteration 5 — run [40743601771](https://github.com/dorrabbkhan/Epiora/actions/runs/40743601771)

- Config patch applied (`TextEncryptionModule` row created, `FileName` →
  `FipsCrypto.dll`); `.prj` read still fails — `FipsCrypto.FipsCrypto.Decrypt`
  throws the **same** "Padding is invalid" CryptographicException. The
  shipped `.prj` files are not decryptable by either key in this repo.
- Fallback `READ 'Sample.mdb':Oswego` + `LIST *` + `QUIT`: **Analysis.exe
  exited code 0 in ~3 s** — the first clean headless run — but listed only
  `RecStatus UniqueKey GlobalRecordId FKEY` (Oswego is a 4-column parent
  table, not the data).

### Iteration 6 — run [40745103316](https://github.com/dorrabbkhan/Epiora/actions/runs/40745103316)

- Fallback `DISPLAY TABLES` on `Sample.mdb` (exit 0, ~20 s) enumerated the
  database: `Oswego` (4 fields) plus **`Oswego12` (22 fields)** — the real
  Oswego data table. Verified locally against the sibling `SampleDB.db`
  (SQLite mirror): `Oswego12` has `VANILLA`/`ILL` INTEGER columns, 75 rows,
  2×2 counts 43/11/3/18.

### Iteration 7 — run [40746740835](https://github.com/dorrabbkhan/Epiora/actions/runs/40746740835) — PASS

- Official 7.2.7.0 ZIP extracted; **it ships no `EpiInfo.Config.xml`** — one
  is generated at first launch.
- `Analysis.exe oswego-tables.pgm7` (`READ 'Sample.mdb':Oswego12` +
  `ROUTEOUT` + `TABLES VANILLA ILL` + `QUIT`): exit code 0 after ~5 min;
  `ei7-output/oswego-tables.html` contains the canonical table — **43/11/3/18,
  RR = 5.5741 (1.9383–16.0296), OR = 23.4545, χ² = 27.2225** (artifact
  `ei7-output` of the run).

### Iteration 8 — run [40749181932](https://github.com/dorrabbkhan/Epiora/actions/runs/40749181932) — PASS

Full route matrix, one run (artifact `ei7-output`):

| Route | Result |
|---|---|
| official 7.2.7.0 exe + `Sample.mdb:Oswego12` + TABLES | exit 0 in ~4 s → `oswego-tables.html` (canonical 2×2) |
| **source-built** exe + `Sample.mdb:Oswego12` + TABLES | exit 0 in ~2 s → `oswego-tables-built.html` (canonical 2×2) |
| official exe + `{Sample.prj}:Oswego` + TABLES | **fails**: `CryptographicException: Padding is invalid` inside `FipsCrypto.FipsCrypto.Decrypt` → app idles → killed at 300 s |

So even the **official installed binary cannot read the shipped `.prj`
files** — their connection strings are encrypted with a key that exists in
neither the repo nor the release ZIP (likely bound to CDC's install-time
config). `.prj` view READs are therefore off the table for CI reference use;
direct `.mdb`/`.db` table READs work identically on both binaries.

## Decision

**go-with-changes: Epi Info 7 is usable as a CI reference on `windows-latest`
— both from a Release build of `Epi Info 7.sln` and from the official
7.2.7.0 release ZIP.** `Analysis.exe <file>.pgm7` runs Classic Analysis
programs headless and exits cleanly on `QUIT`; `ROUTEOUT … REPLACE` yields
HTML artifacts. **Change required:** reference `.pgm`s must `READ` data
tables directly (`'file.mdb':table`) — no binary can decrypt the shipped
`.prj` connection strings, so `.prj` view READs are unavailable.

## Consequences

- For the §12.2 reference corpus: author `.pgm`s that `READ` `.mdb` data
  tables (table names via `DISPLAY TABLES`; e.g. the Oswego data is table
  `Oswego12`, not `Oswego`). `.prj` view READs are unusable from CI binaries
  — flagging as a limitation for future tasks (corpus coverage of
  view-level metadata).
- `Analysis.exe` needs a window station: it works on GitHub-hosted
  `windows-latest` runners but may fail on self-hosted agents in session 0
  variants — keep `run-pgm.ps1`'s UIA dump for diagnosis.
- Any failing command silently aborts the rest of the program (errors go to
  the editor text, not the log or exit code) — a `QUIT` at the end is also
  the only thing that exits the process, so a failure means "hangs until
  timeout". Harnesses must treat "no output file" as the failure signal.
- `.pgm` string literals treat `\` as an escape — substitute paths with
  forward slashes.
- No Epi Info binaries or outputs are committed; the release ZIP is
  downloaded at workflow time and outputs live only as artifacts (D2 rule).

## Licences

- `Epi-Info/Epi-Info-Community-Edition` @ `4cd353c` (source + bundled
  third-party DLLs + Sample data + official 7.2.7.0 release ZIP): Apache-2.0
  per D2; "Epi Info" trademark remains CDC's.
- `com.calitha.goldparser` (linked inside the interpreter): licence still
  undetermined (D2 open item — matters only if its code is reused).
- GitHub Actions used: `actions/checkout@v4`, `microsoft/setup-msbuild@v2`,
  `nuget/setup-nuget@v2`, `actions/upload-artifact@v4` — MIT.
- NuGet packages restored for the source build (not vendored by Epiora):
  Newtonsoft.Json (MIT), MongoDB.Driver & deps (Apache-2.0),
  System.Data.SQLite (public domain), EntityFramework (Apache-2.0),
  RedcapAPI (MIT), Serilog (Apache-2.0), MSTest (MIT).
- Spike code: no new dependencies beyond the plan's tech stack.
