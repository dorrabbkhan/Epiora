# Epiora — platform design and roadmap

**Status:** draft for owner review · **Date:** 2026-10-28 · **Owner:** Dorrabb
**Name:** Epiora (chosen 2026-10-28; formal trademark clearance is open decision D1, §17).

This is the platform-level spec: intent, decisions, architecture, modules, integrations,
security, validation, business model and the versioned roadmap. It is deliberately too large
for one implementation plan. Each roadmap version (§14) gets its own spec → plan → agent cycle,
as in NeoPhylo; the first is v0.1 (spikes).

---

## 1. Intent

**What the owner asked for.** A complete, modern, premium successor to CDC's Epi Info: every
existing Epi Info capability, plus the features a modern product should have, built over time
into a polished production product for universities, NGOs, health departments, ministries,
WHO-linked teams and US agencies. No deadline; the goal is to build something good and big.
It must be monetizable.

**Constraints the owner set.**
- Solo developer with a full-time job; work is part-time and agent-driven (NeoPhylo's loop).
- Local-first data with optional sync (self-hosted or hosted by us).
- Open core: AGPL open core, closed paid layer.
- Full legacy Epi Info parity, except that writing Access files is deferred.
- All four new areas (outbreak/surveillance, interoperability, collaboration/reproducibility,
  AI), ordered easiest and most attractive first.
- Architecture approach A: Rust core + React app + R (WebR) on demand.

**Assumptions (correct these if wrong).**
- The NeoPhylo engineering discipline (reference corpus, never-loosened tolerances, fuzzing,
  owner gates) applies unchanged.
- The owner will recruit at least one epidemiologist adviser before v1.0 ships.
- English first; localization later (v4.3).

## 2. Context

Epi Info is CDC's free epidemiology suite, first released in 1985 for MS-DOS, on Windows since
2001. Modules: Form Designer (MakeView), Enter, Classic Analysis, Visual Dashboard, Map,
StatCalc and AnthStat (child growth), plus data packaging and import/export tools.

- CDC ended Epi Info Desktop product development and technical assistance on 1 Sept 2025 and
  removed downloads; Mobile left the app stores in Feb 2025; Secure Web Survey shut on
  1 Sept 2025 (CDC sunset page). A second CDC notice gives 30 Sept 2025 as the end of support.
- CDC "will not offer a replacement for Epi Info™" and gives no data-migration guidance. It lists
  a mix of collection tools, analysis languages, GIS and calculators users can combine.
- The Epi Info 7 source is on GitHub (`Epi-Info/Epi-Info-Community-Edition`, 2,458 commits,
  not archived). Wikipedia lists Apache 2.0; the repo describes itself as "public domain". The
  exact licence is open decision D2.
- New Epi Info 7 projects default to Microsoft Access 2000–2003 (`.prj` + `.mdb`); the only
  alternative was SQL Server (`.prj` holding connection details).

## 3. Goals, non-goals and success criteria

**Goals.**
1. Every Epi Info workflow can be done in Epiora, and legacy projects open in one click.
2. Four hero differentiators (§5) ship early, each in usable form.
3. Every number is verifiable: validated against reference software, reproducible, and receipted.
4. Works fully offline on a laptop or a low-end Android phone.
5. Adoptable by every segment over time, starting with those needing no certification.

**Non-goals.**
- Writing Access `.mdb` files (deferred indefinitely; revisit only on paying demand).
- Competing on raw data-collection price with KoboToolbox or ODK.
- Clinical-trial regulatory validation (FDA 21 CFR Part 11).
- An electronic health record or a full hospital information system.
- Replacing R or Stata for advanced statisticians; we bridge to them instead.

**Success criteria by milestone.**
- *Parity (after v1.5):* every Classic Analysis command in the parity list (§8.3) matches Epi Info
  7 and R on the reference corpus; the CDC sample projects import and their scripts run
  unchanged; the open app is listed as a Digital Public Good candidate.
- *Field-ready (after v2.5):* paying Team customers (NGOs or health departments) in at least
  two countries; at least three courses teaching with Epiora.
- *Powerhouse (after v3.4):* SOC 2 Type II report; the outbreak module used in at least one
  real investigation with a published or internal report.
- *Agency production (after v4.4):* a ministry or agency running a multi-level deployment.

## 4. Users and sequencing

| Segment | Needs | Enters at |
|---|---|---|
| Students, instructors, individual epidemiologists | Free, offline, explainable analysis; Epi Info replacement | v1.0 |
| Researchers | Reproducibility, REDCap, exports to R/Stata | v1.0, deepened v2.2 |
| NGOs and field teams | Offline mobile capture and analysis, Kobo/ODK, outbreak tools | v2.0–v2.4 |
| Local and state health departments | Teams, audit, privacy, surveillance, HIPAA | v2.1–v3.4 |
| Ministries, WHO-linked teams | Self-hosting, data sovereignty, DHIS2, Go.Data, multi-level | v3.2–v4.2 |
| US federal agencies | FedRAMP or agency-hosted deployment | v4.4 |

Order principle: segments that need no certification come first. Agencies adopt early through
self-hosting (§11), long before Epiora holds certifications itself.

## 5. Unique selling points

Hero USPs (each early version ships one in usable form):

1. **Keep your collection tool, add the epidemiologist.** Connect to Kobo, ODK, REDCap and DHIS2
   and act as the analysis and outbreak layer on top. Adoption without changing field workflows.
2. **Analysis receipts.** Every result carries the dataset version and hash, method and options,
   engine version, validation reference, and one-click "reproduce in R or Stata".
3. **Outbreak in a box.** Investigation templates (foodborne, measles, cholera, respiratory): case
   definitions with automatic classification, line list, epi curve, attack rates, food-specific
   2×2 tables with Mantel–Haenszel stratification, transmission chains, and an auto-updating
   multilingual situation report. Target: first case to first sitrep in under an hour.
4. **The whole pipeline offline, even on cheap phones.** Forms, entry, analysis, maps with offline
   basemaps, and reports with no connection.

Supporting USPs: a one-click home for Epi Info users with a parity report (5); explain mode, in
plain language and in maths with live numbers (6); privacy by default with automatic small-cell
suppression and tiered sharing (7); an AI copilot that never states a number the engine did not
compute (8); a free tier better than Epi Info (9).

Competitive claims in this section must be re-checked in a competitor scan before any marketing
use (open decision D9).

## 6. Decisions

| # | Decision | Rationale | Rejected alternatives |
|---|---|---|---|
| A1 | Local-first data, optional sync to a self-hosted or hosted server | Field work is offline; ministries need data in their control; cloud SaaS still possible | Cloud-only; self-hosted-only; desktop-only |
| A2 | Open core: AGPL-3.0 open part, closed paid layer, CLA for contributors | Digital Public Goods eligibility ("Open Licencing" indicator), auditable statistics, ministry procurement | Fully closed; source-available (BSL); fully open |
| A3 | Rust engine compiled to wasm and native; React + TypeScript app; R via WebR on demand | One validated engine everywhere; reuses NeoPhylo stack; R for niche methods | TypeScript + WebR for all statistics; server-centric R/Python |
| A4 | Custom record- and field-level sync protocol in Rust, conflicts to a review queue | Silent merges of case data are data-quality incidents; licences of off-the-shelf engines | PowerSync (Functional Source License: not open source); Electric (Apache-2.0, kept as fallback; adds Elixir); CRDT auto-merge |
| A5 | Legacy: read `.prj`/`.mdb` and SQL Server-backed projects, translate check code, run `.pgm` scripts; never write `.mdb` | Access is Epi Info 7's default store; writing is experimental in mdbtools and pointless after the sunset | Writing `.mdb`; data-only import; clean break |
| A6 | Boundaries from geoBoundaries; never GADM | GADM forbids commercial use and redistribution without permission | GADM |
| A7 | No SQLite Encryption Extension in the browser | SEE builds may not be hosted on public-facing sites | SEE |
| A8 | Hand-write only the hot statistical core; delegate niche methods to R; use existing libraries for distributions, linear algebra and data wrangling | Speed, offline, determinism where it matters; no reinvention elsewhere | All-Rust statistics; all-R statistics |

## 7. Architecture

### 7.1 Units

| Unit | Purpose | Depends on | Licence tier |
|---|---|---|---|
| `epiora-core` (Rust) | Statistics, form model, check-code language, `.pgm` compatibility interpreter, importers (CSV, Excel, stats files, `.prj`), receipts | `statrs`, `faer`/`nalgebra`, `calamine`, `rust_xlsxwriter`, ReadStat (C, via FFI), `arrow-rs` | Open |
| `epiora-mdb` | Read-only Access `.mdb` reader, mdbtools' LGPL libraries compiled to wasm and native; loaded only for legacy import | mdbtools (LGPL) | Open |
| `epiora-wasm` | Type conversion between engine and browser; no logic; TypeScript types generated from Rust | `wasm-bindgen`, `serde`, `ts-rs` | Open |
| `epiora-app` (React + TypeScript) | Form Designer, Entry, Analysis workspace, Dashboards, Maps, StatCalc, Growth, Outbreak, Reports; PWA, and the same code in a Tauri 2 shell for desktop, Android and iOS | ECharts, MapLibre GL JS, PMTiles, Typst (wasm), WebR | Open (premium screens gated) |
| Local store | SQLite on the device (browser private file system via sqlite-wasm; native SQLite in Tauri); every edit appended to a local change log | sqlite-wasm; SQLite3MultipleCiphers in Tauri | Open |
| `epiora-sync` (Rust + Postgres) | Receives change logs, orders them, detects field conflicts, sends updates; connectors run here so credentials never sit on devices | Axum, Postgres | Open (basic) |
| `epiora-cloud` | Multi-tenant hosting, billing, SSO and SCIM, enterprise admin, compliance tooling, AI gateway, managed self-hosting | Keycloak, Paddle | Closed |

### 7.2 Data flow

1. A versioned form definition is published to devices.
2. Records are entered offline into the local SQLite store; each change is appended to the change log.
3. When connected, the change log syncs to `epiora-sync`; the server orders changes and returns
   updates from other devices. Concurrent edits to the same field on different devices create a
   conflict item in a reviewer queue; nothing is silently merged.
4. Analyses always run in `epiora-core` on local data, identically offline and online, and emit a
   receipt (§8.8).
5. Connectors on the server pull from and push to external systems (§10) and land data in the
   same project model.

### 7.3 Sync protocol (outline; detailed in its own spec, v2.0)

- Unit of change: (record id, field, new value, author, device, logical timestamp, base version).
- The server is the ordering authority; devices hold a high-water mark per project.
- Conflict: two changes to the same field whose base versions are the same. Both are kept and a
  conflict item is raised; the record shows a conflict badge until resolved.
- Deletions are tombstones; form-version upgrades migrate records with an audited mapping.
- Invariants tested by simulation: no lost write, every conflict surfaced, all devices converge.

### 7.4 Determinism

As in NeoPhylo: `libm` for floating-point functions, seeded ChaCha RNG with a fixed draw order for
anything random (bootstrap, simulation), no clock or environment reads inside the engine,
bit-identical native and wasm results.

## 8. Feature modules

Each module lists Epi Info parity, additions and libraries. "Reference" means the library is used
only to generate expected values; its code is not copied.

### 8.1 Form Designer
- Parity: Epi Info field types, related child forms, grids, check code.
- Check code is translated into a typed, sandboxed expression language (`epiora-core`).
- Additions: versioned forms, multi-language forms, template library (outbreak investigation,
  contact tracing, standard surveys), XLSForm import and export.

### 8.2 Data Entry
- Parity: entry with skip logic and validation, offline.
- Additions: double-entry verification, duplicate detection, GPS, photo, barcode/QR and signature
  fields on mobile.

### 8.3 Analysis
- Parity list (Classic Analysis): frequencies; tables including 2×2 with odds and risk ratios and
  confidence intervals, Mantel–Haenszel stratified analysis, chi-square and Fisher's exact test;
  means with t-test, ANOVA and Kruskal–Wallis; linear regression; logistic regression including
  conditional; Kaplan–Meier and Cox; complex-sample frequencies, tables and means; data
  management commands (select, sort, recode, define, assign, merge, relate); `.pgm` scripts via
  the compatibility interpreter.
- Data wrangling engine: SQLite SQL by default; DuckDB-wasm alongside it only if the v0.1 spike
  shows SQLite is too slow for analytic queries (open decision D3).

### 8.4 Dashboards and charts
- Parity: Visual Dashboard. Additions: epi curves, population pyramids, linked filters.
- Library: Apache ECharts (Apache-2.0), SVG renderer for exports.

### 8.5 Maps
- Parity: region and GPS maps. Additions: offline basemaps.
- Libraries: MapLibre GL JS and PMTiles (BSD-3); OpenStreetMap data with ODbL attribution;
  geoBoundaries boundaries (CC BY 4.0 / ODbL; commercial terms to confirm, D5).

### 8.6 StatCalc
- Parity: sample size and power (cohort, case-control, cross-sectional, population survey), 2×2
  calculator, chi-square for trend, Poisson. Reference: OpenEpi and R.

### 8.7 Growth (AnthStat)
- Parity: z-scores and percentiles against WHO 2006, WHO 2007 and CDC 2000 references.
- CDC tables are US government work (public domain). WHO tables' licence for bundling is open
  decision D4. Reference: WHO `anthro` R package (GPL-3).

### 8.8 Reports, receipts and reproducibility
- Every result carries a receipt: dataset id, version and content hash; method and options;
  engine version; reference software and tolerance it was validated against; timestamp.
- One-click export of an equivalent R, Stata or Python script.
- PDF and HTML reports with Typst (Apache-2.0), in the browser or on the server.

### 8.9 Outbreak and surveillance
- Outbreak (v2.3): case definitions with automatic classification, line lists, epi curves, attack
  rates, food-specific 2×2 with stratification, transmission chains, templates, auto-updating
  situation reports.
- Surveillance (v3.0): EARS C1–C3, CUSUM and Farrington detection (reference: R `surveillance`);
  R(t) by the Cori method (reference: EpiEstim); scheduled alerts. Nowcasting via WebR (v3.3).

### 8.10 Collaboration
- Roles per project, form and field; comments; append-only audit log (open); shared templates;
  enterprise administration (closed).

### 8.11 AI copilot (closed, v3.1)
- Drafts forms, proposes analyses, explains results, drafts reports.
- Never states a number the engine did not compute; every number links to its receipt.
- Providers: Anthropic, OpenAI, Azure OpenAI, local models via Ollama. Default sends form
  structure only, never records; admins control what is allowed.

## 9. Statistics strategy

| Layer | What | Why |
|---|---|---|
| Rust core (hand-written, validated) | Everyday procedures in §8.3, StatCalc, growth z-scores, outbreak and alert algorithms that must run on phones | Instant, offline, deterministic, drives interactive dashboards |
| Existing Rust libraries | Distributions (`statrs`), linear algebra (`faer`/`nalgebra`), file formats | No reinvention |
| R via WebR (on demand), server R in hosted tiers | Complex Cox options, mixed models, GAMs, survey-weighted regression, Farrington, nowcasting, Bayesian methods, user R code | R's ecosystem is far ahead; rewriting is waste |
| R, Stata, SAS, OpenEpi, Epi Info 7 | References for the corpus | Validation |

WebR's download size and start-up time on low-end Android are measured in the v0.1 spike; if
unacceptable, advanced methods are hosted-only on mobile.

## 10. Integrations and connectors

Credentialed connectors run on `epiora-sync`; file imports run in the browser. Every connector
has a field-mapping screen, dry-run preview, incremental pull and a change log.

| Order | Integration | Notes | Version |
|---|---|---|---|
| 1 | CSV, Excel, SPSS `.sav`, Stata `.dta`, SAS `.sas7bdat`, Parquet, XLSForm | `calamine`, `rust_xlsxwriter`, ReadStat, `arrow-rs` | v0.3, v1.1, v1.5 |
| 1 | Epi Info `.prj`/`.mdb`, SQL Server-backed projects | `epiora-mdb`; direct DB connection for SQL Server | v1.2 |
| 2 | ODK Central, KoboToolbox | ODK Central is Apache-2.0 with REST, OpenRosa and OData APIs; Kobo shares the form standard | v2.2 |
| 3 | REDCap | Per-project API tokens; export and import of records and metadata | v2.2 |
| 4 | Go.Data | WHO open-sourced it in April 2024; exchange cases and contacts; OpenFn adaptor exists | v3.2 |
| 5 | DHIS2 | Aggregate push, tracker pull | v3.2 |
| 6 | SORMAS | REST API for pseudonymized or anonymized data | v4.1 |
| 7 | FHIR R4, then HL7 v2 lab reporting | Test against a HAPI FHIR server | v4.1 |

Cross-cutting: OData feed and read-only Postgres views for Power BI, Excel and Tableau; Parquet
exports to S3 or Azure (v3.2). Sign-in through OpenID Connect and SAML via Keycloak for
self-hosters; Google and Microsoft Entra for the hosted service; institutional SSO
(eduGAIN/InCommon) and LTI 1.3 in paid tiers (v2.5, v4.0). Notifications by email (SES or
Postmark), SMS and WhatsApp via Twilio or Africa's Talking (v3.0). OpenFn supported as a bridge
for long-tail systems. Billing through Paddle as merchant of record (v2.0).

## 11. Security, privacy and compliance

### 11.1 Baseline (every release)
- TLS everywhere; server database and backups encrypted at rest.
- Identifying fields encrypted on device with WebCrypto (AES-GCM, keys derived from a
  passphrase); whole-database encryption in Tauri apps with SQLite3MultipleCiphers. Whole-database
  encryption in the browser via an open SQLCipher wasm build is a v0.1 spike (D6).
- Roles per project, form and field; append-only audit log of views, changes and exports.
- De-identified export: drop or hash identifiers, date shifting, small-cell suppression
  (default under 5, configurable) on every shared table and dashboard.
- Memory-safe Rust, fuzzed parsers, dependency audit (`cargo-deny`, npm audit), CycloneDX SBOM,
  signed releases, vulnerability disclosure policy.
- Accessibility to WCAG 2.1 AA. US ADA Title II applies to state and local government web content
  and apps; the DOJ's 20 April 2026 interim rule set deadlines of 26 April 2027 (large entities)
  and 26 April 2028 (small). Section 508 for federal buyers; EN 301 549 for EU buyers.

### 11.2 By segment
| Segment | Requirements | Version |
|---|---|---|
| Universities, researchers | DPA template, EU hosting (GDPR), minimal student data (FERPA); SOC 2 Type II once revenue covers compliance automation | v2.0, v3.4 |
| NGOs, health departments | HIPAA BAA and HIPAA-eligible hosting; AI only with BAA-signing providers or local models; country hosting regions; ISO 27001; annual pen test | v3.4 |
| US state and local | GovRAMP (StateRAMP renamed in 2025) | v4.4 |

Until v3.4, US health departments handling protected health information use self-hosting
(§11.3) or work with de-identified data in the hosted Team tier.
| US federal | FedRAMP (including the FedRAMP 20x track), likely with a partner | v4.4 |

### 11.3 The self-hosting route
An agency or ministry running the open-source server in its own authorized cloud or data centre
is covered by its own security authorization, not Epiora's. This is how agencies, ministries and
WHO-linked teams adopt Epiora before it holds certifications, and it matches data-sovereignty
and Digital Public Goods requirements.

## 12. Validation and testing

1. **Reference corpus.** Fixtures generated by R (`epitools`, `exact2x2`, `survival`, `survey`,
   `surveillance`, `EpiEstim`, `anthro`), cross-checked with Stata and SAS where available;
   StatCalc against OpenEpi. Fixtures are never hand-edited and never change in the same commit
   as engine code.
2. **Epi Info as reference.** `.pgm` scripts run through Epi Info 7 on a Windows CI runner;
   the compatibility interpreter must match. Test data from Epi Info sample projects and CDC
   training datasets (public domain), including the Oswego outbreak data. Building or obtaining
   Epi Info 7 now that CDC removed installers is a v0.1 spike.
3. **Tolerances.** 1e-9 for closed-form results, 1e-6 for iterative fits; documented per
   procedure; any loosening is an owner gate.
4. **Property tests and fuzzing.** `proptest` invariants; `cargo-fuzz` over every importer and
   parser (`.mdb`, `.prj`, check code, `.pgm`, XLSForm, stats files); the engine never panics.
5. **Native and wasm parity.** Bit-identical results on the corpus subset.
6. **Sync simulation.** Deterministic simulation with dropped connections, clock skew and
   concurrent offline edits; invariants from §7.3.
7. **App tests.** Vitest; Playwright on Chromium, Firefox and WebKit; axe accessibility checks;
   visual snapshots; Android emulator tests for the Tauri app.
8. **Performance budgets.** Set in each version's spec; initial targets: 2×2 table on one million
   rows under 1 s in the browser; first load under 5 MB gzipped excluding WebR and basemaps;
   usable on a low-end Android phone (reference device chosen in v0.1).
9. **Human review.** The epidemiologist adviser signs off each statistics module before release.
10. **Validation report per release.** Generated by CI: procedures, references, tolerances, results.

Process: NeoPhylo's loop (spec → plan → one task per branch, CI gates, owner gates on
tolerances, fixtures and baselines).

