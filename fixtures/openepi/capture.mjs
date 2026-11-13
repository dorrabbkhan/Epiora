#!/usr/bin/env node
// Capture OpenEpi displayed values for the cases in cases.json (v1.0 Task 3,
// spec §9). Runs only under .github/workflows/openepi.yml; never hand-edit the
// <id>.json output it writes.
//
//   node capture.mjs --dry-run        validate cases.json offline (no network)
//   node capture.mjs                  capture every case, write <id>.json
//   node capture.mjs --calc <id> [--case <id>]   subset
//   node capture.mjs --compare-r      compare committed output with Task 2 R fixtures
//
// Politeness (Task 1 terms verdict): one page at a time, >=2 s between cases.

import fs from 'node:fs';
import path from 'node:path';
import url from 'node:url';
import { CALCULATORS, INPUT_SPECS, validateInputs, setCells, syncEntry } from './drivers.mjs';

const DIR = path.dirname(url.fileURLToPath(import.meta.url));
const REPO = path.resolve(DIR, '..', '..');
const CASES_PATH = path.join(DIR, 'cases.json');
const LABELS_PATH = path.join(DIR, 'labels.json');
const OUTPUT_SKIP = new Set(['cases.json', 'labels.json', 'package.json', 'package-lock.json']);
const CASE_DELAY_MS = 2000;
const NAV_TIMEOUT_MS = 60000;

const args = process.argv.slice(2);
const flag = (n) => args.includes(n);
const opt = (n) => args[args.indexOf(n) + 1];
const DRY_RUN = flag('--dry-run');
const COMPARE_R = flag('--compare-r');
const ONLY_CALC = flag('--calc') ? opt('--calc') : null;
const ONLY_CASE = flag('--case') ? opt('--case') : null;
const PROXY = process.env.PLAYWRIGHT_PROXY ?? process.env.https_proxy ?? process.env.HTTPS_PROXY ?? null;
// Local development only: the workflow uses Playwright's bundled Chromium.
const CHROMIUM_PATH = process.env.CHROMIUM_PATH ?? null;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const deepEqual = (a, b) => JSON.stringify(a) === JSON.stringify(b);

// ---------- displayed-number parsing ----------
// decimals = the number of decimals in the displayed text (trailing zeroes
// count), read before numeric parsing. Bound displays ("<0.0000001") and
// not-available displays ("'?'", "'undefined'") keep their displayed text and
// are never equality-mapped: they emit statistic null with a string value.

export function parseDisplayed(text) {
  const raw = String(text).replace(/\([A-Za-z]{1,4}\)/g, '').replace(/[()]/g, '').trim();
  if (raw === '') return null;
  const bound = /^([<>≤≥])/.exec(raw)?.[1] ?? null;
  const t = raw.replace(/[<>≤≥%,\s¹²³°*†‡]/g, '');
  const value = Number(t);
  const mantissa = t.replace(/[eE][+-]?\d+$/, '');
  const frac = (mantissa.split('.')[1] ?? '').length;
  const exp = /[eE]([+-]?\d+)$/.exec(t);
  const decimals = exp ? Math.max(0, frac - Number(exp[1])) : frac;
  // Bounds are thresholds, not equalities: keep the displayed text.
  if (bound !== null || !Number.isFinite(value)) {
    return { value: null, decimals, percent: false, bound, text: raw };
  }
  return { value, decimals, percent: /%/.test(raw), bound, text: raw };
}

// ---------- results-row emission ----------
// cellParts: null when the cell is not a value cell; otherwise the list of
// numeric display tokens it holds (one per emitted value: "(a - b)" and
// "a , b" pairs split). Blank placeholders ("-") are neither values nor
// labels.

const NUMPART = /[<>≤≥]?\s*-?[\d,]*\.?\d+(?:[eE][+-]?\d+)?\s*%?\s*(\([A-Za-z]{1,4}\))?\s*[¹²³°*†‡]*|'(?:\?|undefined)'[¹²³°*†‡]*/g;
const SEPARATORS = /^[\s,\-–—]*$/;
const isBlank = (s) => /^[-–—\s]*$/.test(s);

