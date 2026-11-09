// Settings screen e2e (v0.3 plan Task 10): the theme choice is stored
// under the `epiora.theme` localStorage key and survives a reload; the
// screen shows the engine version and the storage estimate.
import { expect, test } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';

test('theme_switch_persists_across_reload', async ({ page }) => {
  await page.goto('/settings');
  // RAC radios hide the input; the label text is the click target.
  await page.getByText('Dark', { exact: true }).click();
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem('epiora.theme')))
    .toBe('dark');

  await page.reload();
  await expect(page.locator('html')).toHaveAttribute('data-theme', 'dark');
  await expect(
    page.getByRole('radio', { name: 'Dark', exact: true }),
  ).toBeChecked();
  await expectNoA11yViolations(page, 'settings (dark)');
});

test('shows_engine_version_and_storage', async ({ page }) => {
  await page.goto('/settings');
  // Engine version is a semver string reported by the data worker.
  await expect(page.getByText(/\d+\.\d+\.\d+/)).toBeVisible();
  // Storage estimate: usage, quota and persistence state.
  await expect(page.getByText(/Persistent storage/)).toBeVisible();
  await expect(page.getByText(/quota/i)).toBeVisible();
  await expectNoA11yViolations(page, 'settings (light)');
});
