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
