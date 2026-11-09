// Analysis runs and the output log (v0.3 spec §5.2–5.3, §8.2): a run
// loads the dataset snapshot through the verified loader (the LRU of 2
// handles lives in snapshots.ts), calls `freq`/`tables` with a
// `RunContext` stamped by the worker's clock and naming the project —
// receipt v2 — and persists result + receipt to `runs`. `newerVersion`
// is the dataset's current version when it exceeds the run's.

import type { DatasetHandle } from '@engine/epiora_wasm';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { FreqResult } from '@engine-types/FreqResult';
import type { ProcOutput } from '@engine-types/ProcOutput';
import type { Receipt } from '@engine-types/Receipt';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { TablesResult } from '@engine-types/TablesResult';
import type { RunRecord } from './api';
import { engine } from './engines';
import { rpc } from './errors';
import { loadHandle } from './snapshots';
import * as datasets from '../store/repo/datasets';
import * as projects from '../store/repo/projects';
import * as runs from '../store/repo/runs';
import type { Store } from '../store/types';

function toRunRecord(run: runs.Run, currentVersion: number): RunRecord {
  return {
    id: run.id,
    projectId: run.projectId,
    datasetId: run.datasetId,
    version: run.datasetVersion,
    procedure: run.procedure as 'freq' | 'tables',
    options: JSON.parse(run.options) as FreqOptions | TablesOptions,
    result: JSON.parse(run.result) as FreqResult | TablesResult,
    receipt: JSON.parse(run.receipt) as Receipt,
    receiptSha256: run.receiptHash,
    createdAt: run.createdAt,
    newerVersion: currentVersion > run.datasetVersion ? currentVersion : null,
  };
}

async function run(
  store: Store,
  projectId: string,
  datasetId: string,
  version: number,
  procedure: 'freq' | 'tables',
  options: FreqOptions | TablesOptions,
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
      : (eng.tables(handle, options, ctx) as ProcOutput<TablesResult>);
  const row = await runs.insert(store, {
    projectId,
    procedure,
    options: JSON.stringify(options),
    datasetId,
    datasetVersion: version,
    result: JSON.stringify(out.result),
    receipt: JSON.stringify(out.receipt),
    receiptHash: out.receipt.receipt_sha256,
    now: new Date().toISOString(),
  });
  return toRunRecord(row, dataset.currentVersion);
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

export async function listRuns(store: Store, projectId: string): Promise<RunRecord[]> {
  const current = new Map(
    (await datasets.listByProject(store, projectId)).map((d) => [d.id, d.currentVersion]),
  );
  return (await runs.listByProject(store, projectId)).map((r) =>
    toRunRecord(r, current.get(r.datasetId) ?? r.datasetVersion),
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
    old.procedure as 'freq' | 'tables',
    JSON.parse(old.options) as FreqOptions | TablesOptions,
  );
}

export async function deleteRun(store: Store, runId: string): Promise<void> {
  if ((await runs.get(store, runId)) === null) {
    throw rpc('not_found', `run ${runId} does not exist`);
  }
  await runs.markDeleted(store, runId, new Date().toISOString());
}
