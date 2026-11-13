# D9 — Competitor scan: are Epiora's USP claims true?

**Date:** 2026-11-12 · **Resolves:** open decision D9 (platform spec §17) · **Checks:** platform spec §5
(USPs), with §2, §13 and v1.0 spec §1 as context · **Method:** web searches plus primary sources (product
sites, GitHub repositories and their commit histories via the GitHub API, release notes, forum
announcements). Anything not confirmed from a primary source is marked **unverified**.

## Summary

1. **No hero USP is fully "Already done" by anyone, but none is fully unique either.** Each one is partly
   matched; what no one offers is the *combination* in one free, offline, no-code tool.
2. **The biggest threat is "Epi Info AI"** (Apache-2.0, v0.2.0, September 2026): a browser-first, offline,
   WASM rebuild of Epi Info. Its source is mirrored on GitHub, and its README names a CDC GitLab (`git.cdc.gov`)
   as the main repository. It already claims parity registers, receipts, Access conversion, small-cell control
   and an AI that "never" acts as the statistical authority. That overlaps Epiora's USPs 2, 5, 7 and 8.
   Whether CDC officially backs it is **unverified**.
3. **The Epi Info Community Edition is not maintained.** Its last commit was 2025-04-09, and none of its forks
   has a commit after May 2025. Guides that say it is "maintained outside CDC" are wrong.
4. **OpenEpi is frozen** (last version 3.03, 2014; MIT licence). Small, unfunded OpenEpi successors have appeared
   in 2026: Wepisia, EpiPlus/EpiCalc and the commercial MetricGate. None is dominant, so Epiora's v1.0 aim is still open.
5. **Collection platforms are moving up into analysis**, mostly with AI: Kobo's AI analysis, SurveyCTO's AI
   credits, CommCare Companion, DHIS2 Chap outbreak alerts, and BAO's Analytics Platform with Kobo and ODK connectors.
6. **Reword four claims before any marketing use:** the AI copilot (USP 8), the free tier (USP 9), the
   "Epi Info home" (USP 5), and the "only" framing of USPs 1–3. Suggested wording is in the last section.

## Verdict table

| # | USP (spec §5) | Verdict | Closest matches |
|---|---|---|---|
| 1 | Keep your collection tool, add the epidemiologist | **Partly matched** | BAO Analytics Platform (DHIS2 + Kobo + ODK connectors, general BI); EpiGraphHub (DHIS2, Kobo); Go.Data `dhis2togodata` |
| 2 | Analysis receipts | **Partly matched** | jamovi and JASP (R syntax per analysis); MetricGate (shows R code); Epi Info AI (plan/result/receipt pipeline, checksummed manifests) |
| 3 | Outbreak in a box | **Partly matched** | Go.Data (WHO templates, contacts, chains); SORMAS (chains, epi curve, sitreps, offline Android); R `sitrep` (measles, cholera and other report templates); MetricGate (attack-rate and epi-curve calculators) |
| 4 | Whole pipeline offline, even on cheap phones | **Partly matched** | DHIS2 Android (offline capture plus offline line lists); SORMAS Android (offline); Wepisia and Epi Info AI (browser-local) |
| 5 | One-click home for Epi Info users, with a parity report | **Partly matched (by Epi Info AI)** | Epi Info AI (49-command parity register, Access `.mdb`→SQLite); EIAnalysis (CSV/JSON only) |
| 6 | Explain mode (plain language plus maths with live numbers) | **Unique as specified** (weak moat) | MetricGate (R code plus agent explanations); OpenEpi documentation (**unverified**) |
| 7 | Privacy by default, automatic small-cell suppression, tiered sharing | **Partly matched** | Epi Info AI (small-cell control in governed examples); Wepisia ("no data leaves your device"); agencies' own scripts |
| 8 | AI copilot that never states a number the engine did not compute | **Partly matched; same principle already published by Epi Info AI** | Epi Info AI "Epi Assist"; Kobo (AI answers marked verified or unverified); MetricGate agent runs R (whether it ever writes numbers itself is **unverified**) |
| 9 | A free tier better than Epi Info | **Not yet checkable** | Free today: OpenEpi, Wepisia, EpiPlus, jamovi, JASP, EpiData, Kobo Community (nonprofits) |

