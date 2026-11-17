// Calculator e2e (v1.0 plan Task 14 Step 1, spec §7.1–7.2): the index
// lists every registry calculator, a complete link computes on load,
// a malformed link marks inputs instead of blanking, calculate
// writes the inputs to the URL, random_numbers seeds itself into the
// link, the store-less worker never touches the `epiora-store` Web
// Lock, and calculator tabs coexist with the app's single-store lock.
import { expect, test, type Page } from '@playwright/test';

const OSWEGO = 'strata=10,11;4,20&conf_level=95';

/** Collects page errors + console errors; calc routes must never
 *  produce either, malformed links included. */
function consoleErrors(page: Page): string[] {
  const errors: string[] = [];
  page.on('pageerror', (e) => errors.push(`pageerror: ${e.message}`));
  page.on('console', (m) => {
    if (m.type() === 'error') errors.push(`console: ${m.text()}`);
  });
  return errors;
}

test('index_lists_every_registry_calculator', async ({ page }) => {
  const errors = consoleErrors(page);
  await page.goto('/calc');
  await expect(
    page.getByRole('heading', { name: 'Calculators' }),
  ).toBeVisible();
  const links = page
    .getByRole('list')
    .getByRole('link');
  await expect(links).toHaveCount(24);
  await expect(
    page.getByRole('link', { name: 'Two by two table' }),
  ).toBeVisible();
  await expect(
    page.getByRole('link', { name: 'Random numbers' }),
  ).toBeVisible();
  for (const family of [
    'Counts',
    'Person time',
    'Continuous (summary data)',
    'Sample size',
    'Power',
    'Other',
  ]) {
    await expect(
      page.getByRole('heading', { name: family, exact: true }),
    ).toBeVisible();
  }
  expect(errors).toEqual([]);
});

test('link_with_inputs_computes_on_load', async ({ page }) => {
  const errors = consoleErrors(page);
  await page.goto(`/calc/two_by_two?${OSWEGO}`);
  const row = page.getByRole('row', { name: /Odds ratio — estimate/ }).first();
  await expect(row).toContainText('4.545');
  await expect(
    page.getByRole('heading', { name: 'Results' }),
  ).toBeVisible();
  expect(errors).toEqual([]);
});

test('malformed_link_marks_inputs', async ({ page }) => {
  const errors = consoleErrors(page);
  await page.goto('/calc/proportion?numerator=abc&denominator=-3&zz=1');
  // Both bad fields keep their text and are marked with their typed
  // message; the unknown key is named; no result renders.
  await expect(page.getByLabel('Numerator')).toHaveValue('abc');
  await expect(page.getByLabel('Denominator')).toHaveValue('-3');
  await expect(
    page.getByText('Enter a whole number from 0 up to 2^53.'),
  ).toHaveCount(2);
  await expect(
    page.getByText('Unknown link parameters ignored: zz.'),
  ).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Results' }),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('missing_table_row_marks_table', async ({ page }) => {
  const errors = consoleErrors(page);
  await page.goto('/calc/two_by_two?strata=10,11;;4,20');
  await expect(page.getByText('The table shape is wrong.')).toBeVisible();
  await expect(
    page.getByRole('heading', { name: 'Results' }),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
});

test('calculate_updates_url', async ({ page }) => {
  const errors = consoleErrors(page);
  await page.goto('/calc/proportion');
  await page.getByLabel('Numerator').fill('12');
  await page.getByLabel('Denominator').fill('40');
  await page.getByRole('button', { name: 'Calculate' }).click();
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  expect(page.url()).toContain('numerator=12');
  expect(page.url()).toContain('denominator=40');
  expect(errors).toEqual([]);
});

test('random_numbers_seed_written_to_url', async ({ page }) => {
  const errors = consoleErrors(page);
  await page.goto('/calc/random_numbers?lower=1&upper=100&count=5');
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  await expect(page).toHaveURL(/seed=\d+/);
  // Five drawn values render as a list.
  await expect(
    page.getByRole('table', { name: 'Values' }).getByRole('listitem'),
  ).toHaveCount(5);
  expect(errors).toEqual([]);
});

test('no_database_or_lock_on_calc', async ({ page }) => {
  await page.goto(`/calc/two_by_two?${OSWEGO}`);
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  const locks = await page.evaluate(async () => {
    const snapshot = await navigator.locks.query();
    return (snapshot.held ?? []).map((l) => l?.name ?? '');
  });
  expect(locks.filter((n) => n.includes('epiora'))).toEqual([]);
});

test('calculator_tabs_coexist_with_app_lock', async ({ context }) => {
  const app = await context.newPage();
  await app.goto('/');
  // The projects list needs the store, so seeing it means this tab's
  // worker holds the epiora-store lock.
  await expect(
    app.getByRole('heading', { name: 'Projects' }),
  ).toBeVisible();

  const calc1 = await context.newPage();
  await calc1.goto(`/calc/two_by_two?${OSWEGO}`);
  await expect(
    calc1.getByRole('heading', { name: 'Results' }),
  ).toBeVisible();

  const calc2 = await context.newPage();
  await calc2.goto('/calc/proportion?numerator=3&denominator=7');
  await expect(
    calc2.getByRole('heading', { name: 'Results' }),
  ).toBeVisible();

  // A second app tab still hits the single-store lock, while the
  // calculator tabs keep working.
  const app2 = await context.newPage();
  await app2.goto('/');
  await expect(
    app2.getByText('Epiora is open in another tab'),
  ).toBeVisible();
  await expect(
    calc1.getByRole('row', { name: /Odds ratio — estimate/ }).first(),
  ).toContainText('4.545');
  await expect(
    calc2.getByRole('heading', { name: 'Results' }),
  ).toBeVisible();
  await app.close();
  await app2.close();
  await calc1.close();
  await calc2.close();
});

test('works_at_320px', async ({ page }) => {
  await page.setViewportSize({ width: 320, height: 640 });
  await page.goto(`/calc/two_by_two?${OSWEGO}`);
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  const scrollWidth = await page.evaluate(
    () => document.documentElement.scrollWidth,
  );
  expect(scrollWidth).toBeLessThanOrEqual(320);
});
