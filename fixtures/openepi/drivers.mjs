// Per-calculator OpenEpi page drivers and Task-1 input specs for capture.mjs.
// Grid coordinates come from each page's rendered Etable (myTable.row[r].cell[c]),
// verified against the live site on 2026-11-13.
// Pages without a confidence-level cell read the global ConfLevel (ReadCookie.js /
// DataStore.js); capture.mjs sets it whenever inputs.conf_level is present.
//
// OE_BASE is only for offline development against a local mirror; the workflow
// never sets it.

const OE = process.env.OE_BASE ?? 'https://www.openepi.com';

// --- helpers passed to driver fills -------------------------------------
//
// myTable.insert stores values verbatim and OpenEpi's page code does `a + b`
// arithmetic on them: interactive entry goes through evalEntry which
// parseFloats, so a string "50" + "100" displays as "50100" (the concat defect
// in the first capture). Numbers must be inserted as numbers. Every data cell
// NOT in `writes` is cleared so leftover demo data cannot leak into a case
// (e.g. the ANOVA/Proportion demo rows). Returns the verification result:
// the number of data cells the table holds and any mismatches.

export async function setCells(page, writes) {
  return page.evaluate((ws) => {
    const t = window.myTable;
    const keep = new Set(ws.map(([r, c]) => `${r},${c}`));
    t.row.forEach((row, r) => row.cell.forEach((cell, c) => {
      if (cell.type === 'data' && !keep.has(`${r},${c}`)) t.insert(r, c, '');
    }));
    for (const [r, c, v] of ws) {
      const n = Number(v);
      t.insert(r, c, v === '' || !Number.isFinite(n) ? v : n);
      if (typeof t.calcTotals === 'function') t.calcTotals(r, c);
    }
    let dataCells = 0;
    const errors = [];
    const written = new Map(ws.map(([r, c, v]) => [`${r},${c}`, v]));
    const dataWrites = [];
    t.row.forEach((row, r) => row.cell.forEach((cell, c) => {
      if (cell.type !== 'data') return;
      dataCells += 1;
      const want = written.get(`${r},${c}`);
      if (want === undefined) {
        if (!(cell.value === '' || cell.value == null || Number(cell.value) === 0)) {
          errors.push(`data cell ${r},${c} holds ${JSON.stringify(cell.value)} but no input was written`);
        }
      } else {
        dataWrites.push([r, c, want]);
        if (want === '') {
          if (!(cell.value === '' || Number(cell.value) === 0)) {
            errors.push(`data cell ${r},${c} holds ${JSON.stringify(cell.value)} expected empty`);
          }
        } else if (Number(cell.value) !== Number(want)) {
          errors.push(`data cell ${r},${c} holds ${JSON.stringify(cell.value)} expected ${JSON.stringify(want)}`);
        }
      }
    }));
    // Writes aimed at non-data cells (e.g. the dose-response exposure-score
    // column, which is not a 'data' cell) are legal but verified here.
    const nonDataWrites = [];
    ws.forEach(([r, c, v]) => {
      const cell = t.row[r]?.cell[c];
      if (cell && cell.type !== 'data') {
        nonDataWrites.push([r, c, v]);
        if (String(cell.value ?? '') !== String(v)) {
          errors.push(`non-data cell ${r},${c} holds ${JSON.stringify(cell.value)} expected ${JSON.stringify(v)}`);
        }
      }
    });
    return { writes: ws, dataWrites, nonDataWrites, dataCells, errors };
  }, writes);
}

// After filling, point the input cursor at the last written data cell with its
// own value in the entry field, so the evalEntry() that addStratum/Calculate
// call is an idempotent rewrite of that same cell (as it is for a human whose
// cursor sits on the last cell they typed).
export async function syncEntry(page, r, c) {
  await page.evaluate(([rr, cc]) => {
    const t = window.myTable;
    t.inputR = rr;
    t.inputC = cc;
    const box = document.inBox?.entryfield ?? document.querySelector('input[name="entryfield"]');
    if (box) box.value = String(t.row[rr].cell[cc].value ?? '');
  }, [r, c]);
}

