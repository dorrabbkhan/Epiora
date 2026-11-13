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

// The MEANS excerpt is a byte-verbatim slice of the committed
// fixtures/ei7/oswego.html — itself byte-identical (SHA-256 1a599aa2…)
// to the ei7-output artifact of ei7.yml runs 43498896712 and
// 43500418310 (v1.0 Task 4). Everything from the #Results3 anchor to
// end of file is exactly the "MEANS AGE" and "MEANS AGE SEX" sections.
// Slicing the committed file (rather than pasting a copy) keeps the
// excerpt verbatim by construction; parses_means_blocks asserts the
// substring property anyway.
const OSWEGO_HTML = readFileSync(join(HERE, 'oswego.html'), 'utf8');
const MEANS_START = OSWEGO_HTML.indexOf('<a name="#Results3"');
const MEANS_EXCERPT = OSWEGO_HTML.slice(MEANS_START);
const MEANS_SNIPPET = `<!DOCTYPE HTML>\n<html><body>\n${MEANS_EXCERPT}</body></html>`;

test('parses_means_blocks', () => {
  assert.ok(MEANS_START >= 0, 'no #Results3 anchor in fixtures/ei7/oswego.html');
  assert.ok(OSWEGO_HTML.includes(MEANS_EXCERPT),
    'the MEANS excerpt is not a byte-verbatim substring of fixtures/ei7/oswego.html');
  const { outputs } = parseHtml(MEANS_SNIPPET, labels);
  assert.equal(outputs.length, 2);
  assert.equal(outputs[0].command, 'MEANS AGE');
  assert.equal(outputs[1].command, 'MEANS AGE SEX');

  const byLabel = (out) => new Map(out.values.map(v => [v.label, v]));
  const get = (out, label) => {
    const v = byLabel(out).get(label);
    assert.ok(v, `no value labelled ${label}`);
    return v;
  };
  const check = (out, label, text, value, decimals, key) => {
    const v = get(out, label);
    assert.equal(v.text, text, `${label}: text`);
    assert.equal(v.value, value, `${label}: value`);
    assert.equal(v.decimals, decimals, `${label}: decimals`);
    assert.equal(v.key, key, `${label}: key`);
  };

  // Ungrouped: descriptives land on means.overall.*; the "25%"/"75%"
  // cells are headers, not data.
  const overall = [
    ['Obs', '75.0000', 75, 'observations'], ['Total', '2761.0000', 2761, 'total'],
    ['Mean', '36.8133', 36.8133, 'mean'], ['Variance', '460.1809', 460.1809, 'variance'],
    ['Std Dev', '21.4518', 21.4518, 'std_dev'], ['Minimum', '3.0000', 3, 'min'],
    ['25%', '16.0000', 16, 'p25'], ['Median', '36.0000', 36, 'median'],
    ['75%', '58.0000', 58, 'p75'], ['Maximum', '77.0000', 77, 'max'],
    ['Mode', '11.0000', 11, 'mode'],
  ];
  for (const [label, text, value, field] of overall) {
    check(outputs[0], label, text, value, 4, `means.overall.${field}`);
  }

  // Grouped: Female is groups[0], Male groups[1] (v0.2 category order).
  check(outputs[1], 'Female Obs', '44.0000', 44, 4, 'means.groups[0].observations');
  check(outputs[1], 'Male Std Dev', '24.1234', 24.1234, 4, 'means.groups[1].std_dev');
  check(outputs[1], 'Female 25%', '16.5000', 16.5, 4, 'means.groups[0].p25');
  check(outputs[1], 'Female Median', '35.5000', 35.5, 4, 'means.groups[0].median');
  check(outputs[1], 'Male 75%', '62.0000', 62, 4, 'means.groups[1].p75');
  check(outputs[1], 'Male Mode', '17.0000', 17, 4, 'means.groups[1].mode');

  // T-Test table: the spanning "95% CL Mean" header covers two cells and
  // the Pooled/Satterthwaite rows repeat "Diff (Group 1 - Group 2)", so
  // later duplicates carry " (n)" suffixes in emission order.
  const tt = 'T-Test: Diff (Group 1 - Group 2)';
  check(outputs[1], `${tt} Mean`, '-0.0433', -0.0433, 4, 'means.t_test.pooled.diff');
  check(outputs[1], `${tt} 95% CL Mean`, '-10.1369', -10.1369, 4, 'means.t_test.pooled.diff_ci_95.lower');
  check(outputs[1], `${tt} 95% CL Mean (2)`, '10.0504', 10.0504, 4, 'means.t_test.pooled.diff_ci_95.upper');
  check(outputs[1], `${tt} Std Dev`, '21.5982', 21.5982, 4, null);
  check(outputs[1], `${tt} Mean (2)`, '-0.0433', -0.0433, 4, 'means.t_test.welch.diff');
  check(outputs[1], `${tt} 95% CL Mean (3)`, '-10.5528', -10.5528, 4, 'means.t_test.welch.diff_ci_95.lower');
  check(outputs[1], `${tt} 95% CL Mean (4)`, '10.4663', 10.4663, 4, 'means.t_test.welch.diff_ci_95.upper');
  check(outputs[1], 'T-Test: Pooled DF', '73', 73, 0, 'means.t_test.pooled.df');
  check(outputs[1], 'T-Test: Pooled t Value', '-0.01', -0.01, 2, 'means.t_test.pooled.t');
  check(outputs[1], 'T-Test: Pooled Pr > |t|', '0.9932', 0.9932, 4, 'means.t_test.pooled.p_value');
  check(outputs[1], 'T-Test: Satterthwaite DF', '56.05', 56.05, 2, 'means.t_test.welch.df');
  check(outputs[1], 'T-Test: Satterthwaite t Value', '-0.01', -0.01, 2, 'means.t_test.welch.t');
  check(outputs[1], 'T-Test: Satterthwaite Pr > |t|', '0.9935', 0.9935, 4, 'means.t_test.welch.p_value');

  // ANOVA; the p-value is a "P-value = …" paragraph, not a table cell.
  const anova = 'ANOVA, a Parametric Test for Inequality of Population Means';
  check(outputs[1], `${anova}: Between SS`, '0.03403', 0.03403, 5, 'means.anova.ss_between');
  check(outputs[1], `${anova}: Between df`, '1', 1, 0, 'means.anova.df_between');
  check(outputs[1], `${anova}: Between MS`, '0.03403', 0.03403, 5, 'means.anova.ms_between');
  check(outputs[1], `${anova}: Between F statistic`, '0.00007', 0.00007, 5, 'means.anova.f');
  check(outputs[1], `${anova}: Within SS`, '34053.35264', 34053.35264, 5, 'means.anova.ss_within');
  check(outputs[1], `${anova}: Within df`, '73', 73, 0, 'means.anova.df_within');
  check(outputs[1], `${anova}: Within MS`, '466.48428', 466.48428, 5, 'means.anova.ms_within');
  check(outputs[1], `${anova}: Total SS`, '34053.38667', 34053.38667, 5, 'means.anova.ss_total');
  check(outputs[1], `${anova}: Total df`, '74', 74, 0, 'means.anova.df_total');
  check(outputs[1], `${anova}: P-value`, '0.99321', 0.99321, 5, 'means.anova.p_value');

  // Bartlett and Kruskal–Wallis rows use "name =" / "name=value" cells;
  // the prose note <P> between them must not enter any label.
  const bart = "Bartlett's Test for Inequality of Population Variances";
  check(outputs[1], `${bart}: Bartlett's chi square`, '1.49534', 1.49534, 5, 'means.bartlett.statistic');
  check(outputs[1], `${bart}: df`, '1', 1, 0, 'means.bartlett.df');
  check(outputs[1], `${bart}: P value`, '0.22139', 0.22139, 5, 'means.bartlett.p_value');
  const kw = 'Mann-Whitney/Wilcoxon Two-Sample Test (Kruskal-Wallis test for two groups)';
  check(outputs[1], `${kw}: Kruskal-Wallis H (equivalent to Chi square)`, '0.0005', 0.0005, 4, 'means.kruskal_wallis.h');
  check(outputs[1], `${kw}: Degrees of freedom`, '1', 1, 0, 'means.kruskal_wallis.df');
  check(outputs[1], `${kw}: P value`, '0.9828', 0.9828, 4, 'means.kruskal_wallis.p_value');

  // Nothing unmapped besides the pooled Std Dev line; no "colN" or
  // caption-leaking labels.
  for (const out of outputs) {
    for (const v of out.values) {
      assert.ok(!/\bcol\d+$/.test(v.label), `fallback label ${v.label}`);
      assert.ok(!v.label.includes('='), `label leaks a caption: ${v.label}`);
      assert.ok(!v.label.includes('p-value'), `note text in label: ${v.label}`);
    }
  }
  const unmapped = outputs[1].values.filter(v => v.key === null).map(v => v.label);
  assert.deepEqual(unmapped, [`${tt} Std Dev`]);
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
