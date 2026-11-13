// Unit tests for the pure parts of capture.mjs: displayed-number parsing,
// results-row emission (per-cell values, sections, dedupe) and the R-fixture
// comparison used by --compare-r. Runs with `node --test capture.test.mjs`;
// no network, no Playwright.

import test from 'node:test';
import assert from 'node:assert/strict';
import {
  parseDisplayed,
  cellParts,
  emissions,
  scaleToR,
  multisetEqual,
  resolveStatPath,
} from './capture.mjs';

test('parseDisplayed: plain numbers and trailing zeros', () => {
  assert.deepEqual(parseDisplayed('12.55'), { value: 12.55, decimals: 2, percent: false, bound: null, text: '12.55' });
  assert.equal(parseDisplayed('0.0269').decimals, 4);
  assert.equal(parseDisplayed('0.02690').decimals, 5);
  assert.equal(parseDisplayed('5.460').decimals, 3);
  assert.equal(parseDisplayed('1,234').value, 1234);
  assert.equal(parseDisplayed('3.58e17').decimals, 0);
  assert.equal(parseDisplayed('1e-07').decimals, 7);
});

test('parseDisplayed: percent marker kept separately', () => {
  const p = parseDisplayed('63.92 %');
  assert.equal(p.value, 63.92);
  assert.equal(p.decimals, 2);
  assert.equal(p.percent, true);
});

test('parseDisplayed: bounds are not equalities', () => {
  const b = parseDisplayed('<0.0000001');
  assert.equal(b.value, null);
  assert.equal(b.bound, '<');
  assert.equal(b.decimals, 7);
  assert.equal(parseDisplayed('>0.9999999').bound, '>');
});

test('parseDisplayed: footnote marks do not change decimals', () => {
  assert.equal(parseDisplayed('1.23°').decimals, 2);
  assert.equal(parseDisplayed('0.66¹').value, 0.66);
});

test('cellParts: one value per numeric cell, pairs split', () => {
  assert.deepEqual(cellParts('47.62%'), ['47.62%']);
  assert.deepEqual(cellParts('(5.06 - 12.0)'), ['5.06', '12.0']);
  assert.deepEqual(cellParts('0.66 ,   1.23°'), ['0.66', '1.23°']);
  assert.deepEqual(cellParts('29,24'), ['29', '24']);
  // OpenEpi formatters never emit thousands separators, so "4499,4999" is
  // the df(numerator,denominator) pair, not a single large number.
  assert.deepEqual(cellParts('4499,4999'), ['4499', '4999']);
  assert.deepEqual(cellParts('1,234'), ['1', '234']);
  assert.deepEqual(cellParts("'?'"), ["'?'"]);
  assert.equal(cellParts('Level 1'), null);
  assert.equal(cellParts('Taylor series'), null);
  assert.equal(cellParts('-'), null);
  assert.equal(cellParts(''), null);
});

test('emissions: each numeric cell yields its own value (no concat)', () => {
  const rows = emissions([
    { cells: ['', 'Uncorrected chi square', '2.33', '0.06347', '0.127'] },
  ]);
  assert.deepEqual(rows.map((r) => r.text), ['2.33', '0.06347', '0.127']);
  assert.deepEqual(rows.map((r) => r.label), [
    'Uncorrected chi square [1]',
    'Uncorrected chi square [2]',
    'Uncorrected chi square [3]',
  ]);
});

test('emissions: identical repeated tables dedupe (DoseResponse double-write)', () => {
  const block = [
    { cells: ['Exposure Level', 'Cases', 'Controls'] },
    { cells: ['0', '50', '100'] },
    { cells: ['', 'Extended Mantel-Haenszel chi square for linear trend=', '3.62'] },
  ];
  const rows = emissions([...block, ...block]);
  assert.equal(rows.filter((r) => r.label === 'Extended Mantel-Haenszel chi square for linear trend= [1]').length, 1);
});

test('emissions: same label in different sections gets section prefix', () => {
  const rows = emissions([
    { th: true, cells: ['Statistics / d.f. / p-value (2 sided)'] },
    { cells: ['Byar approximation:', '4.64', '0.03131'] },
    { th: true, cells: ['Lower CL / SMR / Upper CL'] },
    { cells: ['Byar approximation:', '1.043', '1.815'] },
  ]);
  const labels = rows.map((r) => r.label);
  assert.ok(labels.some((l) => l.startsWith('Statistics') && l.includes('Byar approximation')));
  assert.ok(labels.some((l) => l.startsWith('Lower CL') && l.includes('Byar approximation')));
});

test('emissions: leading stratum cell prefixes the label', () => {
  const rows = emissions([
    { cells: ['1', 'z-score', '2.28', '0.01124', '0.02248'] },
  ]);
  assert.equal(rows[0].label, '1 / z-score [1]');
  assert.equal(rows[0].text, '2.28');
});

