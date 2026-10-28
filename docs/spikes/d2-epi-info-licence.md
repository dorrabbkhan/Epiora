# D2 — Epi Info 7 source licence

**Status:** go

## Question

Spec §17 open decision D2, exercised by plan Task 2: what is the exact licence of the
Epi Info 7 source (`github.com/Epi-Info/Epi-Info-Community-Edition`), and may Epiora
reuse its code, its GOLD grammar files (Classic Analysis `.pgm` and Enter check
code) and its sample data — and under what notice?

## Method

Repository inspection only; nothing was built or executed. At the current `master`
commit, `license.md` was fetched and read in full; the recursive git tree was
searched for other `LICENSE*`/`NOTICE*`/`COPYING*` files; representative sources in
`Epi.Core.Interpreter` were checked for per-file licence headers; and the tree was
searched for sample projects (`.prj`/`.mdb`), grammar files (`.grm`/`.cgt`) and
`.pgm` scripts. Commands used (all read-only GitHub API calls):

    gh api repos/Epi-Info/Epi-Info-Community-Edition/commits/master
    gh api "repos/Epi-Info/Epi-Info-Community-Edition/git/trees/<sha>?recursive=1"
    gh api "repos/Epi-Info/Epi-Info-Community-Edition/contents/<path>?ref=<sha>"
    gh api repos/Epi-Info/Epi-Info-Community-Edition

## Environment

- Repo: `Epi-Info/Epi-Info-Community-Edition`, default branch `master`.
- Commit inspected: `4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6`
  ("Delete .github/workflows/simple-desktop.yml", committed 2025-04-09) — the
  `master` HEAD at the time of this spike.
- Local environment: Linux; no toolchain exercised. No RNG used.

## Measurements

### Licence files found

- `license.md` (repo root, 9,206 bytes) — quoted verbatim below. It is the
  complete Apache License, Version 2.0, January 2004, sections 1–9 (no appendix).
- `EpiDashboard/license.txt` (1,944 bytes) — a *third-party* licence for bundled
  Modern UI icons: Creative Commons BY-ND 3.0, creator Austin Andrews
  (@templarian). It covers icons only, not the project source.
- No `NOTICE*` or `COPYING*` files anywhere in the tree.
- No per-file copyright/licence headers in the checked `Epi.Core.Interpreter`
  sources (`EpiInfo7EventGrammar.cs`, `Rule_Context.cs`,
  `Properties/AssemblyInfo.cs` begin directly with `using` statements).
- `SolutionInfo.cs` carries assembly attributes:
  `AssemblyCompany("Centers for Disease Control and Prevention")`,
  `AssemblyCopyright("© Centers for Disease Control and Prevention")`,
  `AssemblyTrademark("Epi Info is a trademark of CDC")`,
  `AssemblyVersion("7.2.*")`, `AssemblyFileVersion("7.2.7.0")`.
- `README.md` describes Epi Info as "a public domain suite of interoperable
  software tools" — descriptive text; the repository's operative licence grant is
  `license.md`.
- GitHub licence detection reports `license.spdx_id: NOASSERTION` for the repo;
  the file content itself is verbatim Apache-2.0 as shown below.

### `license.md` verbatim (commit 4cd353c)