## The landscape (November 2026)

### Epi Info itself, and forks

- **CDC position.** Development and support ended in September 2025 (CDC gave 1 Sept and 30 Sept). Its
  alternatives sheet says CDC "will not offer a replacement" and lists tool mixes: Kobo, ODK, REDCap, SurveyCTO,
  Enketo, EpiCollect5 and others for collection; R, Python, SAS, Stata, Power BI and Tableau for analysis;
  ArcGIS and QGIS for maps; OpenEpi and Statulator for calculators. Wikipedia reports that as of August 2026,
  CDC had removed nearly all Epi Info content from its website. The live `cdc.gov/epiinfo/sunsetnews.html`
  page now returns 404.
- **Epi-Info-Community-Edition** (GitHub). Not archived, but its last push was 2025-04-09 (last release 7.2.7.0,
  2025-03-09). It has 11 forks; the newest activity is `erik1066` at 2025-05-12, and no fork has a commit after
  the sunset. **Not maintained.** The CASRAI guide (reviewed 2026-08-31) calls it "maintained outside CDC";
  the commit history does not support that.
- **CDC `epiinfo` Python package** (Apache-2.0, last push 2025-07-14). Has `Frequencies`, `Means`,
  `TablesAnalysis`, `LogisticRegression`, `LogBinomialRegression` and `LineList`, with CSV, JSON, SQLite and
  encrypted-sync import (no `.mdb` or `.prj`). Frozen since the sunset.