export async function addStratum(page) {
  await page.click('input[name="btnAddStrat"]');
  await page.waitForTimeout(400);
}

// Writes stratum tables one stratum at a time, clicking "Add Stratum" between
// strata. cellsFor(stratumInputs) must return [row, col, value] writes for the
// currently displayed stratum. Returns one {writes,dataCells,errors} per
// stratum for the post-calculation dataMatrix guard.
export async function fillStrata(page, strata, cellsFor) {
  const out = [];
  for (let s = 0; s < strata.length; s++) {
    if (s > 0) await addStratum(page);
    const res = await setCells(page, cellsFor(strata[s]));
    out.push(res);
    if (res.writes.length) {
      const [r, c] = res.writes[res.writes.length - 1];
      await syncEntry(page, r, c);
    }
  }
  return out;
}

const t22 = (m) => [[2, 2, m[0][0]], [2, 3, m[0][1]], [3, 2, m[1][0]], [3, 3, m[1][1]]];

// Fills every cell whose Etable type is "data" in row-major order. Used for
// pages whose table dimensions are chosen by prompts (RbyC, DiagnosticTest).
export async function fillDataRowMajor(page, flat) {
  const coords = await page.evaluate(() => {
    const out = [];
    window.myTable.row.forEach((row, r) =>
      row.cell.forEach((c, col) => { if (c.type === 'data') out.push([r, col]); }));
    return out;
  });
  if (coords.length !== flat.length) {
    throw new Error(`data-cell count ${coords.length} != values ${flat.length}`);
  }
  return [await setCells(page, coords.map(([r, c], i) => [r, c, flat[i]]))];
}

// --- drivers -------------------------------------------------------------
// Each entry: url (string | (inputs) => string), optional prompts(inputs)
// answered in order when "Enter New Data" asks, and fill(page, inputs).