```text
Apache
Apache License
Version 2.0, January 2004
http://www.apache.org/licenses/

TERMS AND CONDITIONS FOR USE, REPRODUCTION, AND DISTRIBUTION

1. Definitions.

"License" shall mean the terms and conditions for use, reproduction, and distribution as defined by Sections 1 through 9 of this document.

"Licensor" shall mean the copyright owner or entity authorized by the copyright owner that is granting the License.

"Legal Entity" shall mean the union of the acting entity and all other entities that control, are controlled by, or are under common control with that entity. For the purposes of this definition, "control" means (i) the power, direct or indirect, to cause the direction or management of such entity, whether by contract or otherwise, or (ii) ownership of fifty percent (50%) or more of the outstanding shares, or (iii) beneficial ownership of such entity.

"You" (or "Your") shall mean an individual or Legal Entity exercising permissions granted by this License.

"Source" form shall mean the preferred form for making modifications, including but not limited to software source code, documentation source, and configuration files.

"Object" form shall mean any form resulting from mechanical transformation or translation of a Source form, including but not limited to compiled object code, generated documentation, and conversions to other media types.

"Work" shall mean the work of authorship, whether in Source or Object form, made available under the License, as indicated by a copyright notice that is included in or attached to the work (an example is provided in the Appendix below).

"Derivative Works" shall mean any work, whether in Source or Object form, that is based on (or derived from) the Work and for which the editorial revisions, annotations, elaborations, or other modifications represent, as a whole, an original work of authorship. For the purposes of this License, Derivative Works shall not include works that remain separable from, or merely link (or bind by name) to the interfaces of, the Work and Derivative Works thereof.

"Contribution" shall mean any work of authorship, including the original version of the Work and any modifications or additions to that Work or Derivative Works thereof, that is intentionally submitted to Licensor for inclusion in the Work by the copyright owner or by an individual or Legal Entity authorized to submit on behalf of the copyright owner. For the purposes of this definition, "submitted" means any form of electronic, verbal, or written communication sent to the Licensor or its representatives, including but not limited to communication on electronic mailing lists, source code control systems, and issue tracking systems that are managed by, or on behalf of, the Licensor for the purpose of discussing and improving the Work, but excluding communication that is conspicuously marked or otherwise designated in writing by the copyright owner as "Not a Contribution."

"Contributor" shall mean Licensor and any individual or Legal Entity on behalf of whom a Contribution has been received by Licensor and subsequently incorporated within the Work.

2. Grant of Copyright License.

Subject to the terms and conditions of this License, each Contributor hereby grants to You a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable copyright license to reproduce, prepare Derivative Works of, publicly display, publicly perform, sublicense, and distribute the Work and such Derivative Works in Source or Object form.

3. Grant of Patent License.

Subject to the terms and conditions of this License, each Contributor hereby grants to You a perpetual, worldwide, non-exclusive, no-charge, royalty-free, irrevocable (except as stated in this section) patent license to make, have made, use, offer to sell, sell, import, and otherwise transfer the Work, where such license applies only to those patent claims licensable by such Contributor that are necessarily infringed by their Contribution(s) alone or by combination of their Contribution(s) with the Work to which such Contribution(s) was submitted. If You institute patent litigation against any entity (including a cross-claim or counterclaim in a lawsuit) alleging that the Work or a Contribution incorporated within the Work constitutes direct or contributory patent infringement, then any patent licenses granted to You under this License for that Work shall terminate as of the date such litigation is filed.

4. Redistribution.

You may reproduce and distribute copies of the Work or Derivative Works thereof in any medium, with or without modifications, and in Source or Object form, provided that You meet the following conditions:

1. You must give any other recipients of the Work or Derivative Works a copy of this License; and

2. You must cause any modified files to carry prominent notices stating that You changed the files; and

3. You must retain, in the Source form of any Derivative Works that You distribute, all copyright, patent, trademark, and attribution notices from the Source form of the Work, excluding those notices that do not pertain to any part of the Derivative Works; and

4. If the Work includes a "NOTICE" text file as part of its distribution, then any Derivative Works that You distribute must include a readable copy of the attribution notices contained within such NOTICE file, excluding those notices that do not pertain to any part of the Derivative Works, in at least one of the following places: within a NOTICE text file distributed as part of the Derivative Works; within the Source form or documentation, if provided along with the Derivative Works; or, within a display generated by the Derivative Works, if and wherever such third-party notices normally appear. The contents of the NOTICE file are for informational purposes only and do not modify the License. You may add Your own attribution notices within Derivative Works that You distribute, alongside or as an addendum to the NOTICE text from the Work, provided that such additional attribution notices cannot be construed as modifying the License.

You may add Your own copyright statement to Your modifications and may provide additional or different license terms and conditions for use, reproduction, or distribution of Your modifications, or for any such Derivative Works as a whole, provided Your use, reproduction, and distribution of the Work otherwise complies with the conditions stated in this License.

5. Submission of Contributions.

Unless You explicitly state otherwise, any Contribution intentionally submitted for inclusion in the Work by You to the Licensor shall be under the terms and conditions of this License, without any additional terms or conditions. Notwithstanding the above, nothing herein shall supersede or modify the terms of any separate license agreement you may have executed with Licensor regarding such Contributions.

6. Trademarks.

This License does not grant permission to use the trade names, trademarks, service marks, or product names of the Licensor, except as required for reasonable and customary use in describing the origin of the Work and reproducing the content of the NOTICE file.

7. Disclaimer of Warranty.

Unless required by applicable law or agreed to in writing, Licensor provides the Work (and each Contributor provides its Contributions) on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied, including, without limitation, any warranties or conditions of TITLE, NON-INFRINGEMENT, MERCHANTABILITY, or FITNESS FOR A PARTICULAR PURPOSE. You are solely responsible for determining the appropriateness of using or redistributing the Work and assume any risks associated with Your exercise of permissions under this License.

8. Limitation of Liability.

In no event and under no legal theory, whether in tort (including negligence), contract, or otherwise, unless required by applicable law (such as deliberate and grossly negligent acts) or agreed to in writing, shall any Contributor be liable to You for damages, including any direct, indirect, special, incidental, or consequential damages of any character arising as a result of this License or out of the use or inability to use the Work (including but not limited to damages for loss of goodwill, work stoppage, computer failure or malfunction, or any and all other commercial damages or losses), even if such Contributor has been advised of the possibility of such damages.

9. Accepting Warranty or Additional Liability.

While redistributing the Work or Derivative Works thereof, You may choose to offer, and charge a fee for, acceptance of support, warranty, indemnity, or other liability obligations and/or rights consistent with this License. However, in accepting such obligations, You may act only on Your own behalf and on Your sole responsibility, not on behalf of any other Contributor, and only if You agree to indemnify, defend, and hold each Contributor harmless for any liability incurred by, or claims asserted against, such Contributor by reason of your accepting any such warranty or additional liability.
```

