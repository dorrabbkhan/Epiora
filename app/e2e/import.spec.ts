// Import wizard e2e (v0.3 plan Task 11): the tests drive the real
// wizard UI; projects/datasets are seeded through `window.dataApi`
// like dataset.spec.ts. Every wizard step gets an axe pass.
import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';

const FIXTURES = new URL('../../fixtures/import/', import.meta.url);
const UTF8_COMMA = readFileSync(new URL('utf8_comma/input.csv', FIXTURES), 'utf8');
const OVERRIDE = readFileSync(
  new URL('override_coerces_to_missing/input.csv', FIXTURES),
  'utf8',
);
const SEMICOLON = readFileSync(
  new URL('semicolon_decimal_comma/input.csv', FIXTURES),
  'utf8',
);
const RAGGED = readFileSync(new URL('ragged_row_error/input.csv', FIXTURES), 'utf8');
const XLSX_TWO_SHEETS = readFileSync(new URL('xlsx_two_sheets/input.xlsx', FIXTURES));

async function createProject(page: Page): Promise<string> {
  return page.evaluate(async () => {
    return (await window.dataApi.createProject('Import test')).id;
  });
}

/** Imports `csv` into `projectId` through the real RPC (dataset v1). */
async function seedCsv(page: Page, projectId: string, csv: string): Promise<string> {
  return page.evaluate(
    async ({ projectId, csv }) => {
      const api = window.dataApi;
      const bytes = new TextEncoder().encode(csv);
      const { sessionId, inspection } = await api.inspectFile(
        'input.csv',
        bytes.buffer as ArrayBuffer,
      );
      const result = await api.commitImport(
        sessionId,
        { projectId, datasetName: 'input.csv' },
        inspection.settings,
        false,
        () => {},
      );
      return result.datasetId;
    },
    { projectId, csv },
  );
}

/** Clicks the wizard's Choose file button and hands it `data`. */
async function chooseFile(
  page: Page,
  name: string,
  mimeType: string,
  data: string | Buffer,
): Promise<void> {
  const chooser = page.waitForEvent('filechooser');
  await page.getByRole('button', { name: 'Choose file' }).click();
  await (await chooser).setFiles({
    name,
    mimeType,
    buffer: typeof data === 'string' ? Buffer.from(data, 'utf8') : data,
  });
}

/** preview → settings → confirm for a new-dataset import. */
async function walkNewImportToConfirm(page: Page): Promise<void> {
  await page.getByRole('button', { name: 'Next' }).click();
  await expect(
    page.getByRole('button', { name: 'Record key' }),
  ).toBeVisible();
  await expectNoA11yViolations(page, 'wizard settings step');
  await page.getByRole('button', { name: 'Next' }).click();
  await expect(
    page.getByRole('heading', { name: 'Confirm import' }),
  ).toBeVisible();
}

test('import_csv_end_to_end', async ({ page }) => {
  await page.goto('/');
  const projectId = await createProject(page);
  await page.goto(`/p/${projectId}/import`);

  await expect(
    page.getByRole('heading', { name: 'Import data' }),
  ).toBeVisible();
  await expectNoA11yViolations(page, 'wizard choose step');
  await chooseFile(page, 'input.csv', 'text/csv', UTF8_COMMA);

  // Preview: the columns and the detected format are shown.
  await expect(page.getByRole('columnheader', { name: /AGE/ })).toBeVisible();
  await expect(page.getByRole('gridcell', { name: 'alice' })).toBeVisible();
  await expectNoA11yViolations(page, 'wizard preview step');

  await walkNewImportToConfirm(page);
  await expectNoA11yViolations(page, 'wizard confirm step');
  await page.getByRole('button', { name: 'Import', exact: true }).click();

  await expect(
    page.getByRole('heading', { name: 'Import complete' }),
  ).toBeVisible();
  await expect(page.getByText('Dataset version 1 created.')).toBeVisible();
  await expectNoA11yViolations(page, 'wizard done step');

  await page.goto(`/p/${projectId}`);
  const row = page.getByRole('row', { name: /input\.csv/ });
  await expect(row).toContainText('v1');
});

test('override_type_shows_coerced_count', async ({ page }) => {
  await page.goto('/');
  const projectId = await createProject(page);
  await page.goto(`/p/${projectId}/import`);
  await chooseFile(page, 'input.csv', 'text/csv', OVERRIDE);

  // AGE infers as text; overriding to integer coerces "abc".
  await expect(page.getByRole('columnheader', { name: /AGE/ })).toBeVisible();
  await page.getByRole('button', { name: 'Type for AGE' }).click();
  await page.getByRole('option', { name: 'integer' }).click();
  await expect(page.getByText('1 value set to missing')).toBeVisible();
  await expectNoA11yViolations(page, 'preview after type override');
});

test('semicolon_file_detects_decimal_comma', async ({ page }) => {
  await page.goto('/');
  const projectId = await createProject(page);
  await page.goto(`/p/${projectId}/import`);
  await chooseFile(page, 'input.csv', 'text/csv', SEMICOLON);

  await expect(page.getByText('Delimiter: ;')).toBeVisible();
  await expect(page.getByText('Decimal separator: comma')).toBeVisible();
  await expectNoA11yViolations(page, 'preview with decimal comma');
});