## 13. Business model and pricing

Competitor anchors (October 2026): KoboToolbox is free for nonprofits, government, UN and
education (Community plan, 5,000 submissions/month) and $99/month for others; ODK Cloud from
$199/month; SurveyCTO $225–630/month. Epiora does not compete on collection price.

| Tier | For | Includes | Starting price hypothesis |
|---|---|---|---|
| Open (AGPL) | Students, individuals, self-hosters | Full offline app, legacy import, local exports, basic self-hosted sync | Free |
| Solo cloud | Researchers, consultants | Hosted sync and backup, small sharing, reports | $15–29/month |
| Team | Health departments, NGOs, research groups | Roles, audit, connectors, outbreak module, AI quota, priority support | $149–299/month |
| Education | Departments, training programmes | Course workspaces, LTI, assignments, instructor dashboards | $1–5k/year per department |
| Enterprise / government | Ministries, agencies, large NGOs | SSO, data residency, SLA, compliance pack, DHIS2/FHIR/Go.Data, managed self-hosting | Custom, ~$10k–100k+/year |

Standing discount for nonprofits and low- and middle-income countries. Other revenue: legacy
migration service with a validation report; training and certification; support contracts and
custom connectors; grants from global health digital-goods funders. Prices are validated with
pilots before each tier launches.

