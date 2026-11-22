// Tab lock e2e (v0.3 plan Task 13, spec §7.1, plan Review Focus 4):
// the OPFS sahpool allows one store connection, so the data worker
// holds a Web Lock for its lifetime. A second tab shows the lock
// screen; "Use here" hands the lock over and the old tab loses it.
// The take-over fix (2026-11-22): the old worker must also release
// the sahpool's file access handles or the new tab's list stays
// "Loading…" forever, so this spec waits for the loaded project
// list — never just the page shell.
import { expect, test } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';

const LOCK_TITLE = 'Epiora is open in another tab';
const PROJECTS = 'Projects';
const LOADING = 'Loading…';

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

  // A project created here proves the second tab opened the same
  // database after the take-over: the "Projects" heading alone renders
  // while the list is still "Loading…".
  await first.getByRole('button', { name: 'New project' }).click();
  const createDialog = first.getByRole('dialog');
  await createDialog.getByRole('textbox').fill('Oswego');
  await createDialog.getByRole('button', { name: 'Create' }).click();
  await expect(
    first.getByRole('rowheader', { name: 'Oswego' }),
  ).toBeVisible();

  const second = await context.newPage();
  await second.goto('/');
  await expect(second.getByText(LOCK_TITLE)).toBeVisible();
  await second.getByRole('button', { name: 'Use here' }).click();

  // The second tab reloads into the app; the first tab's worker loses
  // the lock, closes its store, releases the sahpool and shows the
  // lock screen — so the second tab's list finishes loading.
  await expect(
    second.getByRole('rowheader', { name: 'Oswego' }),
  ).toBeVisible();
  await expect(second.getByText(LOADING)).toHaveCount(0);
  await expect(first.getByText(LOCK_TITLE)).toBeVisible();
  await expectNoA11yViolations(first, 'first tab lock screen');

  // The take-over works both ways: "Use here" in the locked first tab
  // takes the lock back and its list loads the same project.
  await first.getByRole('button', { name: 'Use here' }).click();
  await expect(
    first.getByRole('rowheader', { name: 'Oswego' }),
  ).toBeVisible();
  await expect(first.getByText(LOADING)).toHaveCount(0);
  await expect(second.getByText(LOCK_TITLE)).toBeVisible();
  await first.close();
  await second.close();
});
