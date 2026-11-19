// "Export to R" e2e (v1.0 Task 17, spec §8.2): the button downloads a
// zip built in the worker — analysis.R plus data.csv for dataset runs,
// receipt.json always. The zip is unpacked here with the same fflate
// the worker uses and its entries asserted.

import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';
import { strFromU8, unzipSync } from 'fflate';

const OSWEGO = readFileSync(
  new URL('../../fixtures/data/oswego.csv', import.meta.url),
  'utf8',
);

/** Imports Oswego and runs TABLES VANILLA × ILL through the real RPC. */
async function seedTablesRun(page: Page): Promise<void> {
  const projectId = await page.evaluate(async (csv) => {
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
    await api.runTables(project.id, result.datasetId, 1, {
      exposure: { name: 'VANILLA', levels: null },
      outcome: { name: 'ILL', levels: null },
      stratify_by: [],
      include_missing: false,
    });
    return project.id;
  }, OSWEGO);
  await page.goto(`/p/${projectId}/analysis`);
}

test('export_zip_contains_three_files', async ({ page }) => {
  await page.goto('/');
  await seedTablesRun(page);
  const card = page.getByRole('article', {
    name: 'TABLES VANILLA × ILL — oswego.csv v1',
  });
  await expect(card).toBeVisible();

  const [download] = await Promise.all([
    page.waitForEvent('download'),
    card.getByRole('button', { name: 'Export to R' }).click(),
  ]);
  expect(download.suggestedFilename()).toMatch(
    /^epiora-tables-[0-9a-f]{12}\.zip$/,
  );
  const zip = unzipSync(
    new Uint8Array(readFileSync(await download.path())),
  );
  expect(Object.keys(zip).sort()).toEqual([
    'analysis.R',
    'data.csv',
    'receipt.json',
  ]);
  const script = strFromU8(zip['analysis.R']!);
  expect(script).toContain('# R code: MIT licence');
  expect(script).toContain('twobytwo_write(');
  const receipt = JSON.parse(strFromU8(zip['receipt.json']!)) as {
    procedure: string;
  };
  expect(receipt.procedure).toBe('tables');
  const csv = strFromU8(zip['data.csv']!);
  expect(csv.startsWith('Name,AGE,SEX,ILL,')).toBe(true);
});

test('calculator_export_has_no_csv', async ({ page }) => {
  await page.goto('/calc/proportion?numerator=12&denominator=40');
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();

  const [download] = await Promise.all([
    page.waitForEvent('download'),
    page.getByRole('button', { name: 'Export to R' }).click(),
  ]);
  expect(download.suggestedFilename()).toMatch(
    /^epiora-proportion-[0-9a-f]{12}\.zip$/,
  );
  const zip = unzipSync(
    new Uint8Array(readFileSync(await download.path())),
  );
  expect(Object.keys(zip).sort()).toEqual(['analysis.R', 'receipt.json']);
  const script = strFromU8(zip['analysis.R']!);
  expect(script).toContain('# R code: MIT licence');
  expect(script).toContain('proportion_write(');
});