## 14. Roadmap

Sizes assume part-time, agent-driven work: **S** ≈ 2–3 weeks, **M** ≈ 4–6 weeks, **L** ≈ 2–3
months. Every version ships with tests to §12's standard, a validation report and at least one
worked example or template. Re-plan after every pilot.

| Milestone | After | Meaning |
|---|---|---|
| 🔬 Foundations proven | v0.3 | Every risky technical bet has been tested; the engine and app shell exist |
| 🏠 Epi Info parity | v1.5 | Open core does everything Epi Info did, offline, with receipts; public launch |
| 🧭 Field-ready | v2.5 | Sync, teams, connectors, outbreak in a box, mobile; first paid tiers |
| 🦅 Powerhouse | v3.4 | Surveillance, AI copilot, ministry connectors, advanced statistics, SOC 2 and HIPAA |
| 🏛️ Agency production | v4.4 | Enterprise, health standards, multi-level deployments, localization, government certifications |

### Phase 0 — Foundations (v0.1–v0.3)

**v0.1 Spikes (M).** Throwaway probes, each ending in a recorded decision:
WebR size and start-up on a low-end Android reference device; sqlite-wasm on the browser private
file system, with and without an open SQLCipher wasm build (D6); SQLite vs DuckDB-wasm for
analytic queries on one million rows (D3); mdbtools compiled to wasm reading Epi Info sample
`.mdb` files; building and running Epi Info 7 on a Windows CI runner; Tauri 2 shell on Android
and iOS; a two-device sync prototype with a conflict; R-to-fixture corpus pipeline.

