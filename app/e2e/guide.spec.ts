// "Which calculator?" guide and assumption-note display e2e (v1.0
// plan Task 23, spec §6.1): the guide leads from a plain question to
// the right calculator (each answer in `?path=` so a guide state can
// be linked, design inputs prefilled), assumption notes appear under
// the table they refer to with a link to the suggested statistic,
// explain shows the rule and the values that triggered it, and note
// text is marked "pending adviser review".
import { spawn, type ChildProcess } from 'node:child_process';
import { fileURLToPath } from 'node:url';
import { expect, test, type Page } from '@playwright/test';

const VITE = new URL('../node_modules/vite/bin/vite.js', import.meta.url);

// A 2×2 with expected cell (1,1) = 3×4/14 ≈ 0.857 < 5 — the
// `expected_count_below_5` note fires and suggests the Fisher p.
const SPARSE = 'strata=1,2;3,8&conf_level=95';

// Two groups with plainly unequal variances — Bartlett p < 0.05
// fires `unequal_variances` and suggests the Welch t.
const UNEQUAL_CSV = [
  'GRP,VAL',
  'A,10',
  'A,10.1',
  'A,9.9',
  'A,10',
  'A,10.05',
  'A,9.95',
  'B,0',
  'B,20',
  'B,5',
  'B,15',
  'B,10',
  'B,25',
].join('\n');

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));

/** Imports a CSV into a fresh project through the real RPC — the same
 *  seed shape analysis.spec.ts uses for Oswego. */
async function seedCsv(page: Page, csv: string, name: string) {
  return page.evaluate(
    async ({ csv, name }) => {
      const api = window.dataApi;
      const project = await api.createProject('Notes project');
      const bytes = new TextEncoder().encode(csv);
      const { sessionId, inspection } = await api.inspectFile(
        name,
        bytes.buffer as ArrayBuffer,
      );
      const result = await api.commitImport(
        sessionId,
        { projectId: project.id, datasetName: name },
        inspection.settings,
        false,
        () => {},
      );
      return { projectId: project.id, datasetId: result.datasetId };
    },
    { csv, name },
  );
}

/** Starts a second `vite preview` on a worker-private port — the real
 *  "offline" below is the server gone (mirrors calc-offline.spec.ts). */
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

async function swActivated(page: Page): Promise<void> {
  await page.waitForFunction(async () => {
    const reg = await navigator.serviceWorker?.getRegistration();
    return reg?.active?.state === 'activated';
  });
}

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

test('guide_to_two_by_two_with_link', async ({ page }) => {
  await page.goto('/calc/guide');
  await expect(
    page.getByRole('heading', { name: 'Which calculator?' }),
  ).toBeVisible();

  await page.getByRole('link', { name: 'I have counts or a table' }).click();
  await expect(page).toHaveURL(/path=a/);
  await page
    .getByRole('link', { name: 'A 2×2 table — exposure by outcome' })
    .click();
  await expect(page).toHaveURL(/path=a\.b/);
  await page
    .getByRole('link', { name: 'Cohort, cross-sectional or trial' })
    .click();
  await expect(page).toHaveURL(/path=a\.b\.a/);

  await expect(page.getByText('Recommended calculator')).toBeVisible();
  const open = page.getByRole('link', { name: 'Open Two by two table' });
  await expect(open).toBeVisible();
  await open.click();
  await expect(page).toHaveURL(/\/calc\/two_by_two\?.*study_type=cohort/);
  await expect(
    page.getByRole('heading', { name: 'Two by two table' }),
  ).toBeVisible();
});

