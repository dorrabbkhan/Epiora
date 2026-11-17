// axe on every route, light and dark (spec §9.3, Task 7 Step 2).
import { test } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';
import { ROUTES } from './routes';

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
