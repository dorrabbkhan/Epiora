// Analysis workspace e2e (v0.3 plan Task 12, spec §8.2): Oswego is
// seeded through `window.dataApi` like dataset.spec.ts, then the real
// dialogs and result cards are driven. Every visited state gets an
// axe pass.
import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';
import { formatNumber } from '../src/ui/screens/analysis/render-result';

const OSWEGO = readFileSync(
  new URL('../../fixtures/data/oswego.csv', import.meta.url),
  'utf8',
);
const OSWEGO_VANILLA = JSON.parse(
  readFileSync(
    new URL('../../fixtures/2x2/oswego_vanilla.json', import.meta.url),
    'utf8',
  ),
) as { expected: { odds_ratio: { estimate: { value: number } } } };

// One extra record turns the Oswego re-import into a v2.
const OSWEGO_PLUS = `${OSWEGO.trimEnd()}\n"Patient76",45,"Male",1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,"04/19/40 09:00:00",,"g76","P76"\n`;

interface Seed {
  projectId: string;
  datasetId: string;
}

/** Imports the Oswego CSV into a fresh project through the real RPC. */
async function seedOswego(page: Page, csv = OSWEGO): Promise<Seed> {
  return page.evaluate(async (csv) => {
    const api = window.dataApi;
    const project = await api.createProject('Oswego');
    const bytes = new TextEncoder().encode(csv);
    const { sessionId, inspection } = await api.inspectFile(
      'oswego.csv',
      bytes.buffer as ArrayBuffer,
    );
    const result = await api.commitImport(
      sessionId,
      { projectId: project.id, datasetName: 'oswego.csv' },
      inspection.settings,
      false,
      () => {},
    );
    return { projectId: project.id, datasetId: result.datasetId };
  }, csv);
}

async function reimport(page: Page, datasetId: string, csv: string): Promise<void> {
  await page.evaluate(
    async ({ datasetId, csv }) => {
      const api = window.dataApi;
      const bytes = new TextEncoder().encode(csv);
      const { sessionId, inspection } = await api.inspectFile(
        'oswego.csv',
        bytes.buffer as ArrayBuffer,
      );
      await api.commitImport(
        sessionId,
        { datasetId },
        inspection.settings,
        true,
        () => {},
      );
    },
    { datasetId, csv },
  );
}

async function openWorkspace(page: Page, seed: Seed): Promise<void> {
  await page.goto(`/p/${seed.projectId}/analysis`);
  await expect(
    page.getByRole('heading', { name: 'Analysis' }),
  ).toBeVisible();
}

async function openDialog(page: Page, name: 'FREQ' | 'TABLES'): Promise<void> {
  await page.getByRole('button', { name: 'Run procedure' }).click();
  await page.getByRole('menuitem', { name, exact: true }).click();
  await expect(
    page.getByRole('dialog', { name, exact: true }),
  ).toBeVisible();
}

/** Opens the Select whose accessible name matches `label` and picks
 *  the option with that exact name. The popover can open over the
 *  trigger, so it is opened with Enter (a pointer release on the
 *  trigger would activate whatever item landed under it). */
async function pick(page: Page, label: RegExp | string, option: string): Promise<void> {
  await page.getByRole('button', { name: label }).focus();
  await page.keyboard.press('Enter');
  const item = page.getByRole('option', { name: option, exact: true });
  await expect(item).toBeVisible();
  await item.click();
}

/** RAC checkboxes resolve to a visually hidden input; the label text
 *  is the click target (same pattern as import.spec.ts). */
async function check(page: Page, dialog: string, name: string): Promise<void> {
  await page
    .getByRole('dialog', { name: dialog })
    .getByText(name, { exact: true })
    .click();
}

/** The state `runTables` leaves behind when it is replaced by a held
 *  promise (see `holdNextTablesRun`). */
interface HeldRun {
  orig: (...args: unknown[]) => Promise<unknown>;
  args: unknown[];
  resolve: (value: unknown) => void;
  reject: (error: unknown) => void;
}

/** Replaces `dataApi.runTables` with a promise this test resolves:
 *  the dialog's `aria-busy` pending state can then be held open
 *  deterministically for an axe pass. `releaseTablesRun` restores the
 *  real method and resolves with a real engine result. */
