// Calculator offline e2e (v1.0 plan Task 14 Step 1, spec §7.1): a
// calculator link keeps working with the server dead after the first
// visit — the production service worker precaches the shell, the
// core-wasm asset and the calc worker chunk, and the store-less
// worker needs nothing else. Mirrors offline.spec.ts's real-offline
// pattern (a second `vite preview` that gets killed).
import { spawn, type ChildProcess } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';

const VITE = new URL('../node_modules/vite/bin/vite.js', import.meta.url);

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Starts a second `vite preview` on a worker-private port. Real
 *  "offline" below is the server gone: `context.setOffline` and
 *  `route.abort` both preempt the service worker in Chromium, so only
 *  a dead origin proves the precache serves the app. */
async function startPreview(
  workerIndex: number,
): Promise<{ url: string; proc: ChildProcess }> {
  const port = 4400 + workerIndex;
  const url = `http://localhost:${port}`;
  const proc = spawn(
    process.execPath,
    [fileURLToPath(VITE), 'preview', '--port', String(port), '--strictPort'],
    { stdio: 'ignore' },
  );
  for (let i = 0; i < 60; i++) {
    if (proc.exitCode !== null) {
      throw new Error(`vite preview on :${port} exited (${proc.exitCode})`);
    }
    try {
      await fetch(url);
      return { url, proc };
    } catch {
      await sleep(500);
    }
  }
  proc.kill('SIGKILL');
  throw new Error(`vite preview on :${port} did not start`);
}

function killPreview(proc: ChildProcess): Promise<void> {
  return new Promise((resolve) => {
    proc.once('exit', () => resolve());
    proc.kill('SIGKILL');
    setTimeout(resolve, 2000);
  });
}

/** Waits until the production service worker is activated. Without
 *  `clientsClaim` the registering page is only *controlled* from its
 *  next navigation, so callers reload once before depending on the sw. */
async function swActivated(page: Page): Promise<void> {
  await page.waitForFunction(async () => {
    const reg = await navigator.serviceWorker?.getRegistration();
    return reg?.active?.state === 'activated';
  });
}

/** Waits until the service worker controls this page. */
async function swControlled(page: Page): Promise<void> {
  for (let i = 0; i < 10; i++) {
    const controlled = await page.evaluate(
      () => navigator.serviceWorker?.controller !== null,
    );
    if (controlled) return;
    await page.reload();
  }
  throw new Error('service worker never controlled the page');
}

test('calculator_link_works_offline_after_first_visit', async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  const preview = await startPreview(testInfo.workerIndex);
  try {
    // First visit: the sw installs and precaches shell + wasm + the
    // calc worker chunk.
    await page.goto(`${preview.url}/calc/two_by_two?strata=10,11;4,20&conf_level=95`);
    await expect(
      page.getByRole('row', { name: /Odds ratio — estimate/ }).first(),
    ).toContainText('4.545');
    await swActivated(page);
    await page.reload();
    await swControlled(page);
    await killPreview(preview.proc);

    // Server dead: the link still computes — navigation comes from
    // navigateFallback, the worker and wasm from the precache.
    await page.goto(`${preview.url}/calc/two_by_two?strata=10,11;4,20&conf_level=95`);
    await expect(
      page.getByRole('row', { name: /Odds ratio — estimate/ }).first(),
    ).toContainText('4.545');
    await page.goto(`${preview.url}/calc/proportion?numerator=3&denominator=7`);
    await expect(
      page.getByRole('heading', { name: 'Results' }),
    ).toBeVisible();
  } finally {
    await killPreview(preview.proc);
  }
});