### Grammar files (check code and `.pgm`/Classic Analysis)

All present in the repo (sizes in bytes from the git tree):

- `Epi.Core.Interpreter/grammar/EpiInfo.Analysis.Grammar.grm` (63,920) and
  `EpiInfo.Analysis.Grammar.cgt` (869,611) — GOLD-parser grammar for Classic
  Analysis (`.pgm`) commands. `.grm` header: `"Name" = 'Epi Info 7 Grammar'`,
  `"Author" = 'Epi Info Team LM-ESAS and McKing Consulting  jhaines'`.
- `Epi.Core.EnterInterpreter/grammar/EpiInfo.Enter.Grammar.grm` (29,981) and
  `EpiInfo.Enter.Grammar.cgt` (264,281) — Enter check-code grammar.
- `Epi.Core/Resources/EpiInfoGrammar.txt` (56,921) + `.cgt` (416,336) and
  `EpiInfo.Enter.Grammar.txt` (56,921) + `.cgt` (416,461) — resource copies.
- `EpiMenuCommandPlugin/Grammar/working-menu.grm` (5,637) + `.cgt` (80,917).

### Sample projects and data

All present in the repo:

- `Epi.Core/Projects/Sample/` — `Sample.prj` (656), `Sample.mdb` (3,674,112),
  `SampleDB.db` (1,417,216), `Oswego.cvs7` (13,044), plus Excel/CSV/shape
  examples (`Case Control DatabaseExample.xls`, `MXState.shp`, …). `Sample.mdb`
  is the canonical Epi Info sample database that ships the Oswego outbreak
  table.