**v0.2 Engine core (L).** `epiora-core` with the project and dataset model, frequencies and 2×2
tables (odds and risk ratios, confidence intervals, chi-square, Fisher's exact,
Mantel–Haenszel), receipts v1, corpus and tolerances, `proptest`, `cargo-fuzz`, `epiora-wasm`,
native/wasm parity in CI.

**v0.3 App shell and local store (M).** PWA and Tauri desktop shell, local SQLite store and change
log, CSV and Excel import, the Analysis workspace running v0.2 procedures, design tokens and
accessibility checks in CI. **🔬 Foundations proven.**

### Phase 1 — The home for Epi Info users (v1.0–v1.5)

**v1.0 Analysis parity I (L).** Means (t-test, ANOVA, Kruskal–Wallis), linear and logistic
regression, data-management commands, StatCalc, explain mode, reproduce-in-R/Stata export,
offline PWA. First university pilot. *Hero USP 2 (receipts) in usable form.*

**v1.1 Forms and entry (L).** Form Designer, the check-code language, entry with validation and
skip logic, child forms and grids, form versioning, XLSForm import and export, double-entry
verification, template library v1.

**v1.2 Legacy import (L).** `epiora-mdb`, `.prj` and SQL Server project import, check-code
translation, the `.pgm` compatibility interpreter, migration report with parity results against
Epi Info 7. Public "home for Epi Info users" launch; apply for Digital Public Good listing.

