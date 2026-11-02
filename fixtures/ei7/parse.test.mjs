// parse.test.mjs — tests for the Epi Info 7 ROUTEOUT HTML parser
// (v0.2 Task 3). The parser must reproduce fixtures/ei7/oswego.json
// byte-for-byte from the committed fixtures/ei7/oswego.html.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseHtml } from './parse.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const labels = JSON.parse(readFileSync(join(HERE, 'labels.json'), 'utf8'));

const SNIPPET = `<!DOCTYPE HTML>
<html><body>
<a name="#Results0"></a><h3>TABLES VANILLA ILL</h3><hr/>
<table align="left"><tr><th>&nbsp;</th><th colspan=2>ILL</th><th>&nbsp;</th></tr>
<tr><th>VANILLA</th><th>1</th><th>0</th><th>Total</th></tr>
<tr><td><b>1</b></td><td>43</td><td>11</td><td>54</td></tr>
<tr><td>Row%</td><td>79.63%</td><td>20.37%</td><td>100.00%</td></tr>
<tr><td>Col%</td><td>93.48%</td><td>37.93%</td><td>72.00%</td></tr>
<tr><td><b>0</b></td><td>3</td><td>18</td><td>21</td></tr>
<tr><td>Row%</td><td>14.29%</td><td>85.71%</td><td>100.00%</td></tr>
<tr><td>Col%</td><td>6.52%</td><td>62.07%</td><td>28.00%</td></tr>
<tr><td><b>TOTAL</b></td><td>46</td><td>29</td><td>75</td></tr>
<tr><td>Row%</td><td>61.33%</td><td>38.67%</td><td>100.00%</td></tr>
<tr><td>Col%</td><td>100.00%</td><td>100.00%</td><td>100.00%</td></tr>
</table>
<h4 align="center"> Single Table Analysis </h4>
<table><tr><td class="stats"></td><td class="stats">Point</td><td class="stats" colspan=2>95% Confidence Interval</td></tr>
<tr><td class="stats"></td><td class="stats">Estimate</td><td class="stats">Lower</td><td class="stats">Upper</td></tr>
<tr><td class="stats">PARAMETERS: Odds-based</td><td class="stats"></td><td class="stats"></td><td class="stats"></td></tr>
<tr><td class="stats">Odds Ratio (cross product)</td><td class="stats">23.4545</td><td class="stats">5.8410</td><td class="stats">94.1811 (T)</td></tr>
<tr><td class="stats">Odds Ratio (MLE)</td><td class="stats">22.1490</td><td class="stats">5.9280</td><td class="stats">109.1473 (M)</td></tr>
<tr><td class="stats"></td><td class="stats"></td><td class="stats">5.2153</td><td class="stats">138.3935 (F)</td></tr>
<tr><td class="stats" colspan=4>(T=Taylor series; C=Cornfield; M=Mid-P; F=Fisher Exact)</td></tr>
<tr><td class="stats">STATISTICAL TESTS</td><td class="stats">Chi-square</td><td class="stats">1-tailed p</td><td class="stats">2-tailed p</td></tr>
<tr><td class="stats">Chi-square - uncorrected</td><td class="stats">27.2225</td><td class="stats"></td><td class="stats">0.0000001813</td></tr>
<tr><td class="stats">Fisher exact</td><td class="stats"></td><td class="stats">0.0000002597</td><td class="stats">0.0000002597</td></tr>
</table>
</body></html>`;

test('parses a 2x2 block', () => {
  const { outputs } = parseHtml(SNIPPET, labels);
  assert.equal(outputs.length, 1);
  assert.equal(outputs[0].command, 'TABLES VANILLA ILL');
  const or = outputs[0].values.find(v => v.key === 'two_by_two.odds_ratio.estimate');
  assert.ok(or, 'no value mapped to two_by_two.odds_ratio.estimate');
  assert.equal(or.label, 'Single Table Analysis: Odds Ratio (cross product) Estimate (T)');
  assert.equal(or.text, '23.4545');
  assert.equal(or.value, 23.4545);
  assert.equal(or.decimals, 4);
});

test('unmapped label keeps key null', () => {
  const { outputs } = parseHtml(SNIPPET, labels);
  const cell = outputs[0].values.find(v => v.label === 'VANILLA=1 ILL=1');
  assert.ok(cell, 'crosstab cell VANILLA=1 ILL=1 not emitted');
  assert.equal(cell.key, null);
  assert.equal(cell.value, 43);
  assert.equal(cell.decimals, 0);
});

test('oswego.html parses to the committed oswego.json', () => {
  const html = readFileSync(join(HERE, 'oswego.html'), 'utf8');
  const committed = readFileSync(join(HERE, 'oswego.json'), 'utf8');
  const { outputs } = parseHtml(html, labels);
  const rebuilt = { source: JSON.parse(committed).source, outputs };
  assert.equal(JSON.stringify(rebuilt, null, 2), committed);
});

test('every value is finite and decimals >= 0', () => {
  const html = readFileSync(join(HERE, 'oswego.html'), 'utf8');
  const { outputs } = parseHtml(html, labels);
  for (const out of outputs) {
    for (const v of out.values) {
      assert.ok(Number.isFinite(v.value), `${v.label}: non-finite value ${v.value}`);
      assert.ok(Number.isInteger(v.decimals) && v.decimals >= 0,
        `${v.label}: bad decimals ${v.decimals}`);
    }
  }
});