async function holdNextTablesRun(page: Page): Promise<void> {
  await page.evaluate(() => {
    const w = window as unknown as {
      dataApi: { runTables: unknown };
      __heldRun?: HeldRun;
    };
    const orig = w.dataApi.runTables as HeldRun['orig'];
    w.dataApi.runTables = (...args: unknown[]) =>
      new Promise<unknown>((resolve, reject) => {
        w.__heldRun = { orig, args, resolve, reject };
      });
  });
}

async function releaseTablesRun(page: Page): Promise<void> {
  await page.evaluate(() => {
    const w = window as unknown as {
      dataApi: { runTables: unknown };
      __heldRun: HeldRun;
    };
    const held = w.__heldRun;
    w.dataApi.runTables = held.orig;
    void held.orig(...held.args).then(held.resolve, held.reject);
  });
}

test('tables_vanilla_ill_matches_engine', async ({ page }) => {
  await page.goto('/');
  const seed = await seedOswego(page);
  await openWorkspace(page, seed);
  await expectNoA11yViolations(page, 'analysis workspace');

  await page.getByRole('button', { name: 'Run procedure' }).click();
  await expectNoA11yViolations(page, 'procedure menu open');
  await page.getByRole('menuitem', { name: 'TABLES', exact: true }).click();
  const dialog = page.getByRole('dialog', { name: 'TABLES', exact: true });
  await expect(dialog).toBeVisible();
  await expectNoA11yViolations(page, 'TABLES dialog');

  await holdNextTablesRun(page);
  await pick(page, /Exposure variable/, 'VANILLA');
  await pick(page, /Outcome variable/, 'ILL');
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  // RAC Dialog strips `aria-busy`; the pending state rides on the
  // dialog's form element (held here by the patched RPC).
  await expect(dialog.locator('[aria-busy="true"]')).toHaveCount(1);
  await expectNoA11yViolations(page, 'TABLES dialog pending');
  await releaseTablesRun(page);

  const expected = formatNumber(
    OSWEGO_VANILLA.expected.odds_ratio.estimate.value,
  );
  await expect(
    page.getByRole('heading', { name: 'TABLES VANILLA × ILL — oswego.csv v1' }),
  ).toBeVisible();
  await expect(page.getByRole('cell', { name: expected })).toBeVisible();
});

test('project_empty_log_offers_analysis_link', async ({ page }) => {
  await page.goto('/');
  const project = await page.evaluate(() =>
    window.dataApi.createProject('Empty project'),
  );
  await page.goto(`/p/${project.id}`);
  const log = page.getByRole('region', { name: 'Output log' });
  await expect(log.getByText('No results yet.')).toBeVisible();
  await expectNoA11yViolations(page, 'project screen, empty output log');
  await log.getByRole('link', { name: 'Run an analysis' }).click();
  await expect(
    page.getByRole('heading', { name: 'Analysis' }),
  ).toBeVisible();
});

test('freq_runs_and_persists_after_reload', async ({ page }) => {
  await page.goto('/');
  const seed = await seedOswego(page);
  await openWorkspace(page, seed);
  await openDialog(page, 'FREQ');
  await expectNoA11yViolations(page, 'FREQ dialog');

  await check(page, 'FREQ', 'ILL');
  await page.getByRole('button', { name: 'Run', exact: true }).click();

  await expect(
    page.getByRole('heading', { name: 'FREQ ILL — oswego.csv v1' }),
  ).toBeVisible();

  await page.reload();
  await expect(
    page.getByRole('heading', { name: 'FREQ ILL — oswego.csv v1' }),
  ).toBeVisible();
});

test('receipt_copy_json_equals_stored_receipt', async ({ page }) => {
  await page.addInitScript(() => {
    const w = window as unknown as { __copied: string[] };
    w.__copied = [];
    navigator.clipboard.writeText = (text: string) => {
      w.__copied.push(text);
      return Promise.resolve();
    };
  });
  await page.goto('/');
  const seed = await seedOswego(page);
  await openWorkspace(page, seed);
  await openDialog(page, 'FREQ');
  await check(page, 'FREQ', 'ILL');
  await page.getByRole('button', { name: 'Run', exact: true }).click();

  const card = page.getByRole('article', {
    name: 'FREQ ILL — oswego.csv v1',
  });
  await expect(card).toBeVisible();
  await card.getByRole('button', { name: 'Receipt' }).click();
  await expectNoA11yViolations(page, 'expanded receipt');
  await card.getByRole('button', { name: 'Copy receipt JSON' }).click();

  const copied = await page.evaluate(
    () => (window as unknown as { __copied: string[] }).__copied.at(-1),
  );
  const stored = await page.evaluate(async (projectId) => {
    const runs = await window.dataApi.listRuns(projectId);
    return JSON.stringify(runs[0]?.receipt);
  }, seed.projectId);
  expect(copied).toBe(stored);
});

