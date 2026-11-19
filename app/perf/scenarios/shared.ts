// Shared plumbing for the perf spec's scenario modules — the
// environment knobs, the generated-CSV fixtures, the results.jsonl
// `record()`/`runScenario()`/`skipScenario()` plumbing and the
// page-side failure diagnostics. Split out of perf.spec.ts in v1.0
// Task 18 so every perf file stays under ~400 lines; the moved code
// is verbatim, so scenario behaviour is unchanged.

import { execFileSync } from 'node:child_process';
import {
  appendFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import type { Page } from '@playwright/test';
import { batchId } from '../gate-eval';

export const PERF_DIR = dirname(fileURLToPath(import.meta.url));
const WORK = fileURLToPath(new URL('../../../target/perf', import.meta.url));
const GEN = resolve(PERF_DIR, '..', 'gen-csv.mjs');

export const ROWS = Number(process.env.PERF_ROWS ?? 1_000_000);
export const RUNS = Number(process.env.PERF_RUNS ?? 1);
export const PROFILE =
  process.env.PERF_PROFILE === 'ephemeral' ? 'ephemeral' : 'persistent';
// D7 report (plan Task 17): PERF_CPU_THROTTLE=4 measures both gated
// scenarios under Chromium CPU throttling ×4. Throttled samples record
// `*_cpu<N>` scenario names so they can never contaminate the gated
// `import_1m` / `open_tables_1m` medians.
export const CPU_THROTTLE = Number(process.env.PERF_CPU_THROTTLE ?? 1);
export const BIG_CSV =
  process.env.PERF_CSV ?? resolve(WORK, `perf-${ROWS}.csv`);
export const SMALL_CSV =
  process.env.PERF_CSV_10K ?? resolve(WORK, 'perf-10000.csv');
export const EDITED_CSV = `${BIG_CSV}.edited.csv`;
export const OUT = process.env.PERF_OUT ?? resolve(WORK, 'results.jsonl');
export const GATES_MD = resolve(dirname(OUT), 'gates.md');

// Every recorded line carries the invocation's batch id so the gate
// test evaluates this run's samples only — a stale results.jsonl
// from an earlier local run can't pass or fail it. See batchId in
// ../gate-eval.ts for why the id is the runner's (parent's) pid, not
// the worker's.
export const BATCH = batchId(process.env, process);

/** `1m` / `250k` / raw number — keeps scenario names honest at any
 *  PERF_ROWS. At the CI default the names are exactly the plan's
 *  `import_1m` / `open_tables_1m` / `means_1m`. */
export const label =
  ROWS === 1_000_000 ? '1m' : ROWS % 1000 === 0 ? `${ROWS / 1000}k` : String(ROWS);

export const SMALL_LABEL = '10k';

// Scenario filter (owner instruction, v1.0 Task 19, 2026-11-19 —
// "much faster from now on"): PERF_SCENARIOS is a comma-separated
// list of scenario names (the recorded `1m` names at CI defaults;
// `*_${label}` at other PERF_ROWS) from the app.yml `scenarios`
// dispatch input. Empty/unset — the push-to-main case — runs
// everything. A scenario another needs as its fixture still runs:
// `import_*` is the fixture for `reimport_*`, `open_tables_*`,
// `means_*` and `grid_scroll_*` on the big CSV. A scenario the filter
// leaves out is not run and records nothing at all — no skip line —
// and the gate reports it `not run (filtered)`.
const SCENARIO_LIST = (process.env.PERF_SCENARIOS ?? '')
  .split(',')
  .map((s) => s.trim())
  .filter((s) => s !== '');
export const SCENARIOS_FILTERED = SCENARIO_LIST.length > 0;
const SCENARIO_FIXTURES: Record<string, string[]> = {
  [`reimport_${label}`]: [`import_${label}`],
  [`open_tables_${label}`]: [`import_${label}`],
  [`means_${label}`]: [`import_${label}`],
  [`grid_scroll_${label}`]: [`import_${label}`],
};
const enabledScenarios = new Set(SCENARIO_LIST);
for (const s of [...enabledScenarios]) {
  for (const f of SCENARIO_FIXTURES[s] ?? []) enabledScenarios.add(f);
}
/** Whether the scenario named by its un-suffixed base name (no `_cpuN`
 *  tag) runs in this invocation. Always true unfiltered. */
export const scenarioEnabled = (name: string): boolean =>
  !SCENARIOS_FILTERED || enabledScenarios.has(name);

export function ensureCsv(path: string, rows: number): void {
  if (!existsSync(path)) {
    mkdirSync(dirname(path), { recursive: true });
    execFileSync('node', [GEN, String(rows), path], { stdio: 'inherit' });
  }
}

export function ensureEditedCsv(): void {
  if (existsSync(EDITED_CSV)) return;
  // Carryover (v0.3 Task 9): "the biggest generated CSV with one
  // field edited" — bump the first data row's age by one so the
  // re-import diff reports a single updated record while `records`
  // is still fully rewritten.
  const csv = readFileSync(BIG_CSV, 'utf8');
  const nl = csv.indexOf('\n');
  const header = csv.slice(0, nl);
  const rest = csv.slice(nl + 1);
  const eol = rest.indexOf('\n');
  const fields = rest.slice(0, eol).split(',');
  fields[8] = String(Number(fields[8]) + 1); // age
  writeFileSync(EDITED_CSV, `${header}\n${fields.join(',')}\n${rest.slice(eol + 1)}`);
}

export function record(entry: Record<string, unknown>): void {
  mkdirSync(dirname(OUT), { recursive: true });
  appendFileSync(
    OUT,
    `${JSON.stringify({ profile: PROFILE, cpu: `x${CPU_THROTTLE}`, batch: BATCH, ...entry })}\n`,
  );
}

/** Scenario name — carries the throttle tag (`import_1m_cpu4`) so the
 *  D7 pass's samples stay out of the gated medians. */
export const scen = (name: string): string =>
  CPU_THROTTLE > 1 ? `${name}_cpu${CPU_THROTTLE}` : name;

/** Streams a generated CSV to the page as `fetch(url)` — Playwright's
 *  route fulfilment reads from disk, so the ~80 MB never crosses the
 *  protocol as JSON. */
export async function routeCsv(page: Page): Promise<void> {
  await page.route('**/perf-csv/big', (r) => r.fulfill({ path: BIG_CSV }));
  await page.route('**/perf-csv/edited', (r) => r.fulfill({ path: EDITED_CSV }));
  await page.route('**/perf-csv/small', (r) => r.fulfill({ path: SMALL_CSV }));
}

/** inspectFile (untimed) + timed commitImport of `url` — the plan's
 *  `import_*` definition is "time from commitImport call to resolve".
 *  The progress callback writes `stage done/total` onto
 *  `window.__perfProgress` so a failure mid-commit can report how far
 *  the transaction got. */
export async function importCsv(
  page: Page,
  url: string,
  target: { projectId: string; datasetName: string } | { datasetId: string },
): Promise<{ datasetId: string; ms: number }> {
  return page.evaluate(
    async ({ url, target }) => {
      const api = window.dataApi;
      const w = window as unknown as { __perfProgress?: string };
      w.__perfProgress = '';
      const bytes = await (await fetch(url)).arrayBuffer();
      const { sessionId, inspection } = await api.inspectFile('perf.csv', bytes);
      const t0 = performance.now();
      const result = await api.commitImport(
        sessionId,
        target,
        inspection.settings,
        false,
        (p) => {
          w.__perfProgress = `${p.stage} ${p.done}/${p.total}`;
        },
      );
      return { datasetId: result.datasetId, ms: performance.now() - t0 };
    },
    { url, target },
  );
}

/** `<name>/<code>/<detail>/<resultCode>: <message>` — RpcError keeps
 *  `code`/`detail` across the Comlink wire, a sqlite error keeps
 *  `resultCode`; whichever survived is named. */
export function errorText(e: unknown): string {
  if (!(e instanceof Error)) return String(e);
  const x = e as { code?: unknown; detail?: unknown; resultCode?: unknown };
  const tag = [e.name, x.code, x.detail, x.resultCode]
    .filter((v): v is string | number => typeof v === 'string' || typeof v === 'number')
    .join('/');
  return `${tag}: ${e.message}`;
}

/** Samples OPFS usage + sahpool file sizes every 5 s into `log`. The
 *  curve is the evidence for where an IOERR stops (journal vs db vs
 *  quota): after a rollback the journal is gone, so only in-flight
 *  samples show what the transaction had grown to. */
export function startOpfsPoller(page: Page, log: string[]): { stop: () => void } {
  const t0 = Date.now();
  const timer = setInterval(() => {
    void page
      .evaluate(async () => {
        const est = (await navigator.storage?.estimate?.()) ?? {};
        const parts: string[] = [`usage=${est.usage ?? '?'}`];
        try {
          const dir = await (
            await navigator.storage.getDirectory()
          ).getDirectoryHandle('.opfs-sahpool');
          const opaque = await dir.getDirectoryHandle('.opaque');
          for await (const [, handle] of opaque.entries()) {
            if (handle.kind !== 'file') continue;
            let size: number | string = '?';
            let assoc = '';
            try {
              const file = await (
                handle as FileSystemFileHandle
              ).getFile();
              size = file.size;
              const head = new Uint8Array(
                await file.slice(0, 512).arrayBuffer(),
              );
              const nul = head.indexOf(0);
              assoc = new TextDecoder().decode(
                head.subarray(0, nul === -1 ? head.length : nul),
              );
            } catch {
              // SAH-held files can refuse getFile(); name still lists.
            }
            if (assoc !== '') parts.push(`${assoc}=${size}`);
          }
        } catch {
          // The pool directory may not exist yet.
        }
        return parts.join(' ');
      })
      .then((s) =>
        log.push(`t+${Math.round((Date.now() - t0) / 1000)}s ${s}`),
      )
      .catch(() => {});
  }, 5000);
  return { stop: () => clearInterval(timer) };
}

/** Page-side evidence for a failed scenario: the last commitImport
 *  progress stage, storage usage/quota, the OPFS growth curve sampled
 *  while the transaction ran, and every OPFS file with its size — for
 *  `.opfs-sahpool` files the associated sqlite path (db, journal, …)
 *  is decoded from the 512-byte name field the VFS writes at offset 0.
 *  The worker's console tail rides along too: the sahpool VFS
 *  `console.error`s the DOMException a failed OPFS op threw, which is
 *  the only place the underlying error name still exists once the RPC
 *  layer has mapped it to `sql`. */
export async function failureDiagnostics(
  page: Page,
  consoleLines: string[],
  growthLog: string[],
): Promise<Record<string, unknown>> {
  const diag: Record<string, unknown> = {
    ua: await page
      .evaluate(() => navigator.userAgent)
      .catch(() => '(unavailable)'),
    growth: growthLog.join(' | ') || '(none sampled)',
    workerConsole:
      consoleLines.slice(-25).join(' | ') || '(none captured)',
  };
  growthLog.length = 0;
  try {
    Object.assign(
      diag,
      await page.evaluate(async () => {
        const out: Record<string, unknown> = {};
        out.progress =
          (window as unknown as { __perfProgress?: string }).__perfProgress ??
          null;
        const est = (await navigator.storage?.estimate?.()) ?? {};
        out.usageBytes = est.usage ?? null;
        out.quotaBytes = est.quota ?? null;
        try {
          const root = await navigator.storage.getDirectory();
          const files: string[] = [];
          const walk = async (
            dir: FileSystemDirectoryHandle,
            prefix: string,
          ): Promise<void> => {
            for await (const [name, handle] of dir.entries()) {
              if (handle.kind === 'directory') {
                await walk(
                  handle as FileSystemDirectoryHandle,
                  `${prefix}${name}/`,
                );
              } else {
                const file = await (
                  handle as FileSystemFileHandle
                ).getFile();
                let assoc = '';
                try {
                  const head = new Uint8Array(
                    await file.slice(0, 512).arrayBuffer(),
                  );
                  const nul = head.indexOf(0);
                  assoc = new TextDecoder().decode(
                    head.subarray(0, nul === -1 ? head.length : nul),
                  );
                } catch {
                  // Name field is best-effort; size alone still helps.
                }
                files.push(
                  `${prefix}${name}${assoc === '' ? '' : ` (${assoc})`}=${file.size}`,
                );
              }
            }
          };
          await walk(root, '');
          out.opfs = files.join('; ') || '(empty)';
        } catch (e) {
          out.opfs = `(unavailable: ${e instanceof Error ? e.message : String(e)})`;
        }
        return out;
      }),
    );
  } catch (e) {
    diag.inPageDiag = `unavailable: ${errorText(e)}`;
  }
  return diag;
}

/** Everything a scenario module needs from one sample: the page, the
 *  1-based run index, the console/OPFS tails failure diagnostics
 *  reads (and clears), the D7 `gatedOnly` flag, and the ids earlier
 *  scenarios fill in for the dependent ones. */
export interface SampleCtx {
  page: Page;
  run: number;
  consoleLines: string[];
  growthLog: string[];
  projectId: string;
  datasetId: string | null;
  /** The D7 throttled pass measures only the two gated scenarios. */
  gatedOnly: boolean;
}

/** A scenario throwing is a measurement, not a test failure: the
 *  error plus the page-side diagnostics the first dispatch lacked
 *  are recorded and independent scenarios still run. Returns the
 *  recorded entry, or null when the scenario failed. */
export async function runScenario(
  ctx: SampleCtx,
  scenario: string,
  fn: () => Promise<Record<string, unknown> | undefined>,
): Promise<Record<string, unknown> | null> {
  const t0 = Date.now();
  try {
    const entry = { run: ctx.run, scenario, ...((await fn()) ?? {}) };
    record(entry);
    return entry;
  } catch (e) {
    record({
      run: ctx.run,
      scenario,
      error: errorText(e),
      elapsedMs: Date.now() - t0,
      ...(await failureDiagnostics(ctx.page, ctx.consoleLines, ctx.growthLog)),
    });
    return null;
  }
}

/** A dependent scenario that never ran records its reason. */
export function skipScenario(
  ctx: SampleCtx,
  scenario: string,
  reason: string,
): void {
  record({ run: ctx.run, scenario, skipped: reason });
}