- **Epi Info AI** (`drlove/epi-info-ai` on GitHub; README names `git.cdc.gov/epi-info-ai/epi-info-ai` as the main
  repository). Apache-2.0. The GitHub mirror was created 2026-09-23, and the last mirrored commit (v0.2.0) is from
  the same day. It is browser-first and offline-first, with a Rust/WASM statistics kernel, SQLite-WASM/OPFS storage,
  a 49-entry Classic Analysis parity register (status page dated 2026-09-22: "No Classic Analysis command
  currently meets every legacy-parity gate"; 30 entries browser-verified), Check Code, maps, `FILE CONVERT`
  from Access `.mdb` to SQLite or DuckDB, JupyterLite validation notebooks, a foodborne-outbreak validation
  roadmap, FETP demo runbooks, and "Epi Assist", an optional AI (local IBM Granite, OpenAI or Anthropic)
  described as "never statistical authority". **Unverified:** whether this is an official CDC project, whether
  it is funded, and how active the private GitLab is. Mirrored activity stops on 2026-09-23.
- **EIAnalysis** (`zfj4/EIAnalysis`, John Copeland, who also committed to CDC's Python package). Django/HTMX
  single-page app over the `epiinfo` package; "much of the … functionality as the Epi Info 7 Analysis
  Dashboard". v0.4.1.3, last commit 2026-04-23, no licence file, 0 stars. A local Python install, not offline-web
  or mobile.
- No other Epi Info fork or edition turned up in GitHub searches for repositories pushed after 2025-09-01.

### OpenEpi and calculator successors (v1.0 spec §1)

- **OpenEpi.** Latest version 3.03 (2014-09-22) per Wikipedia (the citation used by Wepisia is v3.01,
  2013-04-06); MIT licence. The site is still online. No maintenance announcement found; **frozen**. No funded successor found.
- **Wepisia** (Xcept-Health; MIT; v0.1.0-alpha; Zenodo DOI). "A complete architectural rewrite of OpenEpi" in
  React/TypeScript, "validated against OpenEpi v3", runs locally in the browser, has 6 languages including Mooré,
  Wolof, Swahili and Hausa, plus SIR/SEIR simulation, maps and an in-browser R IDE (WebR). 3 stars; last commit
  2026-05-11, so possibly stalled.
- **EpiPlus / EpiCalc** (PublicHealth Tech Lab, `whlee5503-dot`; MIT). Offline-capable web calculators that
  "fill the gaps left by OpenEpi" (survey sampling, DALY/PAF, age standardisation, meta-analysis). v1.0.0 on Zenodo
  (September 2026). The README says it is "preparing for submission" to the Digital Public Goods Alliance. Last
  push 2026-09-27.
- **MetricGate** (commercial; owner not stated on its site). Browser R platform aimed at epidemiologists
  (posts from April and July 2026): Mantel–Haenszel, attack rate, epi curve, SMR, screening metrics and more; shows
  the R code; has an "Agent" that runs analyses; free tier of about 10 runs a month with a 250 KB upload limit;
  **online only**. It contrasts itself directly with OpenEpi (summary counts only) and Epi Info 7 (Windows only).
- **Episia** (Xcept-Health; MIT). A Python library "validated against OpenEpi", with a DHIS2 client; medRxiv
  preprint April 2026. A library, not an app.

### EpiData

Manager 4.7.0, EntryClient 4.7.0 and Analysis 3.3.0, all dated 22 April 2024. The test-versions page lists nothing
newer than 2022, and a November 2025 mailing-list thread still refers to Analysis 3.3.0. Licence not stated on
the download page (**unverified**). Alive but slow, and not positioned against Epi Info.

### Collection platforms

| Tool | Status (Nov 2026) | Analysis / AI direction | Price anchor |
|---|---|---|---|
| KoboToolbox | Very active (kpi 2.026.37g, 2026-11-11), AGPL | May 2026: AI analysis of audio responses (gpt-oss-120b), answers marked verified or not; transcription and translation | Community free for nonprofits (5,000 submissions/month). Paid prices from third-party listings only: ~$129–199/month (official page returned 403; **unverified**) |
| ODK Central | Active (v2026.3.1, 2026-09-28), Apache-2.0 | Entities (offline by default), Web Forms; "real-time dashboards"; OData to Power BI, R or Python | Cloud $199 / $499 per month; no free cloud tier; self-hosting free |
| REDCap | 16.x LTS (2026) | Institution-enabled AI (text summary, translation) | Free to consortium institutions |
| SurveyCTO | 2.83.x (June 2026) | AI form design; AI audio transcription; "AI credits" in every plan | $225–630/month annual; free community tier (200 submissions/month) |
| CommCare | Active | "CommCare Companion" AI for app building; new data pipeline | not checked |

The §13 anchors still hold. ODK Cloud starts at $199/month and SurveyCTO's range is now $225–630 annual (or
$250–700 monthly). SurveyCTO also has a free community tier, which §13 does not mention.

### Outbreak and surveillance platforms

- **Go.Data** (WHO; GPL-3.0; open source since April 2024; DPG). Case and contact management, contact follow-up,
  transmission-chain views, WHO standard outbreak templates (influenza, yellow fever, mpox, cholera, Ebola),
  mobile apps, Power BI templates, `dhis2togodata`. Release notes v50.3 (Jan 2026); last repo commit 2026-06-03.
  Downloads by email request. No food-specific 2×2 or Mantel–Haenszel found.
- **SORMAS** (GPL-3.0; DPG). v1.104.0 (2026-07-14) adds custom forms, salmonellosis and shigellosis, and a
  NGSurvey import. It has contact follow-up, infection-chain views, an epi curve on the dashboard, situation
  reports, and an offline Android app. A government-scale system, not a laptop tool.
- **DHIS2** (core very active; v43 plus Android 3.4 in 2026). The Android app has offline line-list analytics;
  Chap core v2 adds outbreak alerts and prediction; a "DHIS2 AI Assistant" was a 2026 app competition finalist.
  BAO Systems' **Analytics Platform** is "an open-source data integration and analytics layer" with no-code
  Kobo and ODK connectors, Superset dashboards, natural-language-to-SQL and R/Python. This is the closest product
  to USP 1, but it is general BI, not epidemiological methods.

### R, Stata, jamovi and JASP

- **R4EPI `sitrep`** v0.4.1 (CRAN, 2026-02-24): R Markdown outbreak templates (AJS, cholera, diphtheria, measles,
  meningitis) that produce Word reports, with helpers for preparing offline use. Needs R skills.
- **Applied Epi Epi R Handbook**: repository active (pushed 2026-11-12). Training material, not software.
- **Epiverse-TRACE**: cleanepi 1.1.2 (Oct 2025), linelist 2.0.1, epiparameter 0.4.1, epichains, simulist 0.7.x.
  Building blocks for analysts who write R, not an end-user tool.
- **jamovi** 2.7.30 (May 2026): "syntax mode" gives equivalent R code (`jmv`) for each analysis, but leaves out the
  data-import step. **JASP** 0.99.0 (Oct 2026; AGPL): R syntax per analysis. Neither has a dataset hash or engine
  receipt, and no epidemiology 2×2 module was found for jamovi.
- **Stata**: commercial; not re-checked.

### New tools marketed as an Epi Info replacement since 2025

Only **Epi Info AI**, **EIAnalysis**, **Wepisia** (OpenEpi), **EpiPlus** (OpenEpi gaps) and **MetricGate**
(implicitly) were found. Epiora itself appears in search results as "the only result that explicitly positions
itself as an Epi Info successor", described from its public README as "not usable yet". Esri, CASRAI and
CDC point users to combinations of existing tools.

## USP by USP

**1. Keep your collection tool, add the epidemiologist: Partly matched.** Connecting Kobo and ODK to an analysis
layer exists today (BAO Analytics Platform, EpiGraphHub, ODK's OData feed to Power BI, Kobo→OpenFn→DHIS2). What
was not found: a no-code layer that pulls Kobo, ODK, REDCap *and* DHIS2 and then runs *epidemiological* methods
(2×2, Mantel–Haenszel, attack rates, outbreak workflow) on the result. That part holds. The "adoption without
changing field workflows" pitch is now also BAO's.

**2. Analysis receipts: Partly matched.** "Reproduce in R" for each analysis is already ordinary in jamovi
(syntax mode) and JASP (R syntax), and MetricGate shows its R code. Epi Info AI designs around
"plan → result/receipt" and checksummed manifests. Not found anywhere: one receipt that bundles the dataset hash,
method and options, engine version, *validation reference*, and an R script that CI proves reproduces the number.
The verifiable part (validation reference plus CI-proven reproduction) is the defensible core.

**3. Outbreak in a box: Partly matched.** Go.Data and SORMAS cover cases, contacts, chains, templates, epi curves
and sitreps, but they are server systems aimed at governments, and neither has a food-specific 2×2 with
Mantel–Haenszel. `sitrep` covers templated sitreps in R. MetricGate has attack-rate and epi-curve calculators.
Epi Info AI has a foodborne validation roadmap. The combination (laptop-scale, offline, no code, automatic
classification, food 2×2 with MH, auto-updating multilingual sitrep) was not found. "First case to first sitrep
in under an hour" has no competitor benchmark; it is an internal target, not a comparative claim.

**4. Whole pipeline offline, even on cheap phones: Partly matched.** Offline *collection* is common (Kobo, ODK,
SurveyCTO, CommCare, SORMAS, DHIS2). DHIS2 Android also has offline line lists and downloadable basemaps.
Browser-local analysis exists (Wepisia, Epi Info AI). No current tool was found that does forms, entry,
*analysis*, maps and reports all offline on a low-end Android phone. Epi Info Mobile did some of this and left
the app stores in February 2025, so the gap is real today.

**5. One-click home for Epi Info users, with a parity report: Partly matched by Epi Info AI.** It already
publishes a command-by-command parity register and converts Access `.mdb` tables. It does not (per its docs)
open Epi Info `.prj` projects, and its own status page says no command has reached full legacy parity. EIAnalysis
reads CSV and JSON only. So "one click from `.prj` with a parity report" is still unclaimed, but the parity-report
idea is not unique.

**6. Explain mode: Unique as specified.** No tool was found that shows a plain-language reading *and* the formula
with the run's own numbers substituted. MetricGate explains via R code and an agent. Whether OpenEpi's
per-module documentation counts as static formula help is **unverified**. Easy to copy, so a weak moat.

**7. Privacy by default with automatic small-cell suppression and tiered sharing: Partly matched.** "Data
never leaves your device" is now common (Wepisia, Epi Info AI, OpenEpi offline). Automatic small-cell suppression
was not found as a default in any end-user epidemiology tool. Agencies script it themselves (for example BC's
`suppress-for-release`), and Epi Info AI shows small-cell control only in governed examples. The suppression
default plus tiered sharing is still distinct.

**8. AI copilot that never states a number the engine did not compute: Partly matched; the principle is
published.** Epi Info AI's design says the LLM "should never be the thing calculating an odds ratio" and makes AI
"never statistical authority". Kobo marks AI answers as verified or unverified. BAO and MetricGate run real
queries or R behind their agents. Linking *every* number to its receipt is still distinct, but "never invents
numbers" alone is becoming a baseline expectation.

**9. A free tier better than Epi Info: Not checkable yet.** Epi Info is frozen, so "better" is a low bar in
features but a high bar in breadth. Epi Info had forms, mobile, maps, StatCalc and AnthStat for free, and Epiora
reaches parity only at v1.5. Free rivals for parts of it: OpenEpi, Wepisia, EpiPlus, jamovi, JASP, EpiData, Kobo
Community, self-hosted ODK.

## Threats

1. **Epi Info AI.** Same architecture and pitch (WASM engine, offline browser, receipts, parity register,
   deterministic AI), a permissive licence, apparent CDC hosting and the "Epi Info" name. If CDC or a partner
   adopts it publicly, it becomes the default "home for Epi Info users". **Action:** watch the GitHub mirror and
   the Pages demo monthly. Its status and funding are unverified.
2. **Collection platforms adding analysis and AI.** Kobo, SurveyCTO, CommCare, DHIS2/Chap and BAO are all
   investing; USP 1 depends on them *not* adding epidemiological methods.
3. **Digital Public Good positioning.** Go.Data and SORMAS are DPGs, and EpiPlus says it is preparing a DPG
   submission. Epiora plans to apply after v1.2, so others may be listed first.
4. **Commercial browser-R tools** (MetricGate) that target the same users with AI agents and R-backed numbers.
5. **Low threat:** the Community Edition (dormant), EpiData (slow), and the OpenEpi successors (small and
   unfunded as far as found; funding **unverified**).

## Gaps and opportunities the roadmap misses

- **Localisation comes late (v4.3).** Wepisia already ships West African languages, and Epi Info had French,
  Spanish and Portuguese users. For a "home for Epi Info users" and an OpenEpi replacement, at least French,
  Spanish and Portuguese UI before v2 is worth considering.
- **A scriptable Epi Info reference.** CDC's `epiinfo` Python package (Apache-2.0) implements Frequencies, Means,
  Tables and regressions in Python. It could give v1.0/v1.1 a *scriptable* Epi Info method source alongside the
  desktop. That it matches Epi Info 7 desktop numerically is **unverified**.
- **The OpenEpi licence is MIT** (Wikipedia; Wepisia relies on it). This may make OpenEpi's documentation and
  worked examples reusable as test material. Confirm before relying on it, as D2 did for Epi Info.
- **The calculator scope users now expect** beyond OpenEpi: EpiPlus covers survey sampling, DALY/PAF, age
  standardisation and meta-analysis. These are worth checking against the v1.0 Task 1 inventory as candidates
  for later.
- **FETP / teaching packs.** Epi Info AI ships FELTP demo runbooks and JupyterLite labs. Epiora's Education tier
  arrives at v2.5; a free teaching pack earlier would cost little.
- **AI for qualitative and free-text data** (Kobo, REDCap AI summaries). Not in the roadmap; likely fine to skip,
  but note it.
- **Watch, don't copy:** DHIS2 Chap-style prediction and climate-health work. This is outside Epiora's scope.

## User signals: r/epidemiology thread (added 2026-11-13)

**Source:** an r/epidemiology thread in which the OP (`swisscheesemodel`) asks whether Epi Info is used as much
today as it was 10–15 years ago. The share link is `https://www.reddit.com/r/epidemiology/s/BonYXcy74k`; the
conductor's fetcher is blocked by Reddit, so the owner pasted the text and screenshots on 2026-11-13. The
comments are about **3 years old**, so they predate the 2025 sunset. This is anecdote from a handful of people,
not a survey. It is used to sharpen the roadmap's priorities, not to measure the market.

| Signal | Who said it | What it means for Epiora |
|---|---|---|
| "Epi Info has never really been widely adopted"; Excel does it; OpenEpi-style sites are textbook-related | US/UK academic epidemiologists (most-upvoted thread) | Outside some segments, "Epi Info users" is a smaller market than spec §1 implies. Treat stranded Epi Info users as the **first customers to win**, not the whole market. Lead with *validated* results rather than *built-in formulas*. A hand-rolled OR in Excel is easy; a correct Fisher, mid-p or Mantel–Haenszel CI is not. |
| "It's definitely still in use by Ministries of Health, especially in LMICs. There is a push to move to R but EI is still widely used." | Public health, LMIC | Confirms the core segments in spec §4. After the 2025 sunset these users have no supported tool. |
| Rural health department using Epi Info 7 as a programme database: "worst for large datasets… left with a mess", moving to REDCap; uses Excel and SAS most | US local health department | A stranded legacy user. Legacy import has to come early, and REDCap is where these users go next, so a REDCap bridge matters as much as the import. |
| "Only 1 person can work with the data set at a time… We use REDCap… then export to SAS" | Research team | Multi-user work blocks adoption, and "REDCap for collection, a stats package for analysis" is the common pattern. That calls for an early read-only REDCap/Kobo/ODK pull and an interim way to merge between devices before v2.0 sync. |
| "I mainly used it for power calculations" | Former user | StatCalc (v1.0 calculators) is a real way in. Calculators open from a link with no project (v1.0 G3), so they also cost nothing to try. |
| "You need to know which stats calc to apply"; "tools do not know what confounders to control for"; "latent variable analysis"; "complex causal inference" | Academics | Method guidance is unmet everywhere (Excel, R and Epi Info all assume you know). This points to a "which calculator?" guide, engine-computed assumption notes, a confounder (DAG) helper beside regression, and causal and latent-variable methods through R. |
| "R is the gold standard"; "most schools still teach SAS"; Stata in econometrics; "none of the MPH programs teach [Epi Info]" | Academics | Bridge to R rather than compete with it: the exported R script verified in CI (v1.0), plus SAS as well as Stata scripts, and an R package that reads Epiora exports. University courses are not the first pilot. |
| "There is no one to sue" with open-source R; the NHS stays on SAS for accountability | UK/commercial users | Some buyers want a named, accountable vendor. Epiora can offer one: open (AGPL) code, a public validation dossier from v1.0, and a paid support agreement for validated releases. |
| tidyverse is slow for real-time dashboards and maps | Analysts | Supports the Rust/wasm engine and DuckDB for interactive dashboards (no change needed). |

**Roadmap changes the owner accepted (2026-11-13), applied to the platform spec §14 in the same PR:**
1. Phase 1 reordered so the user who still has stranded Epi Info data is served first: legacy Epi Info import
   with data management moves to v1.1. Regression ships with a read-only REDCap, KoboToolbox and ODK Central
   import in v1.2. Forms and entry come in v1.3, with offline phone entry and merging between devices by file.
2. v1.0 gains a "which calculator?" guide, engine-computed assumption notes and a public validation page.
3. The first pilot is a field epidemiology training programme (FETP), or an LMIC or local health department
   with legacy Epi Info data. University courses run in parallel.
4. SAS script export joins Stata, verified the same way (open decision D10).
5. A DAG adjustment-set helper beside regression (D11). Causal-inference and latent-variable methods come
   through R in v3.3.
6. French, Spanish and Portuguese UI move up to v1.6 (this scan's localisation gap). The validation dossier is
   published from v1.0, and v4.4 adds an independent audit of it.

## Suggested rewording

- **USP 1:** "Keep Kobo, ODK, REDCap or DHIS2 for collection. Epiora adds the epidemiology (2×2, stratified
  analysis, outbreak workflows) on top, without code." Drop any implication that nobody else connects to Kobo or ODK.
- **USP 2:** "Every number is checked: each result carries its dataset fingerprint, method, engine version, the
  reference it was validated against, and an R script our CI proves gives the same answer." Lead with
  *validated and CI-proven*, not "reproduce in R", which jamovi and JASP already offer.
- **USP 3:** keep it, but say "outbreak investigation on a laptop, offline", and only publish the "under an hour"
  target once it has been measured.
- **USP 4:** "Collect, analyse, map and report offline on a low-cost Android phone." Specific and, as far as
  this scan found, true once v2.4 ships. Don't claim it before then.
- **USP 5:** "Opens your Epi Info 7 projects (`.prj`/`.mdb`) and shows, command by command, how the results
  compare." Avoid "the only home for Epi Info users".
- **USP 8:** "AI that only reports numbers the engine computed, each linked to its receipt." Don't present the
  principle as unique; Epi Info AI states it too.
- **USP 9:** replace "better than Epi Info" with a measurable statement, for example "free and open source
  (AGPL), offline, every OpenEpi calculator plus Epi Info's analysis commands". Use it after v1.5 for parity
  claims and after v1.0 for the OpenEpi claim.

## Not verified in this scan

Whether Epi Info AI is CDC-sanctioned or funded, and how active its private repository is; whether any OpenEpi
successor is funded; OpenEpi's per-module documentation content; EpiData's licence; Kobo's paid prices (official
page blocked); CommCare pricing; MetricGate's owner and whether its agent ever writes numbers itself; Stata's
current epidemiology features.

## Sources

Accessed 2026-11-12 unless noted. GitHub facts (commit dates, releases, licences, forks) come from the GitHub API on
the same date.

- Platform spec: `docs/superpowers/specs/2026-10-28-epiora-platform-design.md` §2, §5, §13, §17; v1.0 spec
  `docs/superpowers/specs/2026-11-12-epiora-v1.0-calculators-and-means-design.md` §1
- CDC Epi Info alternatives sheet (v1, 2024-12-02, via mirror): https://restoredcdc.org/www.cdc.gov/epiinfo/pdfs/userguide/Epi_Info_Sunset_Alt_Draft_508.pdf
- CDC sunset page (now 404): https://www.cdc.gov/epiinfo/sunsetnews.html
- Epi Info, Wikipedia: https://en.wikipedia.org/wiki/Epi_Info
- CASRAI guide (reviewed 2026-08-31): https://casrai.org/guides/epi-info
- Epi Info Community Edition: https://github.com/Epi-Info/Epi-Info-Community-Edition and its forks list
- CDC `epiinfo` Python package: https://github.com/Epi-Info/epiinfo
- Epi Info AI: https://github.com/drlove/epi-info-ai (README, `wasm/docs/status.md`, `wasm/docs/project.md`,
  `wasm/docs/architecture.md`)
