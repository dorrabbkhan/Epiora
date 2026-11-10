// Tab lock e2e (v0.3 plan Task 13, spec §7.1, plan Review Focus 4):
// the OPFS sahpool allows one store connection, so the data worker
// holds a Web Lock for its lifetime. A second tab shows the lock
// screen; "Use here" hands the lock over and the old tab loses it.
import { expect, test } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';

const LOCK_TITLE = 'Epiora is open in another tab';
const PROJECTS = 'Projects';

test('second tab shows lock message', async ({ context }) => {
  const first = await context.newPage();
  await first.goto('/');
  // The projects list needs the store, so seeing it means the first
  // tab's worker holds the lock.
  await expect(
    first.getByRole('heading', { name: PROJECTS }),
  ).toBeVisible();

  const second = await context.newPage();
  await second.goto('/');
  await expect(second.getByText(LOCK_TITLE)).toBeVisible();
  await expect(
    second.getByRole('button', { name: 'Use here' }),
  ).toBeVisible();
  await expectNoA11yViolations(second, 'lock screen');
  await first.close();
  await second.close();
});

test('use_here_takes_over', async ({ context }) => {
  const first = await context.newPage();
  await first.goto('/');
  await expect(
    first.getByRole('heading', { name: PROJECTS }),
  ).toBeVisible();

  const second = await context.newPage();
  await second.goto('/');
  await expect(second.getByText(LOCK_TITLE)).toBeVisible();
  await second.getByRole('button', { name: 'Use here' }).click();

  // The second tab reloads into the app; the first tab's worker loses
  // the lock, closes its store and shows the lock screen.
  await expect(
    second.getByRole('heading', { name: PROJECTS }),
  ).toBeVisible();
  await expect(first.getByText(LOCK_TITLE)).toBeVisible();
  await expectNoA11yViolations(first, 'first tab lock screen');
  await first.close();
  await second.close();
});
