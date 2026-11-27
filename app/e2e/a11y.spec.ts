// axe on every route, light and dark (spec §9.3, Task 7 Step 2).
// v1.0 Task 24: `/validation` is in ROUTES, so the loop below covers
// the dossier in every project (three engines) and both schemes.
import { expect, test } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';
import { ROUTES } from './routes';
import { walkToPreview } from './helpers/legacy';

for (const colorScheme of ['light', 'dark'] as const) {
  for (const route of ROUTES) {
    test(`${route} in ${colorScheme} has no axe violations`, async ({
      page,
    }) => {
      await page.emulateMedia({ colorScheme });
      await page.goto(route);
      await expectNoA11yViolations(page, `${route} (${colorScheme})`);
    });
  }
}


// Legacy preview and re-import are states rather than directly seeded
// URLs: cover them in the same light/dark matrix with one seeded project.
for (const colorScheme of ['light', 'dark'] as const) {
  test(`legacy preview and reimport in ${colorScheme} have no axe violations`, async ({ page }) => {
    test.setTimeout(120_000);
    await page.emulateMedia({ colorScheme });
    await walkToPreview(page);
    await expectNoA11yViolations(page, `legacy preview (${colorScheme})`);
    await page.getByRole('button', { name: 'Confirm import' }).click();
    await expect(page).toHaveURL(/\/p\/[^/]+$/, { timeout: 60_000 });
    const projectId = page.url().split('/p/')[1]!;
    await page.goto(`/p/${projectId}/reimport`);
    await expect(page.getByRole('heading', { name: 'Re-import a newer copy' })).toBeVisible();
    await expectNoA11yViolations(page, `legacy reimport (${colorScheme})`);
  });
}

// v1.0 Task 14: one calculator per family — counts, person_time,
// continuous, sample_size, power, other — plus a rendered result.
const CALC_ROUTES = [
  '/calc/matched_case_control',
  '/calc/rate_compare',
  '/calc/mean_ci',
  '/calc/ss_proportion',
  '/calc/power_cohort',
  '/calc/random_numbers',
  '/calc/two_by_two?strata=10,11;4,20&conf_level=95',
  // v1.0 Task 23: a result carrying an assumption note (sparse 2×2 →
  // expected_count_below_5 under the Crude table).
  '/calc/two_by_two?strata=1,2;3,8&conf_level=95',
] as const;

for (const colorScheme of ['light', 'dark'] as const) {
  for (const route of CALC_ROUTES) {
    test(`${route} in ${colorScheme} has no axe violations`, async ({
      page,
    }) => {
      await page.emulateMedia({ colorScheme });
      await page.goto(route);
      await expectNoA11yViolations(page, `${route} (${colorScheme})`);
    });
  }
}

// v1.0 Task 16: the open explain panel — words, expanded "Show the
// maths" disclosure and the steps table — gets the axe pass in every
// engine (the spec's projects) and both schemes, and the formula is a
// `<math>` element with an accessible label.
for (const colorScheme of ['light', 'dark'] as const) {
  test(`open explain panel in ${colorScheme} has no axe violations`, async ({
    page,
  }) => {
    await page.emulateMedia({ colorScheme });
    await page.goto('/calc/two_by_two?strata=10,11;4,20&conf_level=95');
    await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
    const row = page.getByRole('row').filter({
      has: page.getByRole('rowheader', {
        name: 'Odds ratio — estimate',
        exact: true,
      }),
    }).first();
    await row.getByRole('button', { name: 'Explain' }).click();
    const dialog = page.getByRole('dialog', { name: /Explain/ });
    await expect(dialog).toBeVisible();
    await dialog.getByRole('button', { name: 'Show the maths' }).click();
    const math = dialog.locator('math');
    await expect(math).toHaveCount(1);
    await expect(math).toHaveAttribute('aria-label', /.+/);
    await expectNoA11yViolations(
      page,
      `open explain panel (${colorScheme})`,
    );
  });
}