- `Epi.Core/Projects/<Name>/` — 20+ bundled projects each with `.prj` + `.mdb`:
  Cholera, EColi, H1N1(x3), HansensDisease, HIV, HuttYouthSurvey, JohnSnow,
  LandmineStudy, Legionellosis, Lyme, Mumps, Nutrition, PHIN, Salmonellosis,
  STEPS, CommunityHealthAssessment, BeefPrevention (`BeefPrevention.prj`).
- `Epi.Core/Resources/Training Projects/CommunityHealthAssessment/` — duplicate
  training copy of that project.
- `.pgm` files: **none** (0 matches for `*.pgm` in the recursive tree). `.pgm`
  scripts for Task 3 must be written against the grammar, not copied.

## Decision

**Reuse permitted with notice (Apache-2.0).**

`license.md` at `master` is the verbatim Apache License 2.0. Epiora may therefore
reuse Epi Info 7 code, grammar files and sample data (and create derivative
works, including in an AGPL-3.0 codebase — Apache-2.0 is compatible with
GPL/AGPL-3.0) provided the Apache §4 conditions are met:

1. Recipients receive a copy of the Apache-2.0 licence.
2. Modified files carry prominent notices stating they were changed.
3. Copyright, patent, trademark and attribution notices from the source are
   retained in derivative source. (There is no `NOTICE` file, so no NOTICE
   propagation is required.)
4. Apache §6 does **not** grant the "Epi Info" trademark — CDC's mark
   (`AssemblyTrademark` in `SolutionInfo.cs`). Epiora may describe compatibility
   with Epi Info but must never brand itself or outputs "Epi Info".

The spec's "Apache 2.0 or public domain" ambiguity resolves to **Apache-2.0 with
notice**: although the README calls Epi Info "a public domain suite" and CDC is a
US government agency, the repository's explicit licence grant is Apache-2.0, and
the repo contains contractor and community contributions (e.g. the grammar
author line names McKing Consulting), so the Apache grant is the safe reading.

This is **OWNER REVIEW** per the plan's Task Index (licence reading): the owner
confirms this reading before downstream tasks rely on it.

## Consequences

- Task 3 (Epi Info 7 on Windows CI): building from source in CI is permitted;
  CI must not redistribute Epi Info binaries or outputs embedding its code
  without Apache notices.
- Task 4 (mdbtools wasm): `Epi.Core/Projects/Sample/Sample.mdb` may be copied
  into `spikes/mdb-wasm/test/data/` under Apache-2.0 — the copy must ship with a
  notice file naming its origin and licence.
- Grammar files (`.grm`/`.cgt`) may be used to author Epiora's `.pgm` and
  check-code grammars; adapted derivatives carry change notices.
- `.pgm` test scripts cannot be harvested from the repo (none exist); Task 3
  writes them against the grammar.
- Third-party assets inside the repo are not covered by Apache-2.0:
  `EpiDashboard/license.txt` is CC BY-ND 3.0 (no derivatives) — Epiora must not
  reuse those icons. The interpreter also links `com.calitha.goldparser`
  (Calitha GOLD parser engine); its licence must be checked before any reuse of
  that component in later tasks.
- Spec §17 D2 is resolved by this entry (see `docs/decisions.md`).

## Licences

- Subject of this spike: `Epi-Info/Epi-Info-Community-Edition` @
  `4cd353c62c40b78d9d1f013b0f5a2bb685a5fca6` — Apache License 2.0
  (`license.md`, quoted above).
- Nested third-party assets noted during inspection: Modern UI icons —
  CC BY-ND 3.0 (`EpiDashboard/license.txt`); `com.calitha.goldparser` linked by
  `Epi.Core.Interpreter` — licence not determined in this spike (flagged above).
- This spike adds no code and no dependencies to Epiora; tooling used was the
  `gh` CLI (MIT) against the GitHub REST API.
