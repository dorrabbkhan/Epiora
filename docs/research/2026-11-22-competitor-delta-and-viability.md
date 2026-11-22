# Competitor delta and viability review (after v1.0.0)

**Date:** 2026-11-22 · **Follows:** [D9 competitor scan](d9-competitor-scan.md) (2026-11-12) · **Feeds:** the
2026-11-22 platform roadmap review (platform spec header, §5, §8.12, §13–§16) and the v1.1 spec.
**Method:** web searches and primary sources; commit and release dates from the GitHub API on 2026-11-22.
Anything not confirmed from a primary source is marked **unverified**.

## Summary

1. **Nothing material changed in ten days, and no funded newcomer was found.** Epi Info AI has had no
   GitHub activity since 2026-09-23; its GitHub Pages demo now returns 404; still no `.prj` support, no
   connectors, no localisation; CDC backing still **unverified**.
2. **New or newly found:** PublicHealth Tech Lab's EpiStat (calculators) and EpiLog (offline outbreak diary),
   both MIT, one developer, claiming a DPGA "Level 1 Pass" for EpiCalc (**unverified**); Statulator redesigned
   (CSV upload, plain-language interpretation; named by CDC's sunset sheet); volunteer Epi Info 7 forks
   (`jarz/Epi-Info-Workspace`, created 2026-11-22; `MVMLima/epiinfo7-community-edition-fork`, CVE patches);
   Sentinel (GPL-3.0 surveillance platform, no statistics); Esri marketing ArcGIS to former Epi Info users.
3. **General-purpose AI is the real alternative for casual users.** LLMs choose the right test almost always,
   but run it correctly 46–92% of the time depending on prompting (JMIR 2025 e63550), with silent errors such
   as un-exponentiated odds ratios (IJPH 2025). No accuracy study of Mantel–Haenszel specifically was found.
4. **Still unclaimed:** opening Epi Info `.prj`/`.mdb` projects; a published validation dossier with
   CI-proven reproduction; calculators + analysis + connectors + outbreak tools in one free offline tool.

## Feature comparison

Y = yes · P = partial · – = no · ? = unknown. Rows other than Epiora, Epi Info AI, MetricGate, PublicHealth
Tech Lab and Stata carry over from the D9 scan.

| | Free / open | Offline | OpenEpi calculators | Means / regression | Opens Epi Info projects | Forms / entry | Kobo / ODK / REDCap | Outbreak tools | Maps | Reproducible | Validation published | Explains results | AI |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| **Epiora v1.0.0** | Y AGPL | Y | Y (all) | Y / – | – (v1.1) | – (v1.3) | – (v1.2) | – (v2.3) | – (v1.4) | Y (CI-proven R) | Y | Y | – (MCP v1.2) |
| Epi Info AI | Y Apache | Y | P | Y / Y | P (`.mdb` only) | Y | – | P | Y | P | P | ? | Y |
| Epi Info 7 (frozen) | Y | Y | P | Y / Y | Y | Y | – | P | Y | – | – | – | – |
| OpenEpi (frozen 2014) | Y MIT | P | Y | Y / – | – | – | – | – | – | – | – | P | – |
| PublicHealth Tech Lab | Y MIT | Y | Y | ? / P | – | P | – | P | P | ? | ? | ? | – |
| Statulator | Y (closed) | – | P | P / ? | – | – | – | – | – | – | – | Y | – |
| MetricGate | 5 runs/month | – | Y | ? / Y | – | – | – | P | ? | Y | – | P | Y |
| jamovi / JASP | Y AGPL | Y | P | Y / Y | – | – | – | – | – | Y | – | – | – |
| Stata 19 | ~$1k/year | Y | Y | Y / Y | – | – | – | – | P | Y | – | – | ? |
| Kobo / ODK / SurveyCTO | P | Y | – | – | – | Y | (collection) | – | P | – | – | – | Y (paid) |
| Go.Data / SORMAS | Y GPL | P | – | – | – | Y | P | Y | Y | – | – | – | – |
| ChatGPT / Claude / Julius | P | – | Y | Y / Y | – | – | – | P | P | P (code) | – | Y | Y |

## Status changes since 2026-11-12

| Tool | Change |
|---|---|
| Epi Info AI | No commits, tags or releases since v0.2.0 (2026-09-23); Pages demo 404; `drlove` lists no affiliation |
| MetricGate | Free tier cut to 5 runs/month, 2 MB, 1,000 rows × 20 columns; paid $15 / $39 / $99 per month |
| SurveyCTO | 2.85 (2026-10-12): automatic and bulk AI transcription and translation, AI API, new credits |
| CommCare | Prices verified: Free (5 users), $100, $500, $1,000, Enterprise from $4,000 per month; AI on paid plans |
| ODK Central | v2026.3.2 (2026-11-19), security patch |
| DHIS2 Chap | v2.7–v2.8 (to 2026-11-17): alert-to-action, facility-scale data |
| Stata 19 | Checked: `epitab` (2×2, stratified, matched, MH), power for 2×2 and matched designs; no attack-rate or epi-curve tools |
| jamovi, JASP, SORMAS, Go.Data, Wepisia, EIAnalysis, OpenEpi, EpiData | No material change |

## Viability

| Factor | Rating | Why |
|---|---|---|
| Build speed | Strong | v0.1 to v1.0 in about four weeks with the agent loop |
| Timing | Good, window closing | The sunset was 14 months ago; stranded users are choosing replacements now |
| Differentiation | Real but narrow | Validated numbers and Epi Info import are defensible; calculators and explain mode are easy to copy |
| AI | Growing threat, also an opening | AI makes running a test cheap, not trustworthy; Epiora can be the engine an assistant calls |
| Market size | Small | Epi Info users are mostly LMIC ministries, FETPs and small health departments |
| Willingness to pay | Weak in the first segment | Revenue leads with grants, services and a hosted API |
| Trust and distribution | Biggest gap | No users, adviser or pilot yet; owner defers these to v1.6 so the product launches mature |
| Scope | Phases 3–4 unrealistic unfunded | Made funded tracks |
| Bus factor | Risk | One part-time developer; answered by AGPL code, the dossier and a support agreement |

**Overall.** Viable as a grant-fundable open public good with side income from services and the API; a
venture-scale SaaS is unlikely. The deciding risk is adoption, not engineering.

**Owner direction (2026-11-22).** Goal: wide use and/or side income and/or grant funding. Public launch,
adviser, pilot and DPG application wait for v1.6 (features and UI mature first). Accepted: Phases 3–4 as
funded tracks; revenue order led by grants, services and a hosted API; "Epiora for AI" with the local MCP
server and CLI at v1.2 (when the pipeline open → clean → analyse exists), a hosted calculator MCP at the v1.6
launch and a paid metered API at v2.0; monthly competitor watch.

## Sources

Accessed 2026-11-22.

- Epi Info AI: https://github.com/drlove/epi-info-ai ; https://epi-info-ai.github.io/epi-info-ai/ (404)
- Epi Info 7 forks: https://github.com/jarz/Epi-Info-Workspace ; https://github.com/MVMLima/epiinfo7-community-edition-fork
- PublicHealth Tech Lab: https://phtlab.org ; https://github.com/whlee5503-dot/EpiStat ; https://github.com/whlee5503-dot/EpiLog ; https://epilog.phtlab.org
- Statulator: https://statulator.com ; CDC alternatives sheet https://www.cdc.gov/epiinfo/pdfs/userguide/20250722_Cleared-Epi-Info-Sunset-Alt.pdf
- Sentinel: https://github.com/christianpeut95/Sentinel
- Esri: https://www.esri.com/en-us/industries/blog/articles/beyond-epi-info (not fetched; **unverified**)
- MetricGate pricing: https://metricgate.com/pricing
- SurveyCTO release notes: https://docs.surveycto.com/release-notes/
- CommCare pricing: https://commcare.dimagi.com/pricing/
- ODK Central releases: https://github.com/getodk/central
- DHIS2 Chap: https://github.com/dhis2/chap-core
- Stata epidemiology features: https://www.stata.com/features/epidemiology/
- LLM statistics accuracy: https://www.jmir.org/2025/1/e63550 ; https://www.ncbi.nlm.nih.gov/pmc/articles/PMC12627256/ ;
  https://www.ssph-journal.org/journals/international-journal-of-public-health/articles/10.3389/ijph.2025.1608572/full ;
  RWE-bench https://arxiv.org/html/2603.22767v1
- EpidBot preprint: https://sciety.org/articles/activity/10.64898/2026.06.18.26355985
