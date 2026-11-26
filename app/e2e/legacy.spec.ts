// Legacy project import e2e (v1.1 plan Task 15, spec §7.1): file
// import, preview, cancellation, plain-database offer, re-import,
// offline use, the browser storage notice and the keyboard-only path.
import { fileURLToPath } from 'node:url';
import { spawn, type ChildProcess } from 'node:child_process';
import { expect, test, type Locator, type Page } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';
import {
  MISMATCH_DB,
  PLAIN_MDB,
  REIMPORT_DB,
  SAMPLE_DB,
  chooseFiles,
  chooseReimportFile,
  importSample,
  walkToPreview,
} from './helpers/legacy';

const sleep = (ms: number) => new Promise((r) => setTimeout(r, ms));
const VITE = new URL('../node_modules/vite/bin/vite.js', import.meta.url);

async function startPreview(workerIndex: number): Promise<{ url: string; proc: ChildProcess }> {
  const port = 4600 + workerIndex;
  const url = `http://localhost:${port}`;
  const proc = spawn(process.execPath, [fileURLToPath(VITE), 'preview', '--port', String(port), '--strictPort'], {
    stdio: 'ignore',
  });
  for (let i = 0; i < 60; i++) {
    if (proc.exitCode !== null) throw new Error(`vite preview on :${port} exited`);
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

async function swControlled(page: Page): Promise<void> {
  await page.waitForFunction(async () => {
    const reg = await navigator.serviceWorker?.getRegistration();
    return reg?.active?.state === 'activated';
  });
  for (let i = 0; i < 10; i++) {
    if (await page.evaluate(() => navigator.serviceWorker?.controller !== null)) return;
    await page.reload();
  }
  throw new Error('service worker never controlled the page');
}

async function tabTo(page: Page, target: Locator, context: string): Promise<void> {
  for (let i = 0; i < 80; i++) {
    if (await target.evaluate((el) => el === document.activeElement).catch(() => false)) return;
    await page.keyboard.press('Tab');
  }
  throw new Error(`never reached ${context} by Tab`);
}

test('imports_sample_project_with_labels_and_links', async ({ page }) => {
  test.setTimeout(90_000);
  await walkToPreview(page);

  const fields = page.getByRole('grid', { name: 'Oswego fields' });
  await expect(fields.getByRole('row', { name: 'ILL', exact: true })).toContainText('YesNo');
  await expect(fields.getByRole('row', { name: 'ILL', exact: true })).toContainText('boolean');
  await expect(fields.getByRole('row', { name: 'SEX', exact: true })).toContainText('LegalValues');
  await expect(fields.getByRole('row', { name: 'SEX', exact: true })).toContainText('text');
  const links = page.getByRole('grid', { name: 'Linked datasets' });
  await expect(links.getByRole('row', { name: 'Surveillance', exact: true }).first()).toContainText('RHepatitis');
  await expectNoA11yViolations(page, 'legacy preview');

  await page.getByRole('button', { name: 'Confirm import' }).click();
  await expect(page.getByRole('heading', { name: 'SQLiteSample2' })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole('row', { name: /Oswego/ })).toContainText('v1');
  await expectNoA11yViolations(page, 'imported legacy project');
});

for (const colorScheme of ['light', 'dark'] as const) {
  test(`legacy_preview_axe_${colorScheme}`, async ({ page }) => {
    test.setTimeout(90_000);
    await page.emulateMedia({ colorScheme });
    await walkToPreview(page);
    await expectNoA11yViolations(page, `legacy preview ${colorScheme}`);
  });
}

test('preview_lists_report_entries_before_writing', async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto('/');
  const before = await page.evaluate(() => window.dataApi.listProjects().then((p) => p.length));
  await walkToPreview(page);

  const report = page.getByRole('grid', { name: 'Migration report entries' });
  await expect(report.getByRole('row', { name: /field_not_imported/ }).first()).toBeVisible();
  await expect(report.getByRole('row', { name: /column_not_imported/ }).first()).toBeVisible();
  await expect(page.getByRole('grid', { name: 'Saved programs' }).getByText('Statistics')).toBeVisible();
  await expect(page.getByRole('grid', { name: 'Check code blocks' }).getByText('RHepatitis')).toBeVisible();
  await expectNoA11yViolations(page, 'legacy preview report entries');
  await expect.poll(() => page.evaluate(() => window.dataApi.listProjects().then((p) => p.length))).toBe(before);
});

test('cancel_mid_import_rolls_back', async ({ page }) => {
  test.setTimeout(90_000);
  await walkToPreview(page);
  await page.getByRole('button', { name: 'Confirm import' }).click();
  const cancel = page.getByRole('button', { name: 'Cancel import' });
  await expect(cancel).toBeVisible();
  await cancel.click();
  await expect(page.getByRole('alert')).toContainText(/cancelled/i);

  await expect
    .poll(() => page.evaluate(() => window.dataApi.listProjects().then((p) => p.length)))
    .toBe(0);
  const datasetCounts = await page.evaluate(() =>
    window.dataApi.listProjects().then((projects) =>
      Promise.all(projects.map((p) => window.dataApi.listDatasets(p.id).then((d) => d.length))),
    ),
  );
  expect(datasetCounts).toEqual([]);
});

test('plain_mdb_offers_table_import', async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto('/legacy');
  await chooseFiles(page, [PLAIN_MDB]);
  await expect(page.getByRole('alert')).toContainText('not a recognised Epi Info project');
  await expect(page.getByText('Import its tables as plain datasets')).toBeVisible();
  await page.getByRole('button', { name: 'Choose tables to import' }).click();
  await expect(page.getByRole('checkbox', { name: 'hosppts' })).toBeChecked();
  await expectNoA11yViolations(page, 'plain Access database offer');

  await page.getByRole('button', { name: 'Import selected tables' }).click();
  await expect(page.getByRole('heading', { name: 'HospitalPoints' })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole('row', { name: /hosppts/ })).toContainText('v1');
  const counts = await page.evaluate(async () => {
    const projects = await window.dataApi.listProjects();
    return {
      projects: projects.length,
      datasets: await window.dataApi.listDatasets(projects[0]!.id).then((d) => d.length),
    };
  });
  expect(counts).toEqual({ projects: 1, datasets: 1 });
});

