// Seeding helpers for the legacy import routes, shared by the legacy,
// accessibility and viewport specs.

import { fileURLToPath } from 'node:url';
import { expect, type Page } from '@playwright/test';

export const SAMPLE_DB = fileURLToPath(
  new URL('../../../fixtures/legacy/samples/ei7/Sample/SampleDB.db', import.meta.url),
);
export const SAMPLE_PRJ = fileURLToPath(
  new URL('../../../fixtures/legacy/samples/ei7/Sample/Sample.prj', import.meta.url),
);
export const REIMPORT_DB = fileURLToPath(
  new URL('../../../fixtures/legacy/golden/reimport/SampleDB.db', import.meta.url),
);
export const PLAIN_MDB = fileURLToPath(
  new URL('../../../fixtures/legacy/samples/ei3/HospitalPoints.mdb', import.meta.url),
);
export const MISMATCH_DB = fileURLToPath(
  new URL('../../../fixtures/legacy/samples/ei7/BeefPrevention/BeefPrevention.db', import.meta.url),
);

export async function chooseFiles(page: Page, paths: string[]): Promise<void> {
  const chooser = page.waitForEvent('filechooser');
  await page.getByRole('button', { name: 'Choose project files' }).click();
  await (await chooser).setFiles(paths);
}

export async function walkToPreview(page: Page, paths = [SAMPLE_PRJ, SAMPLE_DB]): Promise<void> {
  await page.goto('/legacy');
  await chooseFiles(page, paths);
  await expect(page.getByRole('heading', { name: /Preview SQLiteSample2/ })).toBeVisible();
}

export async function importSample(page: Page): Promise<string> {
  await walkToPreview(page);
  await page.getByRole('button', { name: 'Confirm import' }).click();
  await expect(page).toHaveURL(/\/p\/[^/]+$/, { timeout: 60_000 });
  await expect(page.getByRole('heading', { name: 'SQLiteSample2' })).toBeVisible();
  return page.url().split('/p/')[1]!;
}

export async function chooseReimportFile(page: Page, path: string): Promise<void> {
  const chooser = page.waitForEvent('filechooser');
  await page.getByRole('button', { name: 'Choose newer copy' }).click();
  await (await chooser).setFiles(path);
}