export const CALCULATORS = {
  proportion: {
    url: `${OE}/Proportion/Proportion.htm`,
    // population_size 0 = infinite in our corpus; OpenEpi's infinite sentinel
    // is "leave as 1000000" (FPC applies only for 2 < N < 1e6). Same input
    // semantically; the fixture's 0 maps to OpenEpi's 1000000.
    fill: (i) => [
      [0, 2, i.numerator], [1, 2, i.denominator], [2, 2, i.multiplier],
      [3, 2, i.population_size === 0 ? 1000000 : i.population_size],
      [4, 2, i.compare_with ?? ''],
    ],
  },
  two_by_two: {
    url: `${OE}/TwobyTwo/TwobyTwo.htm`,
    fill: (i, p) => fillStrata(p, i.strata, t22),
  },
  dose_response: {
    url: `${OE}/DoseResponse/DoseResponse.htm`,
    prompts: (i) => [i.table[0].length], // exposure level categories incl. baseline
    fill: (i, p) => fillStrata(p, i.table, (rows) =>
      rows.flatMap(([score, cases, controls], r) => [
        [r + 1, 0, score], [r + 1, 1, cases], [r + 1, 2, controls],
      ])),
  },
  r_by_c: {
    url: `${OE}/RbyC/RbyC.htm`,
    prompts: (i) => [i.table[0].length, i.table.length], // columns prompt first, then rows
    fill: (i, p) => fillDataRowMajor(p, i.table.flat()),
  },
  matched_case_control: {
    url: `${OE}/MatchCC/MatchCC.htm`,
    fill: (i) => t22(i.matched_sets), // M = 1 pair table only; 1:n has no OpenEpi input
  },
  screening: {
    url: `${OE}/DiagnosticTest/DiagnosticTest.htm`,
    prompts: (i) => [i.table.length], // number of diagnostic outcome levels
    fill: (i, p) => fillDataRowMajor(p, i.table.flat()),
  },
  smr: {
    url: `${OE}/SMR/SMR.htm`,
    fill: (i) => [[0, 2, i.observed], [1, 2, i.expected]],
  },
  rate_one: {
    url: `${OE}/PersonTime1/PersonTime1.htm`,
    fill: (i) => [[0, 2, i.cases], [1, 2, i.person_time]],
  },
  rate_compare: {
    url: `${OE}/PersonTime2/PersonTime2.htm`,
    fill: (i, p) => fillStrata(p, i.strata, t22),
  },
  mean_ci: {
    url: `${OE}/Mean/CIMean.htm`,
    fill: (i) => [
      [0, 2, i.mean],
      ...(i.sd_or === 'sd' ? [[1, 2, i.sd_value]] : i.sd_or === 'se' ? [[1, 4, i.sd_value]] : [[1, 6, i.sd_value]]),
      [2, 2, i.n], [3, 2, i.population_size], [4, 2, i.conf_level],
    ],
  },
  median_ci: {
    url: `${OE}/Median/CIMedian.htm`,
    fill: (i) => [[0, 3, i.n], [1, 3, i.percentile], [2, 3, i.conf_level]],
  },
  t_test_summary: {
    url: `${OE}/Mean/t_testMean.htm`,
    fill: (i) => [
      [0, 3, i.conf_level],
      [2, 1, i.n_1], [2, 2, i.mean_1], [2, 3, i.sd_1],
      [3, 1, i.n_2], [3, 2, i.mean_2], [3, 3, i.sd_2],
    ],
  },
  anova_summary: {
    url: `${OE}/Mean/ANOVA.htm`,
    fill: (i) => i.table.flatMap(([n, mean, sd], g) => [
      [g + 1, 1, n], [g + 1, 2, mean], [g + 1, 3, sd],
    ]),
  },
  ss_proportion: {
    url: `${OE}/SampleSize/SSPropor.htm`,
    // `clusters` is EI7-only; OpenEpi has no cell for it and ignores it.
    fill: (i) => [
      [0, 3, i.population_size], [1, 3, i.expected_freq],
      [2, 3, i.margin_error], [3, 3, i.design_effect],
    ],
  },
  ss_unmatched_cc: {
    url: `${OE}/SampleSize/SSCC.htm`,
    fill: (i) => [
      [0, 3, i.conf_level], [1, 3, i.power],
      [2, 3, i.ratio_controls_per_case], [3, 3, i.percent_controls_exposed],
      ...(i.odds_ratio != null ? [[5, 3, i.odds_ratio]] : [[6, 3, i.percent_cases_exposed]]),
    ],
  },
  ss_cohort: {
    url: `${OE}/SampleSize/SSCohort.htm`,
    fill: (i) => [
      [0, 3, i.conf_level], [1, 3, i.power],
      [2, 3, i.ratio_unexposed_per_exposed], [3, 3, i.percent_unexposed_outcome],
      ...(i.odds_ratio != null ? [[5, 3, i.odds_ratio]]
        : i.percent_exposed_outcome != null ? [[6, 3, i.percent_exposed_outcome]]
        : i.risk_ratio != null ? [[7, 3, i.risk_ratio]]
        : [[8, 3, i.risk_difference]]),
    ],
  },
  ss_mean_difference: {
    url: `${OE}/SampleSize/SSMean.htm`,
    fill: (i) => [
      [0, 3, i.conf_level], [1, 3, i.power], [2, 3, i.ratio],
      ...(i.difference != null ? [[4, 6, i.difference]] : [[4, 1, i.mean_1], [4, 3, i.mean_2]]),
      ...(i.sd_1 != null ? [[5, 1, i.sd_1], [5, 3, i.sd_2]] : [[6, 1, i.var_1], [6, 3, i.var_2]]),
    ],
  },
  power_unmatched_cc: {
    url: `${OE}/Power/PowerCC.htm`,
    fill: (i) => [
      [0, 2, i.conf_level],
      [2, 2, i.n_cases], [2, 3, i.n_controls],
      [3, 2, i.percent_cases_exposed], [3, 3, i.percent_controls_exposed],
    ],
  },
  power_cohort: {
    // The three OpenEpi pages are one computation (Task 1 merge); design picks the page.
    url: (i) => `${OE}/Power/${{ cohort: 'PowerCohort', rct: 'PowerRCT', cross_sectional: 'PowerCross' }[i.design]}.htm`,
    canonical: `${OE}/Power/PowerCohort.htm`,
    fill: (i) => [
      [0, 2, i.conf_level],
      [2, 2, i.n_exposed], [2, 3, i.n_unexposed],
      [3, 2, i.percent_exposed_outcome], [3, 3, i.percent_unexposed_outcome],
    ],
  },
  power_mean_difference: {
    url: `${OE}/Power/PowerMean.htm`,
    fill: (i) => [
      [0, 3, i.conf_level],
      ...(i.difference != null ? [[2, 6, i.difference]] : [[2, 1, i.mean_1], [2, 3, i.mean_2]]),
      [3, 1, i.n_1], [3, 3, i.n_2],
      ...(i.sd_1 != null ? [[4, 1, i.sd_1], [4, 3, i.sd_2]] : [[5, 1, i.var_1], [5, 3, i.var_2]]),
    ],
  },
};