export function cellParts(s) {
  // OpenEpi's number formatters (fmtSigFig, Round, fmtPValue) never emit
  // thousands separators, so every comma between digits in a results cell is
  // a pair separator ("4499,4999" = df numerator,denominator; "29,24" idem).
  const t = String(s).trim().replace(/^\((.*)\)$/, '$1').trim()
    .replace(/(\d),\s*(\d)/g, '$1 , $2');
  if (t === '' || isBlank(t)) return null;
  const parts = [];
  const rest = t.replace(NUMPART, (m) => { parts.push(m.trim()); return ' '; });
  if (!parts.length || !SEPARATORS.test(rest)) return null;
  return parts;
}

// Turns extracted panel rows [{cells:[text], th:bool}] into {label,text}
// emissions. One numeric cell -> one value, so a value is never the
// concatenation of several displayed cells. Rows are labelled by the text
// cells of their own row, prefixed by a leading row-number cell ("1 / …",
// "Crude / …" is already text). A label that repeats under more than one
// section gets its section (column header, else <th> title) prepended.
export function emissions(rows) {
  const items = [];
  let title = '';
  let header = '';
  let lastLabel = '';
  // A leading stratum/group marker cell ("1", "Crude", "Adjusted") also
  // describes the following rows of its block: OpenEpi marks only the first
  // row of each stratum block (e.g. "1 / Uncorrected chi square") while the
  // block's remaining rows (Yates, MH, Fisher, Mid-P) have no marker of
  // their own. The marker is carried as `stratum` context and used to
  // qualify repeated labels.
  let sectionLead = '';
  const MARKER = /^(\d+|crude|adjusted|stratum.*|values for stratum.*)$/i;
  const LV_RE = /([A-Za-z][^=<>]*?)\s*[=:]\s*([<>≤≥]?\s*-?[\d,]*\.?\d+(?:[eE][+-]?\d+)?\s*[%¹²³°*†‡]?|'\?'|'undefined')/g;
  for (const row of rows) {
    const cells = (row.cells ?? []).map((s) => String(s).replace(/\s+/g, ' ').trim());
    const present = cells.filter((s) => s !== '' && !isBlank(s));
    if (!present.length) continue;
    if (row.th) { title = present.join(' / '); header = ''; lastLabel = ''; sectionLead = ''; continue; }
    if (cells.length === 1) {
      if (cellParts(cells[0])) {
        for (const part of cellParts(cells[0])) {
          items.push({ label: `${lastLabel} [${items.filter((i) => i.label.startsWith(`${lastLabel} [`)).length + 1}]`, text: part, title, header, stratum: sectionLead });
        }
        continue;
      }
      LV_RE.lastIndex = 0;
      let m;
      let matched = false;
      while ((m = LV_RE.exec(cells[0]))) {
        matched = true;
        items.push({ label: m[1].trim(), text: m[2].trim(), title, header });
      }
      if (!matched) { header = cells[0]; lastLabel = ''; sectionLead = ''; }
      continue;
    }
    const parts = cells.map((s) => cellParts(s));
    const texts = cells.map((s, i) => (s !== '' && !isBlank(s) && !parts[i] ? s : null)).filter(Boolean);
    if (!parts.some(Boolean)) {
      header = texts.join(' / ');
      lastLabel = texts[0] ?? '';
      sectionLead = '';
      continue;
    }
    let label = texts.join(' / ');
    if (!label) label = lastLabel;
    else lastLabel = texts[0];
    const firstIdx = cells.findIndex((s) => s !== '' && !isBlank(s));
    if (firstIdx >= 0 && parts[firstIdx] && texts.length) {
      label = `${cells[firstIdx]} / ${label}`;
      if (MARKER.test(cells[firstIdx])) sectionLead = cells[firstIdx];
    } else if (firstIdx >= 0 && !parts[firstIdx] && MARKER.test(cells[firstIdx]) && cells[firstIdx].length <= 12) {
      sectionLead = cells[firstIdx];
    }
    let k = 0;
    for (let i = 0; i < cells.length; i++) {
      if (!parts[i] || i === firstIdx && texts.length) continue;
      for (const part of parts[i]) {
        k += 1;
        items.push({ label: `${label} [${k}]`, text: part, title, header, stratum: sectionLead });
      }
    }
  }
  // OpenEpi's DoseResponse page writes the whole results block twice; drop
  // identical (section, label, text) repeats — each table is scraped once.
  const seen = new Set();
  const deduped = items.filter((it) => {
    const key = `${it.title} ${it.header} ${it.label} ${it.text}`;
    if (seen.has(key)) return false;
    seen.add(key);
    return true;
  });
  // A label base emitted under more than one section is prefixed by the
  // context that distinguishes the sections (column header, else the <th>
  // title, else both). Prefixing applies to the whole base so a row's values
  // stay under one qualified label.
  const baseOf = (l) => l.replace(/ \[\d+\]$/, '');
  const byBase = new Map();
  for (const it of deduped) {
    const base = baseOf(it.label);
    if (!byBase.has(base)) byBase.set(base, []);
    byBase.get(base).push(it);
  }
  for (const its of byBase.values()) {
    const ctxs = new Set(its.map((it) => `${it.title}|${it.header}|${it.stratum ?? ''}`));
    if (ctxs.size <= 1) continue;
    for (const pick of [
      (it) => it.header,
      (it) => it.stratum,
      (it) => it.title,
      (it) => `${it.stratum} / ${it.header}`,
      (it) => `${it.title} / ${it.header}`,
      (it) => `${it.title} / ${it.stratum}`,
    ]) {
      const vals = new Set(its.map(pick));
      if (vals.size === ctxs.size && its.every((it) => pick(it))) {
        for (const it of its) it.label = `${pick(it)} / ${it.label}`;
        break;
      }
    }
  }
  return deduped.map(({ label, text }) => ({ label, text }));
}

// ---------- percent scaling ----------
// Some OpenEpi displays are percents while the Task 2 fixture keeps the
// fraction. Where the fixture scale is [0,1] the displayed value is divided
// by 100 and decimals raised by 2, leaving the 0.5 x 10^-decimals tolerance
// exactly unchanged. Power fixtures are percent-scale and are NOT converted.

// Displays on a 0-100 scale whose fixture leaf is a 0-1 fraction. OpenEpi
// renders these with a "%" sign (or bare, when the multiplier is 100) — the
// conversion applies whether or not the sign survived tokenization.
const ALWAYS_FRACTION = {
  proportion: (stat, i) => i.multiplier === 100 &&
    (stat === 'calc.proportion.estimate' || stat.startsWith('calc.proportion.ci.')),
  screening: (stat) => /^calc\.screening\.(sensitivity|specificity|ppv|npv|accuracy)\.(est|lower|upper)$/.test(stat) ||
    /^calc\.screening\.entropy\./.test(stat),
  // OpenEpi reports the directly adjusted risk difference in percent while
  // the fixture leaf is a fraction. Stratum/crude RD stays percent-scale in
  // the fixture — only the adjusted path converts.
  two_by_two: (stat) => /^calc\.two_by_two\.adjusted\.direct_rd\./.test(stat),
  // Power estimates themselves are percent-scale in the fixture; only the
  // derived prevalence/risk-difference echo converts to a fraction.
  power_cohort: (stat) => stat === 'calc.power_cohort.risk_difference',
};

const RATE_STATS = {
  rate_one: (stat) => stat === 'calc.rate_one.estimate' || stat.startsWith('calc.rate_one.ci.'),
  rate_compare: (stat) => /\.rate\.(exposed|unexposed|overall)\.(est|lower|upper)$/.test(stat) ||
    /\.rate_difference\.(est|lower|upper)$/.test(stat),
};
// Directly adjusted RD is stored per single person-time unit in the R
// fixtures while OpenEpi displays it per ptunit.
const UNIT1_STATS = {
  rate_compare: (stat) => /^calc\.rate_compare\.adjusted\.direct_rd\.(est|lower|upper)$/.test(stat),
};

export function scaleToR(calc, stat, inputs, parsed, ptunit) {
  const { value, decimals } = parsed;
  if (!Number.isFinite(value)) return { value, decimals };
  if (ALWAYS_FRACTION[calc]?.(stat, inputs)) {
    return { value: value / 100, decimals: decimals + 2 };
  }
  const pt = ptunit ?? 1;
  if (UNIT1_STATS[calc]?.(stat)) {
    return { value: value / pt, decimals: decimals + Math.round(Math.log10(pt)) };
  }
  if (RATE_STATS[calc]?.(stat) && inputs.per_unit && pt !== inputs.per_unit) {
    const f = inputs.per_unit / pt;
    return { value: value * f, decimals: decimals - Math.round(Math.log10(f)) };
  }
  return { value, decimals };
}

// Maps an emitted label to its fixture statistic. A label absent from
// labels.json reports {missing: true}; an emission OpenEpi always produces
// although the case did not request it (the driver marks it `unrequested`,
// e.g. proportion's one-sample z-test when the case has no compare_with —
// the page has no off state) is recorded with statistic null, so --compare-r
// does not seek an R counterpart for it.
export function statForLabel(drv, labels, calc, label, inputs) {
  const mapped = labels?.[calc]?.[label];
  if (mapped === undefined) return { stat: null, missing: true };
  if (mapped && drv.unrequested?.(mapped, inputs)) return { stat: null, missing: false };
  return { stat: mapped ?? null, missing: false };
}

// ---------- input guard ----------
// Before results are trusted, the form model (myTable, verified at fill time
// in setCells) and the array OpenEpi actually computed on (dataMatrix, stored
// by storeAllToMem when Calculate runs) must hold exactly the case's inputs.

export function multisetEqual(want, got) {
  if (want.length !== got.length) return false;
  const count = new Map();
  for (const v of want) count.set(v, (count.get(v) ?? 0) + 1);
  for (const v of got) {
    const n = count.get(v);
    if (!n) return false;
    if (n === 1) count.delete(v); else count.set(v, n - 1);
  }
  return count.size === 0;
}

// Elements of `want` missing from `got`, multiplicity-aware. OpenEpi's
// sample-size pages write derived values back into other data cells, so the
// held array is a superset of the written inputs — a subset check.
export function multisetDiff(want, got) {
  const count = new Map();
  for (const v of got) count.set(v, (count.get(v) ?? 0) + 1);
  const missing = [];
  for (const v of want) {
    const n = count.get(v);
    if (!n) { missing.push(v); continue; }
    if (n === 1) count.delete(v); else count.set(v, n - 1);
  }
  return missing;
}

function readDataMatrix() {
  const dm = window.dataMatrix;
  const out = { strata: [], evals: [] };
  const n = dm && dm[0] ? (dm[0].strata ?? 1) : 0;
  for (let s = 1; s <= n; s++) {
    const t = dm[s] ?? {};
    const vals = [];
    for (const [k, v] of Object.entries(t)) {
      if (/^E\d+D\d+$/.test(k)) {
        const num = Number(v);
        if (Number.isFinite(num)) vals.push(num);
      }
    }
    out.strata.push(vals);
  }
  // The dose-response exposure-score column is stored outside the data cells.
  for (const v of dm?.[0]?.Evals ?? []) {
    const num = Number(v);
    if (Number.isFinite(num)) out.evals.push(num);
  }
  return out;
}

// ---------- panel extraction ----------
// ownText: text belonging to a cell itself, excluding nested result tables
// (descendant mashup was the concat defect). tr.cells gives the row's own
// cells; rows inside nested tables are visited separately.

function readPanelRows() {
  const ownText = (el) => {
    let s = '';
    for (const n of el.childNodes) {
      if (n.nodeType === 3) s += n.textContent;
      else if (n.nodeType === 1 && n.tagName !== 'TABLE') s += ownText(n);
    }
    return s;
  };
  const panel = document.getElementById('panel3');
  const rows = [];
  if (!panel) return rows;
  for (const tr of panel.querySelectorAll('tr')) {
    const cells = [...tr.cells].map((c) => ownText(c).replace(/\s+/g, ' ').trim());
    rows.push({ th: [...tr.cells].some((c) => c.tagName === 'TH'), cells });
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

// ---------- compare with R fixtures (--compare-r) ----------
// For every mapped value, resolve its statistic path in
// fixtures/calc/<id>/<case>.json `expected` (leaves are {value, class}) and
// report |openepi - R| against 0.5 x 10^-decimals. A mapped path with no leaf
// in that case's fixture is listed as NO-R-COUNTERPART (the case's fixture
// does not cover that statistic, e.g. proportion's ztest needs a
// compare_with input the case lacks) and does not fail the run; a missing
// fixture file still fails (UNRESOLVED, non-zero exit). Fixture leaves with
// value null are listed as R-NULL; mismatches are listed, not failures.

// Returns {found: true, value} for a numeric leaf, {found: true, value: null}
// when the fixture marks the leaf undefined, {found: false} when absent.
export function resolveStatPath(fixture, stat) {
  const parts = stat.split('.');
  let node = fixture.expected;
  for (const p of parts.slice(2)) {
    if (node == null || typeof node !== 'object') return { found: false };
    node = node[p];
  }
  if (node == null || typeof node !== 'object' || !('value' in node)) return { found: false };
  return { found: true, value: node.value };
}

function compareR() {
  let ok = 0;
  let mismatch = 0;
  let unresolved = 0;
  let norcounterpart = 0;
  let rnull = 0;
  const files = fs.readdirSync(DIR).filter((f) => f.endsWith('.json') && !OUTPUT_SKIP.has(f)).sort();
  for (const f of files) {
    const doc = JSON.parse(fs.readFileSync(path.join(DIR, f), 'utf8'));
    const items = [];
    for (const c of doc.cases) {
      const fxPath = path.join(REPO, 'fixtures', 'calc', doc.calculator, `${c.case}.json`);
      if (!fs.existsSync(fxPath)) {
        unresolved += 1;
        items.push(`UNRESOLVED ${c.case} (no fixture)`);
        continue;
      }
      const fx = JSON.parse(fs.readFileSync(fxPath, 'utf8'));
      for (const v of c.values) {
        if (!v.statistic || typeof v.value !== 'number') continue;
        const r = resolveStatPath(fx, v.statistic);
        if (!r.found) {
          norcounterpart += 1;
          items.push(`NO-R-COUNTERPART ${c.case} '${v.label}' -> ${v.statistic} openepi=${v.value}`);
          continue;
        }
        if (typeof r.value !== 'number') {
          rnull += 1;
          items.push(`R-NULL ${c.case} '${v.label}' -> ${v.statistic} openepi=${v.value}`);
          continue;
        }
        const tol = 0.5 * 10 ** -v.decimals;
        if (Math.abs(v.value - r.value) <= tol) ok += 1;
        else {
          mismatch += 1;
          items.push(`${c.case} '${v.label}' -> ${v.statistic} openepi=${v.value} R=${r.value} tol=${tol}`);
        }
      }
    }
    if (items.length) {
      console.log(`== ${doc.calculator} ${items.length}`);
      for (const i of items) console.log(`   ${i}`);
    }
  }
  console.log(`ok ${ok} mismatch ${mismatch} unresolved ${unresolved} no-r-counterpart ${norcounterpart} r-null ${rnull}`);
  return unresolved ? 1 : 0;
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
  const strataFills = Array.isArray(filled) ? [await setCells(page, filled)] : await filled;
  const lastWrite = strataFills.at(-1)?.writes.at(-1);
  if (lastWrite) await syncEntry(page, lastWrite[0], lastWrite[1]);
  await page.click('#calc');
  await page.waitForTimeout(1500);

  // Input guard: every input OpenEpi must hold appears in the array it
  // actually computed on (dataMatrix). Pages legitimately hold MORE (derived
  // values written back, dose scores in Evals) — the written inputs are a
  // subset of what is held.
  const matrix = await page.evaluate(readDataMatrix);
  const guardErrors = [];
  strataFills.forEach((f, s) => {
    for (const e of f.errors) guardErrors.push(`stratum ${s + 1}: ${e}`);
    const want = (f.dataWrites ?? []).map((w) => Number(w[2])).filter(Number.isFinite);
    const got = matrix.strata[s];
    if (!got) { guardErrors.push(`stratum ${s + 1}: no dataMatrix entry`); return; }
    const missing = multisetDiff(want, got);
    if (missing.length) {
      guardErrors.push(`stratum ${s + 1}: OpenEpi did not hold inputs [${missing}] (held [${[...got].sort((a, b) => a - b)}])`);
    }
    const nonDataWant = (f.nonDataWrites ?? []).map((w) => Number(w[2])).filter(Number.isFinite);
    if (nonDataWant.length) {
      const miss2 = multisetDiff(nonDataWant, matrix.evals);
      if (miss2.length) guardErrors.push(`stratum ${s + 1}: non-data inputs [${miss2}] not in OpenEpi's Evals [${matrix.evals}]`);
    }
  });
  if (matrix.strata.length && matrix.strata.length !== strataFills.length) {
    guardErrors.push(`OpenEpi computed ${matrix.strata.length} strata, expected ${strataFills.length}`);
  }
  if (c.inputs.conf_level != null) {
    const cl = await page.evaluate(() => window.ConfLevel);
    if (String(cl) !== String(c.inputs.conf_level)) {
      guardErrors.push(`ConfLevel=${cl}, expected ${c.inputs.conf_level}`);
    }
  }

  const missingLabels = [];
  // Person-Time pages choose their own display unit ("Per 1000 Person-Time
  // Units"); read it so displayed rates can be restated on the case's
  // per_unit scale.
  const panelText = await page.evaluate(() => document.getElementById('panel3')?.innerText ?? '');
  const ptm = /Per\s+([\d,]+)\s+Person-Time/i.exec(panelText);
  const ptunit = ptm ? Number(ptm[1].replace(/,/g, '')) : null;

  const values = [];
  const rows = await page.evaluate(readPanelRows);
  for (const { label, text } of emissions(rows)) {
    const parsed = parseDisplayed(text);
    if (!parsed) continue;
    if (parsed.value === null) {
      values.push({ label, statistic: null, value: parsed.text, decimals: parsed.decimals });
      continue;
    }
    const { stat, missing } = statForLabel(drv, labels, c.calculator, label, c.inputs);
    if (missing) missingLabels.push(label);
    const scaled = stat ? scaleToR(c.calculator, stat, c.inputs, parsed, ptunit) : parsed;
    values.push({ label, statistic: stat, value: scaled.value, decimals: scaled.decimals });
  }
  await page.close();
  return { case: c.case, values, _meta: { url: pageUrl, dialogs, missingLabels, guardErrors } };
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
  if (COMPARE_R) { process.exit(compareR()); }

  const labels = fs.existsSync(LABELS_PATH) ? JSON.parse(fs.readFileSync(LABELS_PATH, 'utf8')) : {};
  const run = cases.filter((c) => (!ONLY_CALC || c.calculator === ONLY_CALC) && (!ONLY_CASE || c.case === ONLY_CASE));
  const { chromium } = await import('playwright');
  const allMissing = [];
  const allGuard = [];
  // One fresh browser per calculator: bounds memory, and a crash loses at
  // most the calculator in flight (its output is only written when complete).
  const calcs = [...new Set(run.map((c) => c.calculator))];
  for (const calc of calcs) {
    const calcCases = run.filter((c) => c.calculator === calc);
    const browser = await chromium.launch({
      ...(PROXY ? { proxy: { server: PROXY } } : {}),
      ...(CHROMIUM_PATH ? { executablePath: CHROMIUM_PATH } : {}),
    });
    const caseList = [];
    try {
      for (const c of calcCases) {
        process.stderr.write(`capture ${c.calculator}/${c.case} … `);
        let got;
        try {
          got = await captureCase(browser, c, labels);
        } catch (e) {
          got = { case: c.case, values: [], _meta: { url: '', dialogs: [`capture-error: ${String(e).split('\n')[0]}`], missingLabels: [], guardErrors: [] } };
        }
        const { _meta, ...out } = got;
        caseList.push(out);
        allMissing.push(..._meta.missingLabels.map((l) => `${c.calculator}/${c.case}: '${l}'`));
        allGuard.push(..._meta.guardErrors.map((e) => `${c.calculator}/${c.case}: ${e}`));
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
  }
  if (allGuard.length) {
    console.error('input guard failures (OpenEpi held different inputs than cases.json):');
    for (const e of allGuard) console.error(`  ${e}`);
  }
  if (allMissing.length || allGuard.length) process.exit(1);
}

if (process.argv[1] && url.pathToFileURL(process.argv[1]).href === import.meta.url) {
  main().catch((e) => { console.error(e); process.exit(1); });
}
