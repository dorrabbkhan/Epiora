// PWA e2e (v0.3 plan Task 13, spec §7.1): the production build's
// service worker precaches the shell, both wasm packages and
// sqlite-wasm, so after the first load the whole import → TABLES flow
// works with the network off. Updates prompt instead of swapping
// mid-session, and the first import asks for persistent storage.
import { appendFileSync, cpSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn, type ChildProcess } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';

const UTF8_COMMA = readFileSync(
  new URL('../../fixtures/import/utf8_comma/input.csv', import.meta.url),
  'utf8',
);
const DIST = fileURLToPath(new URL('../dist', import.meta.url));
const VITE = new URL('../node_modules/vite/bin/vite.js', import.meta.url);

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Starts a second `vite preview` on a worker-private port and returns
 *  its URL plus a kill handle. Real "offline" below is the server
 *  gone: `context.setOffline` and `route.abort` both preempt the
 *  service worker in Chromium, so only a dead origin proves the
 *  precache serves the app (verified: reload, fetch and the data
 *  worker all answer with the server killed). */
async function startPreview(
  workerIndex: number,
  outDir?: string,
): Promise<{
  url: string;
  proc: ChildProcess;
}> {
  const port = 4300 + workerIndex;
  const url = `http://localhost:${port}`;
  const args = ['preview', '--port', String(port), '--strictPort'];
  if (outDir !== undefined) args.push('--outDir', outDir);
  const proc = spawn(process.execPath, [fileURLToPath(VITE), ...args], {
    stdio: 'ignore',
  });
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

/** Waits until the service worker controls this page. A freshly
 *  activated sw takes a moment before it starts answering fetches, so
 *  a navigation can slip through uncontrolled; keep reloading until
 *  one lands inside it. */
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

/** Imports `csv` into a fresh project through the real RPC. */
async function seedCsv(
  page: Page,
  csv: string,
): Promise<{ projectId: string; datasetId: string }> {
  return page.evaluate(async (csv) => {
    const api = window.dataApi;
    const project = await api.createProject('Offline');
    const bytes = new TextEncoder().encode(csv);
    const { sessionId, inspection } = await api.inspectFile(
      'input.csv',
      bytes.buffer as ArrayBuffer,
    );
    const result = await api.commitImport(
      sessionId,
      { projectId: project.id, datasetName: 'input.csv' },
      inspection.settings,
      false,
      () => {},
    );
    return { projectId: project.id, datasetId: result.datasetId };
  }, csv);
}

test('works_offline_after_first_load', async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  const preview = await startPreview(testInfo.workerIndex);
  try {
    await page.goto(preview.url);
    await swActivated(page);
    // The registering page is not controlled until its next
    // navigation — reload once while the server is still up and wait
    // for control before taking it down.
    await page.reload();
    await swControlled(page);
    await killPreview(preview.proc);

    await page.reload();
    await expect(
      page.getByRole('heading', { name: 'Projects' }),
    ).toBeVisible();

    // utf8_comma (AGE integer, NAME text) imports and runs offline.
    // The absolute URL matters: a relative goto would land on the
    // config's baseURL server (still alive), a different origin with a
    // different OPFS database. The sw's navigateFallback serves
    // index.html for this navigation.
    const seed = await seedCsv(page, UTF8_COMMA);
    await page.goto(`${preview.url}/p/${seed.projectId}/analysis`);
    // The store re-opens under a fresh worker — the sahpool retry
    // window alone is 10 s.
    await expect(
      page.getByRole('button', { name: 'Run procedure' }),
    ).toBeEnabled({ timeout: 30_000 });
    await page.getByRole('button', { name: 'Run procedure' }).click();
    await page.getByRole('menuitem', { name: 'TABLES', exact: true }).click();
    const dialog = page.getByRole('dialog', { name: 'TABLES', exact: true });
    await expect(dialog).toBeVisible();

    // NAME is two-level Text: both level picks are required. AGE is
    // integer — its {20, (missing)} pair only counts as two levels
    // with include_missing, and the engine refuses to orient it
    // (ambiguous_levels). Left unchecked the run is a valid RxC.
    const pick = async (label: RegExp, option: string) => {
      await dialog.getByRole('button', { name: label }).focus();
      await page.keyboard.press('Enter');
      await page.getByRole('option', { name: option, exact: true }).click();
    };
    await pick(/Exposure variable/, 'NAME');
    await pick(/NAME first level/, 'alice');
    await pick(/NAME second level/, 'bob');
    await pick(/Outcome variable/, 'AGE');
    await dialog.getByRole('button', { name: 'Run', exact: true }).click();

    await expect(
      page.getByRole('heading', { name: 'TABLES NAME × AGE — input.csv v1' }),
    ).toBeVisible();
  } finally {
    preview.proc.kill('SIGKILL');
  }
});

test('update_prompts_instead_of_swapping', async ({ page }, testInfo) => {
  // Serve a worker-private copy of dist on the private port so the
  // "redeploy" below — new sw.js bytes on disk — cannot leak into
  // parallel specs sharing the real preview. (Playwright routes do
  // not reach service-worker update checks, so this has to be a real
  // file on a real origin.)
  const dir = mkdtempSync(join(tmpdir(), 'epiora-dist-'));
  try {
    cpSync(DIST, dir, { recursive: true });
    const preview = await startPreview(testInfo.workerIndex, dir);
    try {
      await page.goto(preview.url);
      await swActivated(page);
      // A silent swap mid-session would fire a document load event.
      let loads = 0;
      page.on('load', () => loads++);

      // "Deploy a second build": changed sw.js bytes are what the
      // update check compares.
      appendFileSync(join(dir, 'sw.js'), '\n// mock redeploy\n');

      // Reloading both controls the page (no clientsClaim: navigations
      // after activation attach the controller) and runs the browser's
      // update check, which finds the new sw; `skipWaiting: false`
      // keeps it `waiting` until the user clicks Reload.
      await page.reload();
      await swControlled(page);
      await page.waitForFunction(
        async () =>
          (await navigator.serviceWorker?.getRegistration())?.waiting !==
          null,
      );

      const baseline = loads;
      const toast = page.getByText('New version available');
      await expect(toast).toBeVisible();
      await expectNoA11yViolations(page, 'update toast');
      // No reload happened since the navigations settled: no silent swap.
      expect(loads).toBe(baseline);

      await Promise.all([
        page.waitForEvent('load'),
        page.getByRole('button', { name: 'Reload', exact: true }).click(),
      ]);
      await swActivated(page);
    } finally {
      preview.proc.kill('SIGKILL');
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('persistence_requested_on_first_import', async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as unknown as { __persistRequested: boolean };
    w.__persistRequested = false;
    if (navigator.storage !== undefined) {
      navigator.storage.persist = () => {
        w.__persistRequested = true;
        return Promise.resolve(true);
      };
    }
  });
  await page.goto('/');
  await seedCsv(page, UTF8_COMMA);
  await expect(
    page.evaluate(
      () => (window as unknown as { __persistRequested: boolean }).__persistRequested,
    ),
  ).resolves.toBe(true);
});