// --- Task 1 input specs (docs/v1.0-calculators.md) for --dry-run -----------
// f = field validator name; required list first, optional after "?".

const V = {
  count: (v) => Number.isInteger(v) && v >= 0,
  int: (v) => Number.isInteger(v),
  pos: (v) => typeof v === 'number' && v > 0,
  real: (v) => typeof v === 'number' && Number.isFinite(v),
  pct: (v) => typeof v === 'number' && v >= 0 && v <= 100,
  conf: (v) => [90, 95, 99].includes(v),
  mat: (v) => Array.isArray(v) && v.every((r) => Array.isArray(r) && r.every((x) => typeof x === 'number')),
  mat22: (v) => Array.isArray(v) && v.every((m) => Array.isArray(m) && m.length === 2 &&
    m.every((r) => Array.isArray(r) && r.length === 2 && r.every((x) => typeof x === 'number'))),
  strata3: (v) => Array.isArray(v) && v.every((s) => Array.isArray(s) &&
    s.every((r) => Array.isArray(r) && r.length === 3 && r.every((x) => typeof x === 'number'))),
};

export const INPUT_SPECS = {
  proportion: { req: ['numerator', 'denominator', 'conf_level', 'multiplier', 'population_size'], opt: ['compare_with'], numerator: 'count', denominator: 'count', conf_level: 'conf', multiplier: 'pos', population_size: 'count', compare_with: 'pct' },
  two_by_two: { req: ['strata', 'conf_level', 'study_type'], strata: 'mat22', conf_level: 'conf', study_type: ['cohort', 'case_control'] },
  dose_response: { req: ['table', 'strata'], table: 'strata3', strata: 'int' },
  r_by_c: { req: ['table'], table: 'mat' },
  matched_case_control: { req: ['controls_per_case', 'matched_sets', 'conf_level'], controls_per_case: 'int', matched_sets: 'mat', conf_level: 'conf' },
  screening: { req: ['table', 'conf_level'], table: 'mat', conf_level: 'conf' },
  smr: { req: ['observed', 'expected', 'conf_level'], observed: 'count', expected: 'pos', conf_level: 'conf' },
  rate_one: { req: ['cases', 'person_time', 'per_unit', 'conf_level'], cases: 'count', person_time: 'pos', per_unit: 'pos', conf_level: 'conf' },
  rate_compare: { req: ['strata', 'per_unit', 'conf_level'], strata: 'mat22', per_unit: 'pos', conf_level: 'conf' },
  mean_ci: { req: ['mean', 'sd_or', 'sd_value', 'n', 'population_size', 'conf_level'], mean: 'real', sd_or: ['sd', 'se', 'variance'], sd_value: 'pos', n: 'int', population_size: 'count', conf_level: 'conf' },
  median_ci: { req: ['n', 'percentile', 'conf_level'], n: 'int', percentile: 'pct', conf_level: 'conf' },
  t_test_summary: { req: ['conf_level', 'n_1', 'mean_1', 'sd_1', 'n_2', 'mean_2', 'sd_2'], conf_level: 'conf', n_1: 'int', mean_1: 'real', sd_1: 'pos', n_2: 'int', mean_2: 'real', sd_2: 'pos' },
  anova_summary: { req: ['table'], table: 'mat' },
  ss_proportion: { req: ['population_size', 'expected_freq', 'margin_error', 'design_effect', 'clusters'], population_size: 'count', expected_freq: 'pct', margin_error: 'pos', design_effect: 'pos', clusters: 'int' },
  ss_unmatched_cc: { req: ['conf_level', 'power', 'ratio_controls_per_case', 'percent_controls_exposed'], oneOf: [['odds_ratio', 'percent_cases_exposed']], conf_level: 'conf', power: 'pct', ratio_controls_per_case: 'pos', percent_controls_exposed: 'pct', odds_ratio: 'pos', percent_cases_exposed: 'pct' },
  ss_cohort: { req: ['conf_level', 'power', 'ratio_unexposed_per_exposed', 'percent_unexposed_outcome'], oneOf: [['odds_ratio', 'percent_exposed_outcome', 'risk_ratio', 'risk_difference']], conf_level: 'conf', power: 'pct', ratio_unexposed_per_exposed: 'pos', percent_unexposed_outcome: 'pct', odds_ratio: 'pos', percent_exposed_outcome: 'pct', risk_ratio: 'pos', risk_difference: 'real' },
  ss_mean_difference: { req: ['conf_level', 'power', 'ratio'], oneOf: [['difference', 'mean_pair'], ['sd_pair', 'var_pair']], conf_level: 'conf', power: 'pct', ratio: 'pos', mean_1: 'real', mean_2: 'real', difference: 'real', sd_1: 'pos', sd_2: 'pos', var_1: 'pos', var_2: 'pos' },
  power_unmatched_cc: { req: ['conf_level', 'n_cases', 'n_controls', 'percent_cases_exposed', 'percent_controls_exposed'], conf_level: 'conf', n_cases: 'int', n_controls: 'int', percent_cases_exposed: 'pct', percent_controls_exposed: 'pct' },
  power_cohort: { req: ['design', 'conf_level', 'n_exposed', 'n_unexposed', 'percent_exposed_outcome', 'percent_unexposed_outcome'], design: ['cohort', 'rct', 'cross_sectional'], conf_level: 'conf', n_exposed: 'int', n_unexposed: 'int', percent_exposed_outcome: 'pct', percent_unexposed_outcome: 'pct' },
  power_mean_difference: { req: ['conf_level', 'n_1', 'n_2'], oneOf: [['difference', 'mean_pair'], ['sd_pair', 'var_pair']], conf_level: 'conf', n_1: 'int', n_2: 'int', mean_1: 'real', mean_2: 'real', difference: 'real', sd_1: 'pos', sd_2: 'pos', var_1: 'pos', var_2: 'pos' },
};

// Field aliases for oneOf groups that come in pairs.
const GROUPS = { mean_pair: ['mean_1', 'mean_2'], sd_pair: ['sd_1', 'sd_2'], var_pair: ['var_1', 'var_2'] };

export function validateInputs(calc, inputs) {
  const spec = INPUT_SPECS[calc];
  const errs = [];
  const present = (k) => inputs[k] !== undefined;
  for (const k of spec.req) {
    if (!present(k)) errs.push(`missing required input '${k}'`);
  }
  for (const group of spec.oneOf ?? []) {
    const hits = group.filter((alt) => (GROUPS[alt] ?? [alt]).every(present));
    if (hits.length !== 1) errs.push(`expected exactly one of ${group.join('|')}, got ${hits.length}`);
  }
  for (const [k, v] of Object.entries(inputs)) {
    if (['req', 'opt', 'oneOf'].includes(k)) continue;
    const rule = spec[k];
    if (rule === undefined) { errs.push(`unknown input '${k}'`); continue; }
    const ok = Array.isArray(rule) ? rule.includes(v) : V[rule](v);
    if (!ok) errs.push(`${k}=${JSON.stringify(v)} fails '${Array.isArray(rule) ? rule.join('|') : rule}'`);
  }
  return errs;
}
