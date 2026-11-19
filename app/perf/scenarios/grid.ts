// Grid scenarios — `grid_scroll_10k` (carryover from v0.3 Task 10:
// JS heap + frame time scrolling the 10,000-row grid end to end, one
// viewport per step) and `grid_scroll_1m` (the same pass on the
// larger dataset, a fixed step count). Split out of perf.spec.ts in
// v1.0 Task 18; behaviour unchanged.

import type { Page } from '@playwright/test';
import {
  importCsv,
  label,
  runScenario,
  scenarioEnabled,
  skipScenario,
  SMALL_LABEL,
} from './shared';
import type { SampleCtx } from './shared';

/** The grid scroll's viewport stride visits every row's scroll
 *  position; the big grid uses a fixed step count instead — the
 *  carryover asks for "a larger dataset", not for 10,000 page loads,
 *  and every step's page loads pay the `items`-array rebuild that
 *  scales with `total`. */
const BIG_GRID_STEPS = 150;

/** Wall-clock cap per scroll pass. A pass that hits it returns
 *  `{incomplete: 1, stepsDone}` — at scale the DataGrid's per-page
 *  `items` rebuild makes an unbounded pass run past the test timeout
 *  (observed locally: >30 min on the 100k grid). */
const SCROLL_BUDGET_MS = 300_000;

/** One scrolling pass over `[role="grid"]`, top to bottom. `steps = 0`
 *  strides one viewport (visits every scroll position); otherwise the
 *  pass is exactly `steps` evenly spaced scroll positions. Each step
 *  waits — bounded — for the bottom-most rendered row's first cell to
 *  fill, i.e. for its getVersionPage page to land.
 *
 *  A page crash (the DataGrid retains every loaded page and rebuilds
 *  the `items` array per page — the v0.3 Task 10 carryover) is a
 *  measurement, not a test failure: `{crashed: 1, stepsDone}` is
 *  recorded and the rest of the sample reports `page_crashed`. */
async function scrollGrid(
  page: Page,
  steps: number,
  budgetMs: number = SCROLL_BUDGET_MS,
): Promise<Record<string, number>> {
  let stepsDone = 0;
  let stepsPlanned = steps;
  let crashed = false;
  const onConsole = (msg: { text: () => string }) => {
    const m = /^perf-scroll-step (\d+)$/.exec(msg.text());
    if (m !== null) stepsDone = Number(m[1]);
    const p = /^perf-scroll-steps (\d+)$/.exec(msg.text());
    if (p !== null) stepsPlanned = Number(p[1]);
  };
  const onCrash = () => {
    crashed = true;
  };
  page.on('console', onConsole);
  page.on('crash', onCrash);
  try {
    return await page.evaluate(async ({ steps, budgetMs }) => {
      const grid = document.querySelector('[role="grid"]');
      if (!(grid instanceof HTMLElement)) throw new Error('data grid not found');
      const memory = (
        performance as unknown as { memory?: { usedJSHeapSize: number } }
      ).memory;
      const heapStart = memory?.usedJSHeapSize ?? 0;
      const raf = () => new Promise((r) => requestAnimationFrame(r));
      const frames: number[] = [];
      const t0 = performance.now();
      const maxTop = Math.max(0, grid.scrollHeight - grid.clientHeight);
      const n = steps > 0 ? steps : Math.max(1, Math.ceil(maxTop / grid.clientHeight));
      console.log(`perf-scroll-steps ${n}`);
      let done = 0;
      let incomplete = 0;
      for (let i = 1; i <= n; i++) {
        console.log(`perf-scroll-step ${i}`);
        grid.scrollTop = Math.round((maxTop * i) / n);
        await raf();
        const a = performance.now();
        await raf();
        frames.push(performance.now() - a);
        const deadline = performance.now() + 1000;
        for (;;) {
          const rows = [...grid.querySelectorAll('[role="row"]')];
          const firstCell = rows.at(-1)?.querySelector('[role="gridcell"]');
          const ready =
            rows.length > 1 &&
            firstCell !== null &&
            firstCell !== undefined &&
            firstCell.textContent !== '';
          if (ready || performance.now() >= deadline) break;
          await new Promise((r) => setTimeout(r, 25));
        }
        done = i;
        if (performance.now() - t0 > budgetMs) {
          incomplete = 1;
          break;
        }
      }
      const ms = performance.now() - t0;
      const heapEnd = memory?.usedJSHeapSize ?? 0;
      frames.sort((x, y) => x - y);
      const pick = (p: number) =>
        frames.length === 0
          ? 0
          : frames[Math.min(frames.length - 1, Math.floor(p * frames.length))]!;
      return {
        ms,
        stepsDone: done,
        steps: n,
        incomplete,
        heapStartBytes: heapStart,
        heapEndBytes: heapEnd,
        frameMedianMs: pick(0.5),
        frameP95Ms: pick(0.95),
        frames: frames.length,
        scrollHeightPx: maxTop,
      };
    }, { steps, budgetMs });
  } catch (e) {
    if (!crashed) throw e;
    return { crashed: 1, stepsDone, steps: stepsPlanned };
  } finally {
    page.off('console', onConsole);
    page.off('crash', onCrash);
  }
}

/** Waits for the grid's first populated body cell — the initial
 *  getVersionPage(0) has landed — before measuring. */
async function waitForGrid(page: Page): Promise<void> {
  await page.waitForFunction(() =>
    [...document.querySelectorAll('[role="gridcell"]')].some(
      (c) => c.textContent !== '',
    ),
  );
}

export async function runGrid(ctx: SampleCtx): Promise<void> {
  if (ctx.gatedOnly) return;
  let smallScroll: Record<string, unknown> | null = null;
  if (scenarioEnabled(`grid_scroll_${SMALL_LABEL}`)) {
    if (ctx.projectId === '') {
      skipScenario(ctx, `grid_scroll_${SMALL_LABEL}`, 'create_project failed');
    } else {
      smallScroll = await runScenario(ctx, `grid_scroll_${SMALL_LABEL}`, async () => {
        const small = await importCsv(ctx.page, '/perf-csv/small', {
          projectId: ctx.projectId,
          datasetName: 'perf-10k.csv',
        });
        await ctx.page.goto(`/p/${ctx.projectId}/d/${small.datasetId}`);
        await waitForGrid(ctx.page);
        return scrollGrid(ctx.page, 0);
      });
    }
  }

  if (!scenarioEnabled(`grid_scroll_${label}`)) return;
  if (ctx.datasetId === null) {
    skipScenario(ctx, `grid_scroll_${label}`, `import_${label} failed`);
  } else if (smallScroll !== null && 'crashed' in smallScroll) {
    skipScenario(ctx, `grid_scroll_${label}`, 'page_crashed');
  } else {
    const did = ctx.datasetId;
    await runScenario(ctx, `grid_scroll_${label}`, async () => {
      await ctx.page.goto(`/p/${ctx.projectId}/d/${did}`);
      await waitForGrid(ctx.page);
      return scrollGrid(ctx.page, BIG_GRID_STEPS);
    });
  }
}
