// Explain mode e2e (v1.0 plan Task 16 Step 1, spec §6): the panel on
// an Analysis result row (Oswego TABLES odds ratio) shows words then
// "Show the maths" MathML with the run's own numbers; a calculator
// row explains identically; a method-kind trace (the Fisher exact
// CI) shows the method words and the inputs/result table only; and
// every number the panel renders is a trace number, formatted.
import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';
import type { Explanation } from '@engine-types/Explanation';
import { formatNumber } from '../src/ui/format';

const OSWEGO = readFileSync(
  new URL('../../fixtures/data/oswego.csv', import.meta.url),
  'utf8',
);
const OSWEGO_VANILLA = JSON.parse(
  readFileSync(
    new URL('../../fixtures/2x2/oswego_vanilla.json', import.meta.url),
    'utf8',
  ),
) as { expected: { odds_ratio: { estimate: { value: number } } } };

interface Seed {
  projectId: string;
  datasetId: string;
}

/** Imports the Oswego CSV into a fresh project through the real RPC. */
async function seedOswego(page: Page): Promise<Seed> {
  return page.evaluate(async (csv) => {
    const api = window.dataApi;
    const project = await api.createProject('Oswego');
    const bytes = new TextEncoder().encode(csv);
    const { sessionId, inspection } = await api.inspectFile(
      'oswego.csv',
      bytes.buffer as ArrayBuffer,
    );
    const result = await api.commitImport(
      sessionId,
      { projectId: project.id, datasetName: 'oswego.csv' },
      inspection.settings,
      false,
      () => {},
    );
    return { projectId: project.id, datasetId: result.datasetId };
  }, OSWEGO);
}

async function pick(page: Page, label: RegExp | string, option: string): Promise<void> {
  await page.getByRole('button', { name: label }).focus();
  await page.keyboard.press('Enter');
  const item = page.getByRole('option', { name: option, exact: true });
  await expect(item).toBeVisible();
  await item.click();
}

/** Seeds Oswego and runs TABLES VANILLA × ILL; returns the run id. */
async function oswegoTables(page: Page): Promise<string> {
  await page.goto('/');
  const seed = await seedOswego(page);
  await page.goto(`/p/${seed.projectId}/analysis`);
  await page.getByRole('button', { name: 'Run procedure' }).click();
  await page.getByRole('menuitem', { name: 'TABLES', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'TABLES', exact: true });
  await expect(dialog).toBeVisible();
  await pick(page, /Exposure variable/, 'VANILLA');
  await pick(page, /Outcome variable/, 'ILL');
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'TABLES VANILLA × ILL — oswego.csv v1' }),
  ).toBeVisible();
  const runId = await page.evaluate(async (projectId) => {
    const runs = await window.dataApi.listRuns(projectId);
    return runs[0]?.id ?? '';
  }, seed.projectId);
  expect(runId).not.toBe('');
  return runId;
}

test('explain_odds_ratio_shows_words_and_maths', async ({ page }) => {
  await oswegoTables(page);
  const card = page.getByRole('article', {
    name: 'TABLES VANILLA × ILL — oswego.csv v1',
  });
  const row = card
    .getByRole('row')
    .filter({ has: page.getByRole('rowheader', { name: 'Odds ratio', exact: true }) });
  await row.getByRole('button', { name: 'Explain' }).click();

  const dialog = page.getByRole('dialog', { name: /Explain — Odds ratio/ });
  await expect(dialog).toBeVisible();
  // Pending-adviser-review notice under every explanation.
  await expect(
    dialog.getByText('Explain text is pending adviser review.'),
  ).toBeVisible();
  // Words first: the plain-language paragraph names the statistic.
  const words = dialog.getByRole('region', { name: 'How it is computed' });
  await expect(words).toBeVisible();
  await expect(words).toContainText(/odds ratio/i);
  // "Show the maths" reveals MathML carrying the run's numbers.
  await dialog.getByRole('button', { name: 'Show the maths' }).click();
  const math = dialog.locator('math');
  await expect(math).toHaveCount(1);
  await expect(math).toHaveAttribute('aria-label', /.+/);
  await expect(math).toContainText(
    formatNumber(OSWEGO_VANILLA.expected.odds_ratio.estimate.value),
  );
});