**v1.3 Survival, complex samples and growth (M).** Kaplan–Meier, Cox, conditional logistic
regression, complex-sample frequencies, tables and means; growth z-scores (WHO 2006, WHO 2007,
CDC 2000) once D4 is resolved.

**v1.4 Dashboards, epi curves and maps (L).** ECharts dashboards with linked filters, epi curves,
population pyramids; MapLibre maps with offline PMTiles basemaps and geoBoundaries.

**v1.5 Reports and polish (M).** Typst reports and templates; SPSS, Stata, SAS and Parquet import
and export; design-system pass; independent WCAG 2.1 AA audit. **🏠 Epi Info parity.**

### Phase 2 — Field and teams (v2.0–v2.5)

**v2.0 Sync and solo cloud (L).** `epiora-sync` with the conflict review queue (own spec);
sync simulation suite; Tauri Android and iOS apps; device encryption; hosted solo cloud with
backups; Paddle billing; DPA template and EU hosting. *First paid tier.*

**v2.1 Teams and privacy (M).** Roles with field-level permissions, audit log, comments, shared
templates; de-identified export, small-cell suppression, tiered sharing. Team tier opens.

**v2.2 Connectors I (M).** ODK Central, KoboToolbox and REDCap with mapping screens and dry runs.
*Hero USP 1 (keep your collection tool) in usable form.*

