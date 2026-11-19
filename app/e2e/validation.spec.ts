// `/validation` e2e (v1.0 plan Task 24, spec §7.4): the public
// validation dossier lists every `validation` registry statistic,
// every explain dialog links to its entry, no number on the page is
// absent from `validation.json`, and the page works offline like
// `/calc`. Builds without `validation.json` (local preview) exercise
// the registry fallback and its "observed errors not measured in this
// build" notice.

import { spawn, type ChildProcess } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';
import { formatNumber } from '../src/ui/format';

const VITE = new URL('../node_modules/vite/bin/vite.js', import.meta.url);

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

interface Statistic {
  id: string;
  cases: number;
  worst_error?: number | null;
}

/** The `validation.json` the build bundled, read in the page's
 *  origin; `null` when the build has none (the fallback path). */
async function fetchReport(
  page: Page,
): Promise<{ statistics: Statistic[] } | null> {
  return page.evaluate(async () => {
    const res = await fetch('/validation.json');
    // No bundled report: the SPA fallback serves index.html (200,
    // text/html) for the path — only a JSON body counts.
    if (!res.ok || !(res.headers.get('content-type') ?? '').includes('json')) {
      return null;
    }
    return (await res.json()) as { statistics: Statistic[] };
  });
}

/** Starts a second `vite preview` on a worker-private port — the
 *  same real-offline pattern as calc-offline.spec.ts: only a dead
 *  origin proves the precache serves the app. */
async function startPreview(
  workerIndex: number,
): Promise<{ url: string; proc: ChildProcess }> {
  const port = 4500 + workerIndex;
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

/** Waits until the production service worker is activated. */
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

test('page_lists_every_statistic', async ({ page }) => {
  await page.goto('/validation');
  const table = page.getByRole('grid', {
    name: 'Validation registry statistics',
  });
  await expect(table).toBeVisible();
  const report = await fetchReport(page);
  if (report === null) {
    // No bundled report: the wasm registry fallback lists every
    // statistic and names the unmeasured build.
    await expect(
      page.getByText('observed errors not measured in this build'),
    ).toBeVisible();
    await expect(
      page.locator('[id="two_by_two.odds_ratio.estimate"]'),
    ).toHaveCount(1);
    return;
  }
  for (const s of report.statistics) {
    await expect(page.locator(`[id="${s.id}"]`)).toHaveCount(1);
  }
});

test('explain_links_to_entry', async ({ page }) => {
  await page.goto('/calc/two_by_two?strata=10,11;4,20&conf_level=95');
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  const row = page
    .getByRole('row')
    .filter({
      has: page.getByRole('rowheader', {
        name: 'Odds ratio — estimate',
        exact: true,
      }),
    })
    .first();
  await row.getByRole('button', { name: 'Explain' }).click();
  const dialog = page.getByRole('dialog', { name: /Explain/ });
  const link = dialog.getByRole('link', {
    name: "See this statistic's validation entry",
  });
  await expect(link).toBeVisible();
  const href = await link.getAttribute('href');
  expect(href).toMatch(/^\/validation#/);
  await link.click();
  await expect(page).toHaveURL(/\/validation#/);
  const id = decodeURIComponent(href!.split('#')[1]!);
  await expect(page.locator(`[id="${id}"]`)).toHaveCount(1);
});

test('numbers_come_from_json', async ({ page }) => {
  await page.goto('/validation');
  const table = page.getByRole('grid', {
    name: 'Validation registry statistics',
  });
  await expect(table).toBeVisible();
  const report = await fetchReport(page);
  if (report === null) {
    // Fallback: no measured numbers at all — the notice stands in
    // for every observed error.
    await expect(
      page.getByText('observed errors not measured in this build'),
    ).toBeVisible();
    return;
  }
  // Every numeric cell renders a value the JSON carries: a `cases`
  // integer or a `worst_error` through the app's one number format.
  const allowed = new Set<string>();
  for (const s of report.statistics) {
    allowed.add(String(s.cases));
    if (typeof s.worst_error === 'number') {
      allowed.add(formatNumber(s.worst_error));
    }
  }
  for (const text of await table.getByRole('gridcell').allTextContents()) {
    const cell = text.trim();
    if (cell === '' || cell === '—' || allowed.has(cell)) continue;
    expect(
      Number.isNaN(Number(cell)),
      `cell "${cell}" is a number not present in validation.json`,
    ).toBe(true);
  }
});

test('works_offline', async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  const preview = await startPreview(testInfo.workerIndex);
  try {
    // First visit: the sw installs and precaches the shell, core-wasm
    // and the calc worker chunk (and `validation.json` when the build
    // bundled one — it is in the workbox glob).
    await page.goto(`${preview.url}/validation`);
    await expect(
      page.getByRole('grid', { name: 'Validation registry statistics' }),
    ).toBeVisible();
    await swActivated(page);
    await page.reload();
    await swControlled(page);
    await killPreview(preview.proc);

    // Server dead: the dossier still renders — measured numbers from
    // the precached JSON, or the wasm registry fallback.
    await page.goto(`${preview.url}/validation`);
    await expect(
      page.getByRole('grid', { name: 'Validation registry statistics' }),
    ).toBeVisible();
    await expect(
      page.locator('[id="two_by_two.odds_ratio.estimate"]'),
    ).toHaveCount(1);
  } finally {
    await killPreview(preview.proc);
  }
});
