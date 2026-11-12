// Keyboard path e2e (v0.3 plan Task 12, Review Focus 5): the whole
// path — create project → import CSV → open TABLES → run → expand
// receipt — is driven with the keyboard only (the OS file chooser
// excepted; Playwright intercepts it). After every step the focused
// element must show a visible focus indicator (computed outline or
// box-shadow).
import { readFileSync } from 'node:fs';
import { expect, test, type Locator, type Page } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';

const OSWEGO = readFileSync(
  new URL('../../fixtures/data/oswego.csv', import.meta.url),
);

async function assertFocusVisible(page: Page, context: string): Promise<void> {
  const ok = await page.evaluate(() => {
    const el = document.activeElement as HTMLElement | null;
    if (el === null || el === document.body || el === document.documentElement) {
      return false;
    }
    const cs = getComputedStyle(el);
    const outline = cs.outlineStyle !== 'none' && cs.outlineWidth !== '0px';
    return outline || cs.boxShadow !== 'none';
  });
  expect(ok, `focus indicator missing at ${context}`).toBe(true);
}

/** Tabs forward until `target` holds focus; every stop on the way must
 *  show a focus indicator. */
async function tabTo(page: Page, target: Locator, context: string): Promise<void> {
  for (let i = 0; i < 60; i++) {
    if (
      await target
        .evaluate((el) => el === document.activeElement)
        .catch(() => false)
    ) {
      await assertFocusVisible(page, context);
      return;
    }
    await page.keyboard.press('Tab');
  }
  throw new Error(`never reached ${context} by Tab`);
}

/** Tabs forward until focus is anywhere inside `container` (for grid
 *  widgets whose inner controls live behind arrow-key navigation). */
async function tabInto(
  page: Page,
  container: Locator,
  context: string,
): Promise<void> {
  for (let i = 0; i < 60; i++) {
    if (await container.evaluate((el) => el.contains(document.activeElement))) {
      await assertFocusVisible(page, context);
      return;
    }
    await page.keyboard.press('Tab');
  }
  throw new Error(`never reached ${context} by Tab`);
}

/** Presses `key` until `target` holds focus (grid/row internals). */
async function arrowTo(
  page: Page,
  target: Locator,
  key: string,
  context: string,
): Promise<void> {
  for (let i = 0; i < 30; i++) {
    if (
      await target
        .evaluate((el) => el === document.activeElement)
        .catch(() => false)
    ) {
      await assertFocusVisible(page, context);
      return;
    }
    await page.keyboard.press(key);
  }
  throw new Error(`never reached ${context} by ${key}`);
}

test('keyboard path test', async ({ page }) => {
  // Owner-approved 2026-11-12, this test only: ~20–24 s alone but hits
  // the 30 s default inside AxeBuilder.analyze under the full suite
  // (main run 43208388072; PR #56 run 43250830143).
  test.setTimeout(60_000);
  await page.goto('/');

  // Create a project.
  const create = page.getByRole('button', { name: 'New project' });
  await tabTo(page, create, 'New project button');
  await page.keyboard.press('Enter');
  const nameInput = page.getByRole('textbox', { name: 'Project name' });
  await expect(nameInput).toBeFocused();
  await assertFocusVisible(page, 'project name input');
  await page.keyboard.type('Keyboard project');
  const createSubmit = page.getByRole('button', { name: 'Create' });
  await tabTo(page, createSubmit, 'Create button');
  await page.keyboard.press('Enter');

  // Open the project: the row link lives inside the grid's arrow-key
  // navigation, not the tab order.
  const grid = page.getByRole('grid', { name: 'Projects' });
  await tabInto(page, grid, 'projects grid');
  const projectLink = page.getByRole('link', { name: 'Keyboard project' });
  await arrowTo(page, projectLink, 'ArrowRight', 'project link');
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', { name: 'Keyboard project' }),
  ).toBeVisible();

  // Import the CSV (the chooser itself is outside the page).
  const importLink = page.getByRole('link', { name: 'Import data' });
  await tabTo(page, importLink, 'Import data link');
  await page.keyboard.press('Enter');
  const chooseFile = page.getByRole('button', { name: 'Choose file' });
  await tabTo(page, chooseFile, 'Choose file button');
  const chooser = page.waitForEvent('filechooser');
  await page.keyboard.press('Enter');
  await (await chooser).setFiles({
    name: 'oswego.csv',
    mimeType: 'text/csv',
    buffer: OSWEGO,
  });

  // Walk the wizard: preview → settings → confirm → progress → done.
  await expect(
    page.getByRole('columnheader', { name: /VANILLA/ }),
  ).toBeVisible();
  await expectNoA11yViolations(page, 'wizard preview (keyboard path)');
  const next = page.getByRole('button', { name: 'Next' });
  await tabTo(page, next, 'wizard Next (preview)');
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('button', { name: 'Record key' }),
  ).toBeVisible();
  await tabTo(page, next, 'wizard Next (settings)');
  await page.keyboard.press('Enter');
  const importButton = page.getByRole('button', {
    name: 'Import',
    exact: true,
  });
  await tabTo(page, importButton, 'wizard Import');
  await page.keyboard.press('Enter');
  await expect(page.getByText('Dataset version 1 created.')).toBeVisible();

  // Project → analysis workspace.
  const back = page.getByRole('link', { name: 'Back to project' });
  await tabTo(page, back, 'Back to project link');
  await page.keyboard.press('Enter');
  const analysisLink = page.getByRole('link', {
    name: 'Analysis workspace',
  });
  await tabTo(page, analysisLink, 'Analysis workspace link');
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', { name: 'Analysis' }),
  ).toBeVisible();

  // Open the TABLES dialog through the procedure menu.
  const procedure = page.getByRole('button', { name: 'Run procedure' });
  await tabTo(page, procedure, 'Run procedure button');
  await page.keyboard.press('Enter');
  await page.keyboard.press('ArrowDown');
  await page.keyboard.press('Enter');
  const dialog = page.getByRole('dialog', { name: 'TABLES' });
  await expect(dialog).toBeVisible();
  await expectNoA11yViolations(page, 'TABLES dialog (keyboard path)');

  // Exposure: VANILLA (integer levels auto-orient).
  const exposure = page.getByRole('button', { name: /Exposure variable/ });
  await tabTo(page, exposure, 'exposure select');
  await page.keyboard.press('Enter');
  await page.keyboard.press('v');
  await page.keyboard.press('Enter');

  // Outcome: ILL.
  const outcome = page.getByRole('button', { name: /Outcome variable/ });
  await tabTo(page, outcome, 'outcome select');
  await page.keyboard.press('Enter');
  await page.keyboard.press('i');
  await page.keyboard.press('Enter');

  // Run.
  const run = page.getByRole('button', { name: 'Run', exact: true });
  await tabTo(page, run, 'Run button');
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('heading', {
      name: 'TABLES VANILLA × ILL — oswego.csv v1',
    }),
  ).toBeVisible();
  await expectNoA11yViolations(page, 'workspace with result card');

  // Expand the receipt.
  const receipt = page.getByRole('button', { name: 'Receipt' });
  await tabTo(page, receipt, 'Receipt disclosure');
  await page.keyboard.press('Enter');
  await expect(
    page.getByRole('button', { name: 'Copy receipt JSON' }),
  ).toBeVisible();
  await expectNoA11yViolations(page, 'expanded receipt (keyboard path)');
});
