// Layout resilience (spec §9.3): no horizontal scroll at 320px wide and
// no clipped content at 200% zoom.
import { expect, test } from '@playwright/test';
import { ROUTES } from './routes';

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
