// Dataset screen e2e (v0.3 plan Task 10): the grid is seeded through
// `window.dataApi` — the app's own typed RPC client (exposed in
// main.tsx) — so the tests exercise the real import path.
import { readFileSync } from 'node:fs';
import { expect, test, type Page } from '@playwright/test';
import { expectNoA11yViolations } from './helpers/axe';

const UTF8_COMMA = readFileSync(
  new URL('../../fixtures/import/utf8_comma/input.csv', import.meta.url),
  'utf8',
);

interface Seed {
  projectId: string;
  datasetId: string;
}

/** Imports `csv` into a fresh project through the real RPC. */
async function seedCsv(
  page: Page,
  csv: string,
  fileName: string,
): Promise<Seed> {
  return page.evaluate(
    async ({ csv, fileName }) => {
      const api = window.dataApi;
      const project = await api.createProject('Seed project');
      const bytes = new TextEncoder().encode(csv);
      const { sessionId, inspection } = await api.inspectFile(
        fileName,
        bytes.buffer as ArrayBuffer,
      );
      const result = await api.commitImport(
        sessionId,
        { projectId: project.id, datasetName: fileName },
        inspection.settings,
        false,
        () => {},
      );
      return { projectId: project.id, datasetId: result.datasetId };
    },
    { csv, fileName },
  );
}

/** Re-imports a changed file onto the dataset, creating version 2. */
async function reimport(
  page: Page,
  datasetId: string,
  csv: string,
  fileName: string,
): Promise<void> {
  await page.evaluate(
    async ({ datasetId, csv, fileName }) => {
      const api = window.dataApi;
      const bytes = new TextEncoder().encode(csv);
      const { sessionId, inspection } = await api.inspectFile(
        fileName,
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
    { datasetId, csv, fileName },
  );
}

function datasetPath({ projectId, datasetId }: Seed): string {
  return `/p/${projectId}/d/${datasetId}`;
}

test('grid_shows_types_and_missing_counts', async ({ page }) => {
  await page.goto('/');
  const seed = await seedCsv(page, UTF8_COMMA, 'input.csv');
  await page.goto(datasetPath(seed));

  // utf8_comma: AGE integer [20, null], NAME text [alice, bob].
  const grid = page.getByRole('grid');
  const age = page.getByRole('columnheader', { name: /AGE/ });
  await expect(age).toContainText('integer');
  await expect(age).toContainText('1 missing');
  const name = page.getByRole('columnheader', { name: /NAME/ });
  await expect(name).toContainText('text');
  await expect(name).toContainText('0 missing');
  await expect(grid).toHaveAttribute('aria-rowcount', '2');
  await expectNoA11yViolations(page, 'dataset grid');
});

test('grid_is_navigable_by_arrow_keys', async ({ page }) => {
  await page.goto('/');
  const seed = await seedCsv(page, UTF8_COMMA, 'input.csv');
  await page.goto(datasetPath(seed));

  await expect(
    page.getByRole('gridcell', { name: 'alice' }),
  ).toBeVisible();
  await page.getByRole('gridcell', { name: '20' }).click();
  await page.keyboard.press('ArrowRight');
  await expect(page.getByRole('gridcell', { name: 'alice' })).toBeFocused();
  await page.keyboard.press('ArrowDown');
  await expect(page.getByRole('gridcell', { name: 'bob' })).toBeFocused();
  await expectNoA11yViolations(page, 'grid with focused cell');
});

test('version_selector_switches_rows', async ({ page }) => {
  await page.goto('/');
  const seed = await seedCsv(page, 'ID,V\na,1\nb,2\n', 'v1.csv');
  await reimport(page, seed.datasetId, 'ID,V\na,1\nb,3\nc,4\n', 'v2.csv');
  await page.goto(datasetPath(seed));

  // The current version (v2) shows the re-imported rows.
  await expect(page.getByRole('gridcell', { name: 'c' })).toBeVisible();
  await expect(page.getByRole('gridcell', { name: '3' })).toBeVisible();

  // Switching to version 1 shows the original rows.
  await page.getByRole('button', { name: /Version/ }).click();
  await expectNoA11yViolations(page, 'open version menu');
  await page.getByRole('menuitemradio', { name: /Version 1/ }).click();
  await expect(page.getByRole('gridcell', { name: 'c' })).toHaveCount(0);
  await expect(page.getByRole('gridcell', { name: '2' })).toBeVisible();
  await expectNoA11yViolations(page, 'dataset grid on version 1');
});

test('grid_scrolls_to_last_row_of_1m', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  // A tiny real import, then `getVersionPage` is patched to report a
  // 1,000,000-row dataset and serve synthetic `idN` rows for any
  // offset. An off-the-record e2e context cannot hold a real 1M-row
  // store (the perf job needs the persistent profile for exactly that
  // reason — SQLITE_IOERR), and the v0.3 renderer crash this guards
  // (run 44485826291, Target crashed / React error #520) is about the
  // row *count* the grid materialises, not the bytes behind it. The
  // real-store 1M scroll is covered by `grid_scroll_1m` in the perf
  // spec. The patch has to land before `main.tsx` assigns
  // `window.dataApi`, so it rides an init script — the setter shadows
  // `getVersionPage` on the client proxy the moment it is created.
  await page.addInitScript(() => {
    let api: (typeof window)['dataApi'] | undefined;
    Object.defineProperty(window, 'dataApi', {
      configurable: true,
      get: () => api,
      set: (v) => {
        const real = v.getVersionPage.bind(v) as (
          datasetId: string,
          version: number,
          offset: number,
          limit: number,
        ) => Promise<{
          columns: { name: string; type: string; missing: number }[];
          rows: (string | null)[][];
          total: number;
        }>;
        v.getVersionPage = async (
          datasetId: string,
          version: number,
          offset: number,
          limit: number,
        ) => {
          const meta = await real(datasetId, version, 0, 1);
          return {
            columns: meta.columns,
            total: 1_000_000,
            rows: Array.from(
              { length: Math.min(limit, 1_000_000 - offset) },
              (_, j) => [`id${offset + j}`, String(offset + j)],
            ),
          };
        };
        api = v;
      },
    });
  });
  const seed = await seedCsv(page, 'ID,V\nid0,0\n', 'one.csv');
  await page.goto(datasetPath(seed));

  const grid = page.getByRole('grid');
  await expect(grid).toHaveAttribute('aria-rowcount', '1000000', {
    timeout: 30_000,
  });
  await grid.evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  await expect(
    page.getByRole('gridcell', { name: 'id999999' }),
  ).toBeVisible({ timeout: 30_000 });
  await expectNoA11yViolations(page, 'scrolled 1m grid');
});

test('scrolls_to_last_row_of_10k', async ({ page }) => {
  test.setTimeout(120_000);
  await page.goto('/');
  const csv = [
    'ID,V',
    ...Array.from({ length: 10_000 }, (_, i) => `id${i},${i}`),
  ].join('\n');
  const seed = await seedCsv(page, csv, 'big.csv');
  await page.goto(datasetPath(seed));

  const grid = page.getByRole('grid');
  await expect(grid).toHaveAttribute('aria-rowcount', '10000', {
    timeout: 30_000,
  });
  await grid.evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  await expect(
    page.getByRole('gridcell', { name: 'id9999' }),
  ).toBeVisible({ timeout: 30_000 });
  await expectNoA11yViolations(page, 'scrolled 10k grid');
});
