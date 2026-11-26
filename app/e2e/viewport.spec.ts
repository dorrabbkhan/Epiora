// Layout resilience (spec §9.3): no horizontal scroll at 320px wide and
// no clipped content at 200% zoom.
import { expect, test } from '@playwright/test';
import { ROUTES } from './routes';
import { walkToPreview } from './helpers/legacy';

test.describe('320px viewport', () => {
  test.use({ viewport: { width: 320, height: 640 } });
  for (const route of ROUTES) {
    test(`${route} has no horizontal scroll`, async ({ page }) => {
      await page.goto(route);
      const scrollWidth = await page.evaluate(
        () => document.documentElement.scrollWidth,
      );
      expect(scrollWidth).toBeLessThanOrEqual(320);
    });
  }
});

test.describe('200% zoom', () => {
  test.use({ viewport: { width: 1280, height: 800 } });
  for (const route of ROUTES) {
    test(`${route} clips no content at 200% zoom`, async ({ page }) => {
      await page.goto(route);
      await page.evaluate(() => {
        // CSS zoom on <html> zooms layout independent of deviceScaleFactor.
        document.documentElement.style.zoom = '2';
      });
      const { scrollWidth, innerWidth } = await page.evaluate(() => ({
        scrollWidth: document.documentElement.scrollWidth,
        innerWidth: window.innerWidth,
      }));
      expect(scrollWidth).toBeLessThanOrEqual(innerWidth);
    });
  }
});

async function legacyStates(page: import('@playwright/test').Page, check: () => Promise<void>) {
  await walkToPreview(page);
  await check();
  await page.getByRole('button', { name: 'Confirm import' }).click();
  await expect(page).toHaveURL(/\/p\/[^/]+$/, { timeout: 60_000 });
  await page.goto(`/p/${page.url().split('/p/')[1]!}/reimport`);
  await expect(page.getByRole('heading', { name: 'Re-import a newer copy' })).toBeVisible();
  await check();
}

test.describe('320px legacy states', () => {
  test.use({ viewport: { width: 320, height: 640 } });
  test('legacy preview and reimport have no horizontal scroll', async ({ page }) => {
    test.setTimeout(120_000);
    await legacyStates(page, async () => {
      expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(320);
    });
  });
});

test.describe('200% zoom legacy states', () => {
  test.use({ viewport: { width: 1280, height: 800 } });
  test('legacy preview and reimport clip no content', async ({ page }) => {
    test.setTimeout(120_000);
    await legacyStates(page, async () => {
      const widths = await page.evaluate(() => {
        document.documentElement.style.zoom = '2';
        return {
          scrollWidth: document.documentElement.scrollWidth,
          innerWidth: window.innerWidth,
        };
      });
      expect(widths.scrollWidth).toBeLessThanOrEqual(widths.innerWidth);
    });
  });
});