- EIAnalysis: https://github.com/zfj4/EIAnalysis
- OpenEpi: https://www.openepi.com/ ; https://en.wikipedia.org/wiki/OpenEpi ; mirror https://github.com/HIT-Tools/openepi (last commit 2020)
- Wepisia: https://github.com/Xcept-Health/wepisia ; https://wepisia.vercel.app/
- Episia: https://github.com/Xcept-Health/episia ; https://www.medrxiv.org/content/10.64898/2026.04.17.26350337v1.full
- EpiPlus / EpiCalc: https://github.com/whlee5503-dot/EpiPlus ; https://github.com/whlee5503-dot/EpiCalc
- MetricGate: https://metricgate.com/ ; https://metricgate.com/blogs/statistics-software-for-epidemiologists/ ; https://metricgate.com/blogs/outbreak-investigation-statistical-tools/
- EpiData: https://www.epidata.dk/download.php ; https://www.epidata.dk/testing.php ; https://lists.umanitoba.ca/archives/list/epidata-list@lists.umanitoba.ca/latest
- KoboToolbox: https://community.kobotoolbox.org/t/may-2026-feature-update/76236 ; https://github.com/kobotoolbox/kpi ; https://www.kobotoolbox.org/pricing (403; prices from https://www.saasworthy.com/product/kobotoolbox/pricing)
- ODK: https://getodk.org/ (pricing) ; https://github.com/getodk/central ; https://forum.getodk.org/t/odk-central-v2026-2-entities-filtering-and-web-forms-as-default/58254
- REDCap: https://ctsi.ucla.edu/whats-new ; https://ctsi.wakehealth.edu/announcements/2025/09/new-ai-features-in-redcap
- SurveyCTO: https://www.surveycto.com/pricing/ ; https://docs.surveycto.com/release-notes/
- CommCare: https://commcare.dimagi.com/blog/whats-new-in-commcare-spring-2026/ ; https://dimagi.com/commcare-product-updates-spring-2026-webinar-highlights/
- Go.Data: https://github.com/WorldHealthOrganization/godata (incl. `docs/data-mgmt/3-templates/outbreak-templates.md`, `release-notes/v50.3.md`) ; https://www.who.int/tools/godata
- SORMAS: https://www.sormas.org/learn/news/sormas-1104-is-here ; https://www.sormas.org/sormas/features ; https://github.com/SORMAS-Foundation/SORMAS-Project
- DHIS2: https://dhis2.org/android/version-3-0/ ; https://community.dhis2.org/t/dhis2-newsletter-july-2026-highlights/75516 ; BAO Analytics Platform https://dhis2.org/?p=25654
- EpiGraphHub: https://www.healthdatacollaborative.org/fileadmin/uploads/hdc/Documents/2022/HDC_Webinars/epigraphhub-new.pdf
- R4EPI `sitrep`: https://cran.r-project.org/web/packages/sitrep/index.html ; https://github.com/R4EPI/sitrep
- Applied Epi handbook: https://github.com/appliedepi/epiRhandbook_eng
- Epiverse-TRACE: https://github.com/epiverse-trace (cleanepi, linelist, epiparameter, epichains) ; https://epiverse-trace.github.io/
- jamovi syntax mode: https://docs.jamovi.org/_pages/um_6_syntax_mode.html ; https://github.com/jamovi/jamovi
- JASP R syntax: https://jasp-stats.org/?p=9266 ; https://github.com/jasp-stats/jasp-desktop
- Small-cell suppression practice: https://andriy.rbind.io/publication/koval-2019-suppress-for-release/ ; https://kb.data.chhs.ca.gov/ddg/4.-statistical-de-identification/4.4-statistical-masking