test('cancel_mid_plain_table_import_rolls_back', async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto('/legacy');
  await chooseFiles(page, [PLAIN_MDB]);
  await expect(page.getByRole('alert')).toContainText('not a recognised Epi Info project');
  await page.getByRole('button', { name: 'Choose tables to import' }).click();
  await expect(page.getByRole('checkbox', { name: 'hosppts' })).toBeChecked();

  await page.getByRole('button', { name: 'Import selected tables' }).click();
  const cancel = page.getByRole('button', { name: 'Cancel import' });
  await expect(cancel).toBeVisible();
  await cancel.click();
  await expect(page.getByRole('alert')).toContainText(/cancelled/i);
  await expect
    .poll(() => page.evaluate(() => window.dataApi.listProjects().then((p) => p.length)))
    .toBe(0);
  const datasetCounts = await page.evaluate(() =>
    window.dataApi.listProjects().then((projects) =>
      Promise.all(projects.map((p) => window.dataApi.listDatasets(p.id).then((d) => d.length))),
    ),
  );
  expect(datasetCounts).toEqual([]);
});

test('reimport_shows_diff_and_creates_versions', async ({ page }) => {
  test.setTimeout(120_000);
  const projectId = await importSample(page);
  await page.goto(`/p/${projectId}/reimport`);
  await chooseReimportFile(page, REIMPORT_DB);

  const changes = page.getByRole('grid', { name: 'Re-import changes' });
  await expect(changes.getByRole('row', { name: /Oswego/ })).toContainText('3 added');
  await expect(changes.getByRole('row', { name: /Oswego/ })).toContainText('1 removed');
  await expect(changes.getByRole('row', { name: /Oswego/ })).toContainText('2 changed');
  await expect(page.getByText('Added columns: ReimportFlag')).toBeVisible();
  await expect(page.getByText('a0000000-0000-4000-8000-000000000001')).toBeVisible();
  await expect(page.getByText('00a9eb4f-c2d4-45f7-a0bc-edfd44b97772')).toBeVisible();
  await expect(page.getByText(/0672b138-2bb7-44c1-8144-14f26098354d AGE: 33 → 99/)).toBeVisible();
  await expectNoA11yViolations(page, 'legacy re-import diff');

  await page.getByRole('button', { name: 'Confirm re-import' }).click();
  await expect(page.getByRole('heading', { name: 'SQLiteSample2' })).toBeVisible({ timeout: 60_000 });
  await expect(page.getByRole('row', { name: /Oswego/ })).toContainText('v2');
});

