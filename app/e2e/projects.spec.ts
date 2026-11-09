// Projects screen e2e (v0.3 plan Task 10): the empty state offers
// create, and a project can be created, renamed and deleted through
// the dialogs. Every visited state gets an axe pass.
import { expect, test } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';

test('empty_state_offers_create', async ({ page }) => {
  await page.goto('/');
  await expect(page.getByText('No projects yet')).toBeVisible();
  await expect(
    page.getByRole('button', { name: 'New project' }),
  ).toBeVisible();
  await expectNoA11yViolations(page, 'empty projects list');
});

test('create_rename_delete_project', async ({ page }) => {
  await page.goto('/');

  // Create through the name dialog.
  await page.getByRole('button', { name: 'New project' }).click();
  const createDialog = page.getByRole('dialog');
  await expectNoA11yViolations(page, 'create project dialog');
  await createDialog.getByRole('textbox').fill('Oswego');
  await createDialog.getByRole('button', { name: 'Create' }).click();
  const created = page.getByRole('rowheader', {
    name: 'Oswego',
    exact: true,
  });
  await expect(created).toBeVisible();
  await expectNoA11yViolations(page, 'projects list after create');

  // Rename through the row's dialog.
  await page
    .getByRole('row', { name: /Oswego/ })
    .getByRole('button', { name: 'Rename' })
    .click();
  const renameDialog = page.getByRole('dialog');
  await renameDialog.getByRole('textbox').fill('Oswego 1940');
  await renameDialog.getByRole('button', { name: 'Save' }).click();
  await expect(
    page.getByRole('rowheader', { name: 'Oswego 1940' }),
  ).toBeVisible();
  await expect(created).toHaveCount(0);

  // Delete behind a confirm dialog.
  await page
    .getByRole('row', { name: /Oswego 1940/ })
    .getByRole('button', { name: 'Delete' })
    .click();
  const deleteDialog = page.getByRole('dialog');
  await expectNoA11yViolations(page, 'delete project dialog');
  await deleteDialog.getByRole('button', { name: 'Delete' }).click();
  await expect(
    page.getByRole('rowheader', { name: 'Oswego 1940' }),
  ).toHaveCount(0);
  await expectNoA11yViolations(page, 'projects list after delete');
});