test('rerun_on_newer_version_keeps_old_card', async ({ page }) => {
  await page.goto('/');
  const seed = await seedOswego(page);
  await openWorkspace(page, seed);
  await openDialog(page, 'TABLES');
  await pick(page, /Exposure variable/, 'VANILLA');
  await pick(page, /Outcome variable/, 'ILL');
  await page.getByRole('button', { name: 'Run', exact: true }).click();
  await expect(
    page.getByRole('heading', { name: 'TABLES VANILLA × ILL — oswego.csv v1' }),
  ).toBeVisible();

  await reimport(page, seed.datasetId, OSWEGO_PLUS);
  await page.reload();

  const oldCard = page.getByRole('article', {
    name: 'TABLES VANILLA × ILL — oswego.csv v1',
  });
  await expect(oldCard).toBeVisible();
  await oldCard.getByRole('button', { name: 'Rerun on v2' }).click();

  await expect(
    page.getByRole('heading', { name: 'TABLES VANILLA × ILL — oswego.csv v2' }),
  ).toBeVisible();
  await expect(oldCard).toBeVisible();
});

test('delete_card_removes_it_after_reload', async ({ page }) => {
  await page.goto('/');
  const seed = await seedOswego(page);
  await openWorkspace(page, seed);
  await openDialog(page, 'FREQ');
  await check(page, 'FREQ', 'ILL');
  await page.getByRole('button', { name: 'Run', exact: true }).click();

  const card = page.getByRole('article', {
    name: 'FREQ ILL — oswego.csv v1',
  });
  await expect(card).toBeVisible();
  await card.getByRole('button', { name: 'Delete result' }).click();
  await expect(card).toHaveCount(0);

  await page.reload();
  await expect(
    page.getByRole('heading', { name: 'FREQ ILL — oswego.csv v1' }),
  ).toHaveCount(0);
});

test('engine_error_shows_inline_with_retry', async ({ page }) => {
  await page.goto('/');
  const seed = await seedOswego(page);
  await openWorkspace(page, seed);
  await openDialog(page, 'TABLES');

  await pick(page, /Exposure variable/, 'SEX');
  await pick(page, /Outcome variable/, 'ILL');
  // The VITE_TEST_HOOKS build leaves the SEX level pickers empty;
  // running sends `levels: null` and the engine refuses the guess.
  await page.getByRole('button', { name: 'Run', exact: true }).click();

  const alert = page.getByRole('alert');
  await expect(alert).toContainText('The variable levels are ambiguous.');
  await alert.getByRole('button', { name: 'Retry' }).click();
  await expect(alert).toContainText('The variable levels are ambiguous.');
});

test('text_levels_picker_runs_sex_by_ill', async ({ page }) => {
  await page.goto('/');
  const seed = await seedOswego(page);
  await openWorkspace(page, seed);
  await openDialog(page, 'TABLES');

  await pick(page, /Exposure variable/, 'SEX');
  await pick(page, /SEX first level/, 'Female');
  await pick(page, /SEX second level/, 'Male');
  await pick(page, /Outcome variable/, 'ILL');
  await page.getByRole('button', { name: 'Run', exact: true }).click();

  await expect(
    page.getByRole('heading', { name: 'TABLES SEX × ILL — oswego.csv v1' }),
  ).toBeVisible();
});

test('result_is_announced', async ({ page }) => {
  await page.goto('/');
  const seed = await seedOswego(page);
  await openWorkspace(page, seed);
  await openDialog(page, 'TABLES');
  await pick(page, /Exposure variable/, 'VANILLA');
  await pick(page, /Outcome variable/, 'ILL');
  await page.getByRole('button', { name: 'Run', exact: true }).click();

  await expect(page.getByRole('status')).toContainText(
    'TABLES VANILLA × ILL — oswego.csv v1',
  );
});
