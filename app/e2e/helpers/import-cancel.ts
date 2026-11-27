// Deterministic import-cancel e2e helpers: the VITE_TEST_HOOKS=1 worker
// gate parks a commit at its transaction boundary while the test clicks
// the real Cancel control. No wall-clock waits are involved.
import { expect, type Locator, type Page } from '@playwright/test';

type HookWindow = Window & {
  __epioraImportCommitHeld?: Promise<void>;
  __epioraCancelDelivered?: Promise<void>;
};

/** Arms the worker's one-shot pre-commit hold and captures the real
 *  Cancel button's RPC promise so the test releases the hold only after
 *  that click has reached the worker. */
export async function armImportCommitHold(page: Page): Promise<void> {
  await page.evaluate(() => {
    const w = window as HookWindow;
    w.__epioraImportCommitHeld = window.dataApi.__holdNextImportCommit();
    const original = window.dataApi.cancelImport.bind(window.dataApi);
    const api = window.dataApi as unknown as Record<string, unknown>;
    api.cancelImport = (id: string) => {
      const delivered = original(id);
      w.__epioraCancelDelivered = delivered;
      return delivered;
    };
  });
}

/** Waits until the commit is parked immediately before transaction
 *  commit, clicks the real control, waits for its cancellation RPC to
 *  finish in the worker, then releases the test-only hold. */
export async function cancelHeldImport(page: Page, cancel: Locator): Promise<void> {
  await expect(cancel).toBeVisible();
  await page.evaluate(() => {
    const held = (window as HookWindow).__epioraImportCommitHeld;
    if (held === undefined) throw new Error('import commit hold was not armed');
    return held;
  });
  await cancel.click();
  await page.evaluate(() => {
    const delivered = (window as HookWindow).__epioraCancelDelivered;
    if (delivered === undefined) throw new Error('Cancel import did not invoke cancelImport');
    return delivered;
  });
  await page.evaluate(() => window.dataApi.__releaseImportCommitHold());
}

/** The cancellation alert, discriminated from unrelated alert-role
 *  toasts such as the service-worker update prompt. */
export function cancelAlert(page: Page): Locator {
  return page.getByRole('alert').filter({ hasText: /cancelled/i });
}
