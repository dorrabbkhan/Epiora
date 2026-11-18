// Analysis runs and the output log (v0.3 spec §5.2–5.3, §8.2): a run
// loads the dataset snapshot through the verified loader (the LRU of 2
// handles lives in snapshots.ts), calls `freq`/`tables`/`means` with a
// `RunContext` stamped by the worker's clock and naming the project —
// receipt v2 — and persists result + receipt + the Epi Info command to
// `runs`. `newerVersion` is the dataset's current version when it
// exceeds the run's. A pre-v2 row's NULL `command` is generated on load
// by `epiInfoCommand` and never written back (v1.0 Task 15).

import type { DatasetHandle } from '@engine/epiora_wasm';
import type { Explanation } from '@engine-types/Explanation';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { FreqResult } from '@engine-types/FreqResult';
import type { MeansOptions } from '@engine-types/MeansOptions';
import type { MeansResult } from '@engine-types/MeansResult';
import type { ProcOutput } from '@engine-types/ProcOutput';
import type { Receipt } from '@engine-types/Receipt';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { TablesResult } from '@engine-types/TablesResult';
import type { RunRecord } from './api';
import { engine, type Engine } from './engines';
import { rpc } from './errors';
import { loadHandle } from './snapshots';
import * as datasets from '../store/repo/datasets';
import * as projects from '../store/repo/projects';
import * as runs from '../store/repo/runs';
import type { Store } from '../store/types';

function toRunRecord(
  run: runs.Run,
  currentVersion: number,
  eng: Engine,
): RunRecord {
  return {
    id: run.id,
    projectId: run.projectId,
    datasetId: run.datasetId,
    version: run.datasetVersion,
    procedure: run.procedure as 'freq' | 'tables' | 'means',
    options: JSON.parse(run.options) as FreqOptions | TablesOptions | MeansOptions,
    result: JSON.parse(run.result) as FreqResult | TablesResult | MeansResult,
    receipt: JSON.parse(run.receipt) as Receipt,
    receiptSha256: run.receiptHash,
    command:
      run.command ??
      eng.epiInfoCommand(run.procedure, JSON.parse(run.options)),
    createdAt: run.createdAt,
    newerVersion: currentVersion > run.datasetVersion ? currentVersion : null,
  };
}

async function run(
  store: Store,
  projectId: string,
  datasetId: string,
  version: number,
  procedure: 'freq' | 'tables' | 'means',
  options: FreqOptions | TablesOptions | MeansOptions,
): Promise<RunRecord> {
  const project = (await projects.list(store)).find((p) => p.id === projectId);
  if (project === undefined) {
    throw rpc('not_found', `project ${projectId} does not exist`);
  }
  const dataset = await datasets.get(store, datasetId);
  if (dataset === null) {
    throw rpc('not_found', `dataset ${datasetId} does not exist`);
  }
  const handle: DatasetHandle = await loadHandle(store, datasetId, version);
  const eng = await engine();
  const ctx = {
    timestamp: new Date().toISOString(),
    project: { id: project.id, name: project.name },
  };
  const out =
    procedure === 'freq'
      ? (eng.freq(handle, options, ctx) as ProcOutput<FreqResult>)
      : procedure === 'tables'
        ? (eng.tables(handle, options, ctx) as ProcOutput<TablesResult>)
        : (eng.means(handle, options, ctx) as ProcOutput<MeansResult>);
  const command = eng.epiInfoCommand(procedure, options);
  const row = await runs.insert(store, {
    projectId,
    procedure,
    options: JSON.stringify(options),
    datasetId,
    datasetVersion: version,
    result: JSON.stringify(out.result),
    receipt: JSON.stringify(out.receipt),
    receiptHash: out.receipt.receipt_sha256,
    command,
    now: new Date().toISOString(),
  });
  return toRunRecord(row, dataset.currentVersion, eng);
}

export function runFreq(
  store: Store,
  projectId: string,
  datasetId: string,
  version: number,
  options: FreqOptions,
): Promise<RunRecord> {
  return run(store, projectId, datasetId, version, 'freq', options);
}

export function runTables(
  store: Store,
  projectId: string,
  datasetId: string,
  version: number,
  options: TablesOptions,
): Promise<RunRecord> {
  return run(store, projectId, datasetId, version, 'tables', options);
}

export function runMeans(
  store: Store,
  projectId: string,
  datasetId: string,
  version: number,
  options: MeansOptions,
): Promise<RunRecord> {
  return run(store, projectId, datasetId, version, 'means', options);
}

export async function listRuns(store: Store, projectId: string): Promise<RunRecord[]> {
  const current = new Map(
    (await datasets.listByProject(store, projectId)).map((d) => [d.id, d.currentVersion]),
  );
  const eng = await engine();
  return (await runs.listByProject(store, projectId)).map((r) =>
    toRunRecord(r, current.get(r.datasetId) ?? r.datasetVersion, eng),
  );
}

export async function rerun(store: Store, runId: string, version: number): Promise<RunRecord> {
  const old = await runs.get(store, runId);
  if (old === null || old.deletedAt !== null) {
    throw rpc('not_found', `run ${runId} does not exist`);
  }
  return run(
    store,
    old.projectId,
    old.datasetId,
    version,
    old.procedure as 'freq' | 'tables' | 'means',
    JSON.parse(old.options) as FreqOptions | TablesOptions | MeansOptions,
  );
}

// Explain (v1.0 plan Task 16, spec §6): the trace is computed on the
// run's stored dataset snapshot and options — the same inputs that
// produced the displayed leaf — so every number in the panel is one
// the engine returned. Traces are not part of the receipt.
export async function explainRun(
  store: Store,
  runId: string,
  statistic: string,
): Promise<Explanation> {
  const run = await runs.get(store, runId);
  if (run === null || run.deletedAt !== null) {
    throw rpc('not_found', `run ${runId} does not exist`);
  }
  const handle: DatasetHandle = await loadHandle(
    store,
    run.datasetId,
    run.datasetVersion,
  );
  const eng = await engine();
  const options = JSON.parse(run.options) as
    | FreqOptions
    | TablesOptions
    | MeansOptions;
  return eng.explainProcedure(
    handle,
    run.procedure,
    options,
    statistic,
  ) as Explanation;
}

export async function deleteRun(store: Store, runId: string): Promise<void> {
  if ((await runs.get(store, runId)) === null) {
    throw rpc('not_found', `run ${runId} does not exist`);
  }
  await runs.markDeleted(store, runId, new Date().toISOString());
}