**v2.3 Outbreak in a box (L).** Case definitions and automatic classification, line lists,
attack-rate tables, food-specific 2×2 with stratification, transmission chains, investigation
templates (foodborne, measles, cholera, respiratory), auto-updating situation reports.
*Hero USP 3.*

**v2.4 Mobile field kit (M).** GPS, photo, barcode/QR and signature fields; offline basemap
download by area; performance pass on the reference Android device. *Hero USP 4 complete.*

**v2.5 Education tier (M).** Course workspaces, LTI 1.3, assignments with per-student datasets,
instructor dashboards, a course pack of lab handouts. **🧭 Field-ready.**

### Phase 3 — Surveillance and intelligence (v3.0–v3.4)

**v3.0 Surveillance (L).** EARS C1–C3 and CUSUM in Rust, Farrington via WebR or server R, R(t)
by the Cori method, scheduled alerts by email, SMS and WhatsApp, scheduled sitreps.

**v3.1 AI copilot (M).** Provider-agnostic gateway including local models, receipt-linked
answers, admin controls, form and report drafting.

**v3.2 Connectors II (L).** Go.Data, DHIS2 (aggregate push, tracker pull), OData feed, Postgres BI
views, Parquet exports to object storage.

**v3.3 Advanced statistics (M).** Mixed models, GAMs, survey-weighted regression and nowcasting
via WebR and server R, with receipts and references.