test('reimporting_the_identical_file_is_a_noop_and_offers_only_close', async ({ page }) => {
  test.setTimeout(120_000);
  const projectId = await importSample(page);
  const before = await page.evaluate(async (pid) => {
    const datasets = await window.dataApi.listDatasets(pid);
    return datasets.map((d) => `${d.name}:v${d.currentVersion}`).sort();
  }, projectId);
  await page.goto(`/p/${projectId}/reimport`);
  await chooseReimportFile(page, SAMPLE_DB);

  await expect(page.getByText('no record or schema changes')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Confirm re-import' })).toHaveCount(0);
  const close = page.getByRole('link', { name: 'Close' });
  await expect(close).toBeVisible();
  await close.click();
  await expect(page.getByRole('heading', { name: 'SQLiteSample2' })).toBeVisible();
  const after = await page.evaluate(async (pid) => {
    const datasets = await window.dataApi.listDatasets(pid);
    return datasets.map((d) => `${d.name}:v${d.currentVersion}`).sort();
  }, projectId);
  expect(after).toEqual(before);
});

test('reimport_keeps_open_session_on_old_version', async ({ page }) => {
  test.setTimeout(120_000);
  const projectId = await importSample(page);
  const datasetId = await page.evaluate(async (pid) => {
    const datasets = await window.dataApi.listDatasets(pid);
    return datasets.find((d) => d.name === 'Oswego')!.id;
  }, projectId);
  const session = await page.evaluate(
    ({ pid, did }) => window.dataApi.openSession(pid, did, 1),
    { pid: projectId, did: datasetId },
  );
  const recipe = await page.evaluate(
    (sid) => window.dataApi.applyCommand(sid, 'SELECT AGE > 40'),
    session.sessionId,
  );
  expect(recipe.steps.length).toBeGreaterThan(0);
  const receiptBefore = await page.evaluate(async ({ pid, did }) => {
    const run = await window.dataApi.runFreq(pid, did, 1, {
      variables: ['AGE'],
      stratify_by: null,
      include_missing: false,
      ci_method: 'clopper_pearson',
    });
    return { json: JSON.stringify(run.receipt), sha256: run.receiptSha256, id: run.id };
  }, { pid: projectId, did: datasetId });

  await page.getByRole('link', { name: 'Re-import a newer copy' }).click();
  await chooseReimportFile(page, REIMPORT_DB);
  await page.getByRole('button', { name: 'Confirm re-import' }).click();
  await expect(page.getByText('Open working views stayed on their source versions')).toBeVisible();
  await expect(page.getByRole('button', { name: 'Move working views to new versions' })).toBeVisible();
  expect(session.sourceVersion).toBe(1);

  await page.evaluate(() => {
    const api = window.dataApi as unknown as Record<string, unknown>;
    const original = window.dataApi.moveSessionsToCurrent.bind(window.dataApi);
    api['moveSessionsToCurrent'] = (pid: string) => {
      const result = original(pid);
      (window as unknown as Record<string, unknown>)['__epioraMoveResult'] = result;
      return result;
    };
  });
  await page.getByRole('button', { name: 'Move working views to new versions' }).click();
  await expect(page.getByText('Working views moved to the new versions.')).toBeVisible();
  const moved = await page.evaluate(() =>
    (window as unknown as { __epioraMoveResult: Promise<{ fromVersion: number; toVersion: number; state: { steps: string[]; viewSha256: string } }[]> }).__epioraMoveResult,
  );
  expect(moved).toHaveLength(1);
  expect(moved[0]!.fromVersion).toBe(1);
  expect(moved[0]!.toVersion).toBe(2);
  expect(moved[0]!.state.steps).toEqual(recipe.steps);
  expect(moved[0]!.state.viewSha256).toMatch(/^[0-9a-f]{64}$/);

  const receiptAfter = await page.evaluate(
    async (runId) => {
      const runs = await window.dataApi.listRuns((await window.dataApi.listProjects())[0]!.id);
      const run = runs.find((r) => r.id === runId)!;
      return { json: JSON.stringify(run.receipt), sha256: run.receiptSha256 };
    },
    receiptBefore.id,
  );
  expect(receiptAfter).toEqual({ json: receiptBefore.json, sha256: receiptBefore.sha256 });
});


test('reimport_identity_guard_blocks_nonlegacy_and_mismatched_files', async ({ page }) => {
  test.setTimeout(120_000);
  const projectId = await importSample(page);
  const before = await page.evaluate(async (pid) => {
    const datasets = await window.dataApi.listDatasets(pid);
    return Promise.all(datasets.map(async (d) => (await window.dataApi.listVersions(d.id)).length));
  }, projectId);

  const nonLegacy = await page.evaluate(() => window.dataApi.createProject('ordinary').then((p) => p.id));
  const nonLegacyError = await page.evaluate(async (pid) => {
    return window.dataApi.planRefresh(pid, [new File([new Uint8Array([1])], 'SampleDB.db')]).then(
      () => null,
      (e) => ({ detail: (e as { detail?: string }).detail, message: (e as Error).message }),
    );
  }, nonLegacy);
  expect(nonLegacyError?.detail).toBe('reimport_not_legacy');
  await expect(page.evaluate((pid) => window.dataApi.listDatasets(pid).then((d) => d.length), nonLegacy)).resolves.toBe(0);

  await page.goto(`/p/${projectId}/reimport`);
  await chooseReimportFile(page, MISMATCH_DB);
  await expect(page.getByRole('alert')).toContainText('not a newer copy');
  const after = await page.evaluate(async (pid) => {
    const datasets = await window.dataApi.listDatasets(pid);
    return Promise.all(datasets.map(async (d) => (await window.dataApi.listVersions(d.id)).length));
  }, projectId);
  expect(after).toEqual(before);
});

test('offline_import_in_browser', async ({ page }, testInfo) => {
  test.setTimeout(120_000);
  const preview = await startPreview(testInfo.workerIndex);
  try {
    await page.goto(preview.url);
    await swControlled(page);
    await preview.proc.kill('SIGKILL');

    await page.goto(`${preview.url}/legacy`);
    await expect(page.getByRole('heading', { name: 'Open an Epi Info project' })).toBeVisible();
    await chooseFiles(page, [SAMPLE_DB]);
    await expect(page.getByRole('heading', { name: /Preview SQLiteSample2/ })).toBeVisible({ timeout: 60_000 });
    await page.getByRole('button', { name: 'Confirm import' }).click();
    await expect(page.getByRole('heading', { name: 'SQLiteSample2' })).toBeVisible({ timeout: 60_000 });
  } finally {
    preview.proc.kill('SIGKILL');
  }
});

test('browser_shows_unencrypted_storage_notice', async ({ page }) => {
  await page.goto('/legacy');
  await expect(page.getByText('Browser storage is not encrypted')).toBeVisible();
  await expect(page.getByText('identifiable data')).toBeVisible();
  await expect(page.getByRole('heading', { name: 'Connect to SQL Server' })).toHaveCount(0);
  await expectNoA11yViolations(page, 'browser storage notice');
});

test('keyboard_only_legacy_import', async ({ page }) => {
  test.setTimeout(90_000);
  await page.goto('/');
  const open = page.getByRole('link', { name: 'Open an Epi Info project' });
  await tabTo(page, open, 'Open an Epi Info project link');
  await page.keyboard.press('Enter');

  const choose = page.getByRole('button', { name: 'Choose project files' });
  await tabTo(page, choose, 'Choose project files button');
  const chooser = page.waitForEvent('filechooser');
  await page.keyboard.press('Enter');
  await (await chooser).setFiles([SAMPLE_DB]);

  const confirm = page.getByRole('button', { name: 'Confirm import' });
  await expect(confirm).toBeVisible({ timeout: 60_000 });
  await expectNoA11yViolations(page, 'legacy preview keyboard path');
  await tabTo(page, confirm, 'Confirm import button');
  await page.keyboard.press('Enter');
  await expect(page.getByRole('heading', { name: 'SQLiteSample2' })).toBeVisible({ timeout: 60_000 });
});