test('xlsx_sheet_choice', async ({ page }) => {
  await page.goto('/');
  const projectId = await createProject(page);
  await page.goto(`/p/${projectId}/import`);
  await chooseFile(page, 'input.xlsx', 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet', XLSX_TWO_SHEETS);

  // Default sheet "First" shows its only column.
  await expect(page.getByRole('columnheader', { name: /X/ })).toBeVisible();

  await page.getByRole('button', { name: 'Next' }).click();
  await page.getByRole('button', { name: 'Sheet' }).click();
  await page.getByRole('option', { name: 'Second' }).click();

  // Back on preview the second sheet's columns and cells show.
  await page.getByRole('button', { name: 'Back' }).click();
  const rowsGrid = page.getByRole('grid', { name: 'First 100 rows' });
  await expect(
    rowsGrid.getByRole('columnheader', { name: 'A', exact: true }),
  ).toBeVisible();
  await expect(
    rowsGrid.getByRole('gridcell', { name: 'p', exact: true }),
  ).toBeVisible();
  await expectNoA11yViolations(page, 'preview of second sheet');
});

test('reimport_changed_file_shows_diff_and_creates_v2', async ({ page }) => {
  await page.goto('/');
  const projectId = await createProject(page);
  await seedCsv(page, projectId, 'ID,V\na,1\nb,2\n');
  await page.goto(`/p/${projectId}/import`);

  // RAC radios hide the input; the label text is the click target.
  await page.getByText('Update input.csv (v1)', { exact: true }).click();
  await chooseFile(page, 'changed.csv', 'text/csv', 'ID,V\na,1\nb,3\nc,4\n');

  await page.getByRole('button', { name: 'Next' }).click();
  await page.getByRole('button', { name: 'Next' }).click();

  // The diff step shows added/changed/removed counts.
  await expect(
    page.getByRole('heading', { name: 'Changes', exact: true }),
  ).toBeVisible();
  await expect(page.getByText('1 added')).toBeVisible();
  await expect(page.getByText('1 changed')).toBeVisible();
  await expect(page.getByText('0 removed')).toBeVisible();
  await expectNoA11yViolations(page, 'wizard diff step');

  await page.getByRole('button', { name: 'Next' }).click();
  await page.getByRole('button', { name: 'Import', exact: true }).click();
  await expect(page.getByText('Dataset version 2 created.')).toBeVisible();

  await page.goto(`/p/${projectId}`);
  await expect(page.getByRole('row', { name: /input\.csv/ })).toContainText('v2');
});

test('cancel_during_write_leaves_no_dataset', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  const projectId = await createProject(page);
  await page.goto(`/p/${projectId}/import`);

  // >10,000 rows so the write stage crosses at least one batch
  // boundary where a cancel can land (Task 9 batches at 10,000).
  const csv = [
    'ID,V',
    ...Array.from({ length: 30_000 }, (_, i) => `id${i},${i}`),
  ].join('\n');
  await chooseFile(page, 'big.csv', 'text/csv', csv);
  await walkNewImportToConfirm(page);
  await page.getByRole('button', { name: 'Import', exact: true }).click();

  await expect(
    page.getByRole('progressbar', { name: 'Write records' }),
  ).toBeVisible();
  await expectNoA11yViolations(page, 'wizard progress step');
  await page.getByRole('button', { name: 'Cancel import' }).click();
  await expect(page.getByRole('alert')).toContainText(/cancelled/i);

  await expect(
    page.evaluate(
      (pid) => window.dataApi.listDatasets(pid).then((d) => d.length),
      projectId,
    ),
  ).resolves.toBe(0);
});

test('ragged_row_error_names_line', async ({ page }) => {
  await page.goto('/');
  const projectId = await createProject(page);
  await page.goto(`/p/${projectId}/import`);
  await chooseFile(page, 'input.csv', 'text/csv', RAGGED);

  await expect(
    page.getByRole('heading', { name: 'Import failed' }),
  ).toBeVisible();
  await expect(page.getByRole('alert')).toContainText('line 4');
  await expectNoA11yViolations(page, 'wizard error step');
});

test('import shows quota error', async ({ page }) => {
  await page.goto('/');
  const projectId = await createProject(page);
  await page.goto(`/p/${projectId}/import`);

  // Review Focus 3: the Task 8 store hook makes the next write fail
  // with quota_exceeded; the wizard surfaces it and nothing commits.
  await page.evaluate(() => window.dataApi.__simulateQuotaOnNextWrite());
  await chooseFile(page, 'input.csv', 'text/csv', UTF8_COMMA);
  await walkNewImportToConfirm(page);
  await page.getByRole('button', { name: 'Import', exact: true }).click();

  await expect(
    page.getByRole('heading', { name: 'Import failed' }),
  ).toBeVisible();
  await expect(page.getByRole('alert')).toContainText('not enough storage space');
  await expect(
    page.evaluate(
      (pid) => window.dataApi.listDatasets(pid).then((d) => d.length),
      projectId,
    ),
  ).resolves.toBe(0);
});