**v3.4 Compliance I (M).** SOC 2 Type II, HIPAA BAA and HIPAA-eligible hosting, first independent
penetration test, ISO 27001 preparation, Section 508 VPAT. **🦅 Powerhouse.**

### Phase 4 — Enterprise and government (v4.0–v4.4)

**v4.0 Enterprise (L).** SAML/OIDC SSO and SCIM, admin console, data-residency regions, SLA,
managed self-hosting (Docker and Helm), air-gapped installation.

**v4.1 Health standards (L).** FHIR R4 export and import, HL7 v2 electronic lab reporting, SORMAS.

**v4.2 Multi-level deployments (L).** District → region → national roll-ups, tiered sharing at
scale, multi-tenant ministry deployments.

**v4.3 Localization (M).** French, Spanish, Portuguese, Arabic (right-to-left), Russian and
Chinese for app, templates and reports.

**v4.4 Government readiness (L).** ISO 27001 certification, GovRAMP, a FedRAMP path with a
partner, and a public validation dossier. **🏛️ Agency production.**

Indicative horizon at part-time pace, summed from the sizes above: Phase 0 about 4–6 months,
Phase 1 about 10–15 months, Phase 2 about 8–12 months, Phase 3 about 7–11 months, Phase 4 about
9–14 months; roughly three to five years to agency production. The first paid tier (v2.0) starts
around month 14–21; university pilots start earlier, at v1.0.

## 15. Running tracks

| When | Track |
|---|---|
| Before v1.0 | Recruit an epidemiologist adviser; competitor scan (D9) |
| From v1.0 | University pilots; outreach to former Epi Info users, field epidemiology training programmes and MPH courses |
| After v1.2 | Digital Public Good application; migration-service offer; JOSS paper on the engine and validation |
| From v2.0 | NGO and health-department pilots; price validation before each tier |
| After v2.3 | Case study from a real outbreak investigation; conference demos (epidemiology and public-health informatics meetings) |
| Phase 3–4 | Grants from global health digital-goods funders; partner search for FedRAMP |

