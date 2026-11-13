#!/usr/bin/env node
// Capture OpenEpi displayed values for the cases in cases.json (v1.0 Task 3,
// spec §9). Runs only under .github/workflows/openepi.yml; never hand-edit the
// <id>.json output it writes.
//
//   node capture.mjs --dry-run        validate cases.json offline (no network)
//   node capture.mjs                  capture every case, write <id>.json
//   node capture.mjs --calc <id> [--case <id>]   subset
//
// Politeness (Task 1 terms verdict): one page at a time, >=2 s between cases.

import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';
import { CALCULATORS, INPUT_SPECS, validateInputs, setCells } from './drivers.mjs';

const DIR = path.dirname(url.fileURLToPath(import.meta.url));
const REPO = path.resolve(DIR, '..', '..');
const CASES_PATH = path.join(DIR, 'cases.json');
const LABELS_PATH = path.join(DIR, 'labels.json');
const CASE_DELAY_MS = 2000;
const NAV_TIMEOUT_MS = 60000;

const args = process.argv.slice(2);
const flag = (n) => args.includes(n);
const opt = (n) => args[args.indexOf(n) + 1];
const DRY_RUN = flag('--dry-run');
const ONLY_CALC = flag('--calc') ? opt('--calc') : null;
const ONLY_CASE = flag('--case') ? opt('--case') : null;
const PROXY = process.env.PLAYWRIGHT_PROXY ?? process.env.https_proxy ?? process.env.HTTPS_PROXY ?? null;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const deepEqual = (a, b) => JSON.stringify(a) === JSON.stringify(b);

// ---------- displayed-number parsing ----------
// One numeric token: optional < > ≤ ≥ bound marker, number, optional %
// or footnote mark (¹ ² ³ ° * † ‡). A cell holding "a, b" is a pair
// (comma + space); "1,234" without the space is thousands grouping.

export function parseDisplayed(text) {
  const t = String(text).replace(/[<>≤≥%,\s¹²³°*†‡]/g, '');
  const value = Number(t);
  if (!Number.isFinite(value)) return null;
  const mantissa = t.replace(/[eE][+-]?\d+$/, '');
  const frac = (mantissa.split('.')[1] ?? '').length;
  const exp = /[eE]([+-]?\d+)$/.exec(t);
  const decimals = exp ? Math.max(0, frac - Number(exp[1])) : frac;
  return { value, decimals };
}

// ---------- results scrape ----------
// Runs in the page: reads the rendered Results panel (#panel3) and returns
// {label, text} pairs. The row label is the first non-empty non-numeric cell;
// each following numeric cell (or ", "-separated pair in one cell) emits
// "<row label> [k]". Single-cell rows emit "label = value" pairs.

function scrapePanel() {
  const NUMSTR = '-?[\\d,]*\\.?\\d+(?:[eE][+-]?\\d+)?';
  const TOKEN = new RegExp(`^[<>≤≥]?\\s*${NUMSTR}\\s*[%¹²³°*†‡]?$`);
  const isNum = (s) => s !== '' && s.split(/\s*,\s*/).every((p) => TOKEN.test(p));
  const panel = document.getElementById('panel3');
  const rows = [];
  if (!panel) return rows;
  for (const table of panel.querySelectorAll('table')) {
    let lastLabel = '';
    for (const tr of table.rows) {
      // A td holding a multi-cell nested table mashes its descendant texts
      // into one string ("50|100" -> "50100"); skip such rows — the nested
      // table's own rows are iterated separately. A single-cell wrapper
      // table (OpenEpi wraps many result values this way) renders as just
      // its text, so those rows stay.
      const nested = (td) => { let n = 0; for (const t of td.querySelectorAll('table')) for (const r of t.rows) n += r.cells.length; return n; };
      if ([...tr.cells].some((td) => nested(td) > 1)) continue;
      const cells = [...tr.cells].map((c) => c.innerText.replace(/\s+/g, ' ').trim());
      if (cells.length === 0 || cells.every((s) => s === '')) continue;
      if (cells.length === 1) {
        const re = /([A-Za-z][^=<>]*?)\s*[=:]\s*([<>≤≥]?\s*-?[\d,]*\.?\d+(?:[eE][+-]?\d+)?\s*[%¹²³°*†‡]?)/g;
        let m;
        while ((m = re.exec(cells[0]))) rows.push({ label: m[1].trim(), text: m[2].trim() });
        continue;
      }
      const labelIdx = cells.findIndex((s) => s !== '' && !isNum(s));
      const rowLabel = labelIdx >= 0 ? cells[labelIdx] : lastLabel;
      if (labelIdx >= 0) lastLabel = rowLabel;
      let k = 0;
      for (let i = labelIdx + 1; i < cells.length; i++) {
        if (!isNum(cells[i])) continue;
        for (const part of cells[i].split(/\s*,\s*/)) {
          k += 1;
          rows.push({ label: `${rowLabel} [${k}]`, text: part });
        }
      }
    }
  }
  return rows;
}

// ---------- case.json validation (offline) ----------