test('explain_calculator_row', async ({ page }) => {
  await page.goto('/calc/screening?table=80,40;20,360&conf_level=95');
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  const sensitivity = page.getByRole('table', { name: 'Sensitivity' });
  const row = sensitivity
    .getByRole('row')
    .filter({ has: page.getByRole('rowheader', { name: 'Estimate', exact: true }) });
  await row.getByRole('button', { name: 'Explain' }).click();

  const dialog = page.getByRole('dialog', { name: /Explain/ });
  await expect(dialog).toBeVisible();
  await expect(
    dialog.getByText('Explain text is pending adviser review.'),
  ).toBeVisible();
  await expect(
    dialog.getByRole('region', { name: 'How it is computed' }),
  ).toContainText(/proportion|sensitivity/i);
  await dialog.getByRole('button', { name: 'Show the maths' }).click();
  await expect(dialog.locator('math')).toHaveCount(1);
});

test('iterative_shows_method_only', async ({ page }) => {
  await oswegoTables(page);
  const card = page.getByRole('article', {
    name: 'TABLES VANILLA × ILL — oswego.csv v1',
  });
  const row = card.getByRole('row').filter({
    has: page.getByRole('rowheader', {
      name: 'Odds ratio — Fisher exact 95% CI',
      exact: true,
    }),
  });
  await row.getByRole('button', { name: 'Explain' }).click();

  const dialog = page.getByRole('dialog', { name: /Explain/ });
  await expect(dialog).toBeVisible();
  // The method is described in words; there is no closed-form maths.
  const words = dialog.getByRole('region', { name: 'How it is computed' });
  await expect(words).toContainText(/Fisher|exact|conditional/i);
  await expect(
    dialog.getByRole('button', { name: 'Show the maths' }),
  ).toHaveCount(0);
  await expect(dialog.locator('math')).toHaveCount(0);
  // The inputs and the result render as the step table.
  await expect(dialog.getByRole('table').first()).toBeVisible();
});

test('every_number_in_panel_is_from_trace', async ({ page }) => {
  const runId = await oswegoTables(page);
  const statistic = 'two_by_two.odds_ratio.estimate';
  const trace = await page.evaluate(
    async ({ runId, statistic }): Promise<Explanation> =>
      window.dataApi.explainRun(runId, statistic),
    { runId, statistic },
  );
  expect(trace.kind).toBe('closed_form');

  const card = page.getByRole('article', {
    name: 'TABLES VANILLA × ILL — oswego.csv v1',
  });
  const row = card
    .getByRole('row')
    .filter({ has: page.getByRole('rowheader', { name: 'Odds ratio', exact: true }) });
  await row.getByRole('button', { name: 'Explain' }).click();
  const dialog = page.getByRole('dialog', { name: /Explain/ });
  await expect(dialog).toBeVisible();
  await dialog.getByRole('button', { name: 'Show the maths' }).click();

  const allowed = new Set(
    trace.steps.flatMap((step: Explanation['steps'][number]) => {
      const nums = Object.values(step.values).map((v) => formatNumber(v));
      if (typeof step.result === 'number') nums.push(formatNumber(step.result));
      return nums;
    }),
  );
  // Firefox concatenates adjacent <mn> texts without a separator, so
  // MathML numbers are collected one <mn> at a time and the rest of
  // the panel text is read with the math subtrees removed.
  const body = await dialog.evaluate((el) => {
    const clone = el.cloneNode(true) as HTMLElement;
    for (const m of Array.from(clone.querySelectorAll('math'))) m.remove();
    return clone.textContent ?? '';
  });
  const mathNums = await dialog.locator('math').locator('mn').allTextContents();
  const rendered = [
    ...(body.match(/\d+(?:\.\d+)?(?:e[+-]?\d+)?/g) ?? []),
    ...mathNums.flatMap(
      (text) => text.match(/\d+(?:\.\d+)?(?:e[+-]?\d+)?/g) ?? [],
    ),
  ];
  expect(rendered.length).toBeGreaterThan(0);
  for (const n of rendered) {
    expect(allowed, `panel shows untraced number ${n}`).toContain(n);
  }
});