## 16. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Scope far exceeds one part-time developer | Strict version gates; each version shippable alone; agent-driven loop; re-plan after pilots |
| A wrong number in an outbreak analysis | Reference corpus, never-loosened tolerances, adviser sign-off, receipts |
| A free fork of Epi Info 7 gains traction | Compete on what a fork lacks: offline mobile, receipts, outbreak in a box, connectors |
| A competitor copies the analysis layer | AGPL on the open core; the validation corpus and connectors take years to rebuild |
| Sync bugs corrupt data | Own spec, simulation suite, conflict queue instead of silent merges |
| WebR too heavy for phones | Measured in v0.1; fall back to hosted R for advanced methods on mobile |
| Compliance cost before revenue | Self-hosting route first; certifications only when revenue covers them |
| Licensing surprises in data or dependencies | Licence checks recorded as decisions (A6, A7, D2, D4, D5) before use |

## 17. Open decisions

| # | Decision | Resolve by |
|---|---|---|
| D1 | Formal trademark clearance for "Epiora" (USPTO, EUIPO/TMview), domain and package names | Before v1.2 public launch |
| D2 | Exact licence of the Epi Info 7 source (Apache 2.0 or public domain) before reusing its grammar or code | v0.1 |
| D3 | DuckDB-wasm alongside SQLite, or SQLite alone | v0.1 spike |
| D4 | Licence for bundling WHO growth reference tables | Before v1.3 |
| D5 | geoBoundaries commercial-use terms | Before v1.4 |
| D6 | Whole-database encryption in the browser (open SQLCipher wasm build or field-level only) | v0.1 spike |
| D7 | Low-end Android reference device | v0.1 |
| D8 | Whether Electric replaces the custom sync protocol | v2.0 spec |
| D9 | Competitor scan confirming USP claims | Before v1.0 |

## 18. Sources

- CDC, Epi Info sunset updates: https://www.cdc.gov/epiinfo/sunsetnews.html
- CDC, Epi Info sunset alternatives notice: https://cdc.gov/epiinfo/pdfs/userguide/Epi_Info_Sunset_Alt_Draft_508.pdf
- CDC, Form Designer, creating a new project: https://cdc.gov/epiinfo/user-guide/form-designer/how-to-create-new-blank-project.html
- Wikipedia, Epi Info: https://en.wikipedia.org/wiki/Epi_Info
- Epi Info 7 source: https://github.com/Epi-Info/Epi-Info-Community-Edition
- mdbtools: https://github.com/mdbtools/mdbtools
- Digital Public Goods Standard: https://digitalpublicgoods.net/standard
- PowerSync Open Edition (FSL): https://www.powersync.com/blog/powersync-open-edition-release
- Electric (Apache-2.0): https://hex.pm/packages/electric/1.4.3
- GADM licence: https://gadm.org/license.html
- geoBoundaries: https://github.com/wmgeolab/geoBoundaries
- PMTiles: https://awesome.ecosyste.ms/projects/github.com%2Fprotomaps%2FPMTiles
- WHO anthro R package: https://cran.r-project.org/web/packages/anthro/refman/anthro.html
- SQLite wasm and SEE: https://www.sqlite.org/wasm/doc/trunk/see.md
- ODK Central: https://github.com/getodk/central
- WHO, Go.Data open source: https://www.who.int/news/item/24-04-2024-go.data-transition-to-an-open-source-tool
- SORMAS (DPG registry): https://digitalpublicgoods.net/registry/surveillance-outbreak-response-management-and-analysis-system.html
- OpenFn Go.Data adaptor: https://docs.openfn.org/adaptors/godata
- GovRAMP rebrand: https://govramp.org/blog/stateramp-announces-rebrand-to-govramp-reflecting-mission-to-unite-public-and-private-sectors-in-advancing-cybersecurity/
- DOJ Title II deadline extension: https://www.illinoislawyernow.com/2026/04/doj-extends-title-ii-ada-web-content-accessibility-compliance-deadlines/
- KoboToolbox pricing: https://www.kobotoolbox.org/pricing/
- ODK Cloud pricing: https://getodk.org/pricing/
- SurveyCTO pricing: https://www.surveycto.com/pricing/