test('emissions: all-numeric rows take the last header label', () => {
  const rows = emissions([
    { cells: ['Exposure Level', 'Cases', 'Controls', 'Total'] },
    { cells: ['0', '50', '100', '150'] },
    { cells: ['', 'Total', '50', '100', '150'] },
  ]);
  assert.equal(rows[0].label, 'Exposure Level [1]');
  assert.equal(rows[4].label, 'Total [1]');
});

test('emissions: single-cell "label = value" rows', () => {
  const rows = emissions([{ cells: ['Two-sided p-value=<0.0000001'] }]);
  assert.equal(rows[0].label, 'Two-sided p-value');
  assert.equal(rows[0].text, '<0.0000001');
});

test('emissions: blank placeholder cells are skipped', () => {
  const rows = emissions([{ cells: ['', 'Mid-P exact test:', '-', '-', '0.02626'] }]);
  assert.equal(rows.length, 1);
  assert.equal(rows[0].label, 'Mid-P exact test: [1]');
  assert.equal(rows[0].text, '0.02626');
});

test('scaleToR: percent displays only for fraction-scale stats', () => {
  // proportion: multiplier 100 -> /100 and +2 decimals
  const p = { value: 12.55, decimals: 2, percent: false, bound: null, text: '12.55' };
  const s = scaleToR('proportion', 'calc.proportion.ci.wilson.lower', { multiplier: 100 }, p);
  assert.equal(s.value, 0.1255);
  assert.equal(s.decimals, 4);
  // screening: % display -> /100 for est/lower/upper
  const g = { value: 60.0, decimals: 1, percent: true, bound: null, text: '60.0%' };
  const s2 = scaleToR('screening', 'calc.screening.sensitivity.est', {}, g);
  assert.equal(s2.value, 0.6);
  assert.equal(s2.decimals, 3);
  // power: % display but fixture is percent-scale -> unchanged
  const w = { value: 63.92, decimals: 2, percent: true, bound: null, text: '63.92%' };
  const s3 = scaleToR('power_cohort', 'calc.power_cohort.normal', {}, w);
  assert.equal(s3.value, 63.92);
  assert.equal(s3.decimals, 2);
  // two_by_two directly-adjusted RD: percent display, fraction-scale fixture
  const rd = { value: 17.72, decimals: 2, percent: false, bound: null, text: '17.72' };
  const s4 = scaleToR('two_by_two', 'calc.two_by_two.adjusted.direct_rd.est', {}, rd);
  assert.equal(s4.value, 0.1772);
  assert.equal(s4.decimals, 4);
  // strata/crude RD stays percent-scale (fixture is percent-scale too)
  const s5 = scaleToR('two_by_two', 'calc.two_by_two.crude.risk_difference.est', {}, rd);
  assert.equal(s5.value, 17.72);
});

test('scaleToR: person-time rates restated on the case per_unit scale', () => {
  // OpenEpi picked ptunit=1000, the case wants per-10000 -> x10, decimals-1
  const r = { value: 1.667, decimals: 3, percent: false, bound: null, text: '1.667' };
  const s = scaleToR('rate_one', 'calc.rate_one.estimate', { per_unit: 10000 }, r, 1000);
  assert.equal(s.value, 16.67);
  assert.equal(s.decimals, 2);
  // directly adjusted RD is stored per single person-time unit -> /ptunit
  const d = { value: 8.542, decimals: 3, percent: false, bound: null, text: '8.542' };
  const s2 = scaleToR('rate_compare', 'calc.rate_compare.adjusted.direct_rd.est', { per_unit: 10000 }, d, 1000);
  assert.equal(s2.value, 0.008542);
  assert.equal(s2.decimals, 6);
  // unitless statistics are untouched
  const z = { value: 2.282, decimals: 3, percent: false, bound: null, text: '2.282' };
  const s3 = scaleToR('rate_compare', 'calc.rate_compare.crude.zscore.value', { per_unit: 10000 }, z, 1000);
  assert.equal(s3.value, 2.282);
});

test('multisetEqual: guard comparison', () => {
  assert.ok(multisetEqual([1, 2, 0, 0], [0, 2, 1, 0]));
  assert.ok(!multisetEqual([1, 2, 0, 0], [1, 2, 3, 0]));
});

test('resolveStatPath: walks fixture expected to {value,class} leaf', () => {
  const fx = { expected: { anova: { ss_between: { value: 187.5, class: 'closed_form' } } } };
  assert.deepEqual(resolveStatPath(fx, 'calc.anova_summary.anova.ss_between'), { found: true, value: 187.5 });
  assert.deepEqual(resolveStatPath(fx, 'calc.anova_summary.anova.missing'), { found: false });
});