test('guide_works_offline', async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  const preview = await startPreview(testInfo.workerIndex);
  try {
    // First visit: the sw installs and precaches shell + wasm — the
    // guide is store-less static data and needs nothing else.
    await page.goto(`${preview.url}/calc/guide`);
    await expect(
      page.getByRole('heading', { name: 'Which calculator?' }),
    ).toBeVisible();
    await swActivated(page);
    await page.reload();
    await swControlled(page);
    await killPreview(preview.proc);

    // Server dead: the question flow still walks to a leaf and the
    // suggested calculator link still opens the form.
    await page.goto(`${preview.url}/calc/guide?path=a.b`);
    await page
      .getByRole('link', { name: 'Cohort, cross-sectional or trial' })
      .click();
    await page
      .getByRole('link', { name: 'Open Two by two table' })
      .click();
    await expect(
      page.getByRole('heading', { name: 'Two by two table' }),
    ).toBeVisible();
  } finally {
    await killPreview(preview.proc);
  }
});

test('sparse_table_shows_fisher_note_with_link', async ({ page }) => {
  await page.goto(`/calc/two_by_two?${SPARSE}`);
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  // The note sits under the table it refers to (the Crude block).
  const note = page
    .getByRole('note')
    .filter({ hasText: 'expected cell count is below 5' })
    .first();
  await expect(note).toBeVisible();
  const link = note.getByRole('link', { name: /Fisher exact/ });
  await expect(link).toBeVisible();
  await link.click();
  await expect(page).toHaveURL(/#note-/);
  await expect(
    page.getByRole('row', { name: /Fisher exact — p \(2-sided\)/ }).first(),
  ).toBeVisible();
});

test('means_unequal_variances_note', async ({ page }) => {
  await page.goto('/');
  const seed = await seedCsv(page, UNEQUAL_CSV, 'unequal.csv');
  await page.goto(`/p/${seed.projectId}/analysis`);
  await expect(
    page.getByRole('heading', { name: 'Analysis' }),
  ).toBeVisible();

  await page.getByRole('button', { name: 'Run procedure' }).click();
  await page.getByRole('menuitem', { name: 'MEANS', exact: true }).click();
  const pick = async (label: RegExp, option: string) => {
    await page.getByRole('button', { name: label }).focus();
    await page.keyboard.press('Enter');
    await page.getByRole('option', { name: option, exact: true }).click();
  };
  await pick(/Variable/, 'VAL');
  await pick(/Group by/, 'GRP');
  await page.getByRole('button', { name: 'Run', exact: true }).click();

  const card = page.getByRole('article', {
    name: 'MEANS VAL GRP — unequal.csv v1',
  });
  await expect(card).toBeVisible();
  const note = card
    .getByRole('note')
    .filter({ hasText: 'variances differ' });
  await expect(note).toBeVisible();
  // The suggested statistic is the Welch t — a link to its row.
  const link = note.getByRole('link', { name: /Welch/ });
  await link.click();
  await expect(page).toHaveURL(/#note-/);
});

test('explain_shows_note_rule_and_values', async ({ page }) => {
  await page.goto(`/calc/two_by_two?${SPARSE}`);
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  // The note's `statistics` name the crude block's χ² p-values (the
  // Strata 1 row's `strata[0]` path never equals its `strata[]`
  // wildcard), so the row must come from the Crude table. The label's
  // first letter renders uppercased — 'χ' → 'Χ'.
  const row = page
    .getByRole('table', { name: 'Crude' })
    .getByRole('row')
    .filter({
      has: page.getByRole('rowheader', {
        name: 'Χ² — uncorrected — p (2-sided)',
        exact: true,
      }),
    });
  await row.getByRole('button', { name: 'Explain' }).click();
  const dialog = page.getByRole('dialog', { name: /Explain/ });
  await expect(dialog).toBeVisible();
  // The note's rule in words and the values that triggered it.
  await expect(dialog.getByText(/expected cell count/)).toBeVisible();
  await expect(dialog.getByText('Expected cells below 5')).toBeVisible();
  await expect(dialog.getByText('0.8571')).toBeVisible();
});

test('notes_marked_pending_review', async ({ page }) => {
  await page.goto(`/calc/two_by_two?${SPARSE}`);
  await expect(page.getByRole('heading', { name: 'Results' })).toBeVisible();
  await expect(
    page.getByText('Note text is pending adviser review.').first(),
  ).toBeVisible();
});