function validateCases(cases) {
  const errs = [];
  const seen = new Set();
  for (const c of cases) {
    const id = `${c.calculator}/${c.case}`;
    if (!CALCULATORS[c.calculator]) { errs.push(`${id}: unknown calculator`); continue; }
    if (!INPUT_SPECS[c.calculator]) { errs.push(`${id}: no input spec`); continue; }
    if (seen.has(id)) errs.push(`${id}: duplicate case id`);
    seen.add(id);
    const fx = path.join(REPO, 'fixtures', 'calc', c.calculator, `${c.case}.json`);
    if (!fs.existsSync(fx)) { errs.push(`${id}: no Task 2 fixture ${path.relative(REPO, fx)}`); continue; }
    const fixture = JSON.parse(fs.readFileSync(fx, 'utf8'));
    if (fixture.id !== c.case) errs.push(`${id}: fixture id is '${fixture.id}'`);
    if (!deepEqual(fixture.inputs, c.inputs)) errs.push(`${id}: inputs differ from the fixture's`);
    for (const e of validateInputs(c.calculator, c.inputs)) errs.push(`${id}: ${e}`);
  }
  return errs;
}

// ---------- capture ----------

async function captureCase(browser, c, labels) {
  const drv = CALCULATORS[c.calculator];
  const pageUrl = typeof drv.url === 'function' ? drv.url(c.inputs) : drv.url;
  const promptAnswers = [...(drv.prompts?.(c.inputs) ?? [])];
  const dialogs = [];
  const page = await browser.newPage();
  page.on('dialog', async (d) => {
    dialogs.push(`${d.type()}: ${d.message()}`);
    if (d.type() === 'prompt') {
      const a = promptAnswers.shift();
      if (a === undefined) { await d.dismiss(); return; }
      await d.accept(String(a));
    } else {
      await d.accept().catch(() => d.dismiss());
    }
  });
  await page.goto(pageUrl, { waitUntil: 'load', timeout: NAV_TIMEOUT_MS });
  await page.waitForTimeout(1200);
  await page.click('#btnEnter');
  await page.waitForTimeout(1200);
  if (c.inputs.conf_level != null) {
    await page.evaluate((v) => { window.ConfLevel = v; }, String(c.inputs.conf_level));
  }
  const filled = drv.fill(c.inputs, page);
  if (Array.isArray(filled)) await setCells(page, filled); else await filled;
  await page.click('#calc');
  await page.waitForTimeout(1500);
  const scraped = await page.evaluate(scrapePanel);
  const missingLabels = [];
  const values = [];
  for (const { label, text } of scraped) {
    const parsed = parseDisplayed(text);
    if (!parsed) continue;
    const stat = labels?.[c.calculator]?.[label];
    if (stat === undefined) missingLabels.push(label);
    values.push({ label, statistic: stat ?? null, value: parsed.value, decimals: parsed.decimals });
  }
  await page.close();
  return { case: c.case, values, _meta: { url: pageUrl, dialogs, missingLabels } };
}

async function main() {
  const cases = JSON.parse(fs.readFileSync(CASES_PATH, 'utf8'));
  const errs = validateCases(cases);
  if (errs.length) {
    for (const e of errs) console.error(`cases.json: ${e}`);
    process.exit(2);
  }
  console.log(`cases.json: ${cases.length} cases valid against Task 1 inputs and Task 2 fixtures`);
  if (DRY_RUN) return;

  const labels = fs.existsSync(LABELS_PATH) ? JSON.parse(fs.readFileSync(LABELS_PATH, 'utf8')) : {};
  const run = cases.filter((c) => (!ONLY_CALC || c.calculator === ONLY_CALC) && (!ONLY_CASE || c.case === ONLY_CASE));
  const { chromium } = await import('playwright');
  const allMissing = [];
  // One fresh browser per calculator: bounds memory, and a crash loses at
  // most the calculator in flight (its output is only written when complete).
  const calcs = [...new Set(run.map((c) => c.calculator))];
  for (const calc of calcs) {
    const calcCases = run.filter((c) => c.calculator === calc);
    const browser = await chromium.launch(PROXY ? { proxy: { server: PROXY } } : {});
    const caseList = [];
    try {
      for (const c of calcCases) {
        process.stderr.write(`capture ${c.calculator}/${c.case} … `);
        let got;
        try {
          got = await captureCase(browser, c, labels);
        } catch (e) {
          got = { case: c.case, values: [], _meta: { url: '', dialogs: [`capture-error: ${String(e).split('\n')[0]}`], missingLabels: [] } };
        }
        const { _meta, ...out } = got;
        caseList.push(out);
        allMissing.push(..._meta.missingLabels.map((l) => `${c.calculator}/${c.case}: '${l}'`));
        const d = _meta.dialogs.length ? ` [dialogs: ${_meta.dialogs.join(' | ')}]` : '';
        console.error(`${got.values.length} values${d}`);
        await sleep(CASE_DELAY_MS);
      }
    } finally {
      await browser.close();
    }
    const outPath = path.join(DIR, `${calc}.json`);
    let capturedAt = new Date().toISOString();
    if (fs.existsSync(outPath)) {
      // ei7.yml precedent: keep the committed capture stamp on reruns so the
      // workflow's git-diff drift check fails only on real value drift.
      try { capturedAt = JSON.parse(fs.readFileSync(outPath, 'utf8')).captured_at ?? capturedAt; } catch {}
    }
    const doc = {
      calculator: calc,
      captured_at: capturedAt,
      openepi_url: CALCULATORS[calc].canonical ?? CALCULATORS[calc].url,
      cases: caseList,
    };
    fs.writeFileSync(outPath, JSON.stringify(doc, null, 2) + '\n');
    console.log(`wrote ${path.relative(REPO, outPath)} (${caseList.length} cases)`);
  }
  if (allMissing.length) {
    console.error('labels not in labels.json (statistic emitted as null):');
    for (const l of allMissing) console.error(`  ${l}`);
    process.exit(1);
  }
}

main().catch((e) => { console.error(e); process.exit(1); });
