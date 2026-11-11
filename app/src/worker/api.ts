// The typed RPC surface (v0.3 spec §5.3, plan Task 9): `DataApi` is
// what the main thread sees; `api` is the object Comlink exposes from
// data.worker.ts. Store-touching methods run inside `withStore` — the
// one-connection mutex from store-backend.ts — while session and wasm
// calls (inspect, preview, cancel, discard) stay off it so a cancel
// can reach a running commit. Every method wires thrown errors through
// `throwWired` for revival on the client side.

import type { FreqOptions } from '@engine-types/FreqOptions';
import type { FreqResult } from '@engine-types/FreqResult';
import type { Receipt } from '@engine-types/Receipt';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { TablesResult } from '@engine-types/TablesResult';
import type { DiffSummary } from '@import-types/DiffSummary';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { Inspection } from '@import-types/Inspection';
import type { KeyCandidate } from '@import-types/KeyCandidate';
import type { RecordKey } from '@import-types/RecordKey';
import type { ValueType } from '@import-types/ValueType';
import * as analysis from './analysis';
import * as commit from './commit';
import { engine } from './engines';
import { rpc, throwWired } from './errors';
import * as importer from './importer';
import { clearSnapshotCache, gridPage, loadEntry, storedSettings } from './snapshots';
import { withStore } from './store-backend';
import * as datasets from '../store/repo/datasets';
import * as projects from '../store/repo/projects';
import * as versions from '../store/repo/versions';
import type { Project } from '../store/repo/projects';

export type { Project };

export interface DatasetInfo {
  id: string;
  projectId: string;
  name: string;
  /** The record-key choice (spec §4.4). */
  key: RecordKey | null;
  /** The settings saved at the last import. */
  settings: ImportSettings | null;
  currentVersion: number;
  createdAt: string;
}

export interface VersionInfo {
  datasetId: string;
  version: number;
  sourceName: string;
  sourceSha256: string;
  rowCount: number;
  contentHash: string;
  snapshotAvailable: boolean;
}

export interface GridPage {
  columns: { name: string; type: ValueType; missing: number }[];
  /** Display-formatted values (dates ISO, reals via `String`, i64
   *  decimal), null for missing. */
  rows: (string | null)[][];
  total: number;
}

export interface ImportProgress {
  stage: 'parse' | 'diff' | 'write' | 'snapshot';
  done: number;
  total: number;
}

export interface CommitResult {
  datasetId: string;
  /** `null` = unchanged file, no version created. */
  version: number | null;
}

export interface RunRecord {
  id: string;
  projectId: string;
  datasetId: string;
  version: number;
  procedure: 'freq' | 'tables';
  options: FreqOptions | TablesOptions;
  result: FreqResult | TablesResult;
  receipt: Receipt;
  receiptSha256: string;
  createdAt: string;
  newerVersion: number | null;
}

export interface StorageEstimate {
  usage: number;
  quota: number;
  persisted: boolean;
}

export interface DataApi {
  listProjects(): Promise<Project[]>;
  createProject(name: string): Promise<Project>;
  renameProject(id: string, name: string): Promise<void>;
  deleteProject(id: string): Promise<void>;
  listDatasets(projectId: string): Promise<DatasetInfo[]>;
  getDataset(id: string): Promise<DatasetInfo>;
  listVersions(datasetId: string): Promise<VersionInfo[]>;
  getVersionPage(
    datasetId: string,
    version: number,
    offset: number,
    limit: number,
  ): Promise<GridPage>;
  inspectFile(
    name: string,
    bytes: ArrayBuffer,
  ): Promise<{ sessionId: string; inspection: Inspection; keyCandidates: KeyCandidate[] }>;
  previewWithSettings(
    sessionId: string,
    settings: ImportSettings,
  ): Promise<{ inspection: Inspection; keyCandidates: KeyCandidate[] }>;
  diffAgainst(
    sessionId: string,
    datasetId: string,
    settings: ImportSettings,
  ): Promise<DiffSummary>;
  commitImport(
    sessionId: string,
    target: { projectId: string; datasetName: string } | { datasetId: string },
    settings: ImportSettings,
    acceptSchemaChanges: boolean,
    onProgress: (p: ImportProgress) => void,
  ): Promise<CommitResult>;
  cancelImport(sessionId: string): Promise<void>;
  discardSession(sessionId: string): Promise<void>;
  runFreq(
    projectId: string,
    datasetId: string,
    version: number,
    options: FreqOptions,
  ): Promise<RunRecord>;
  runTables(
    projectId: string,
    datasetId: string,
    version: number,
    options: TablesOptions,
  ): Promise<RunRecord>;
  listRuns(projectId: string): Promise<RunRecord[]>;
  rerun(runId: string, version: number): Promise<RunRecord>;
  deleteRun(runId: string): Promise<void>;
  engineVersion(): Promise<string>;
  storageEstimate(): Promise<StorageEstimate>;
  requestPersistence(): Promise<boolean>;
  /**
   * Test hook (Task 8's `Store.__simulateQuotaOnNextWrite`, Task 11
   * e2e): the store's next mutating statement fails `quota_exceeded`.
   * Only functional in builds made with `VITE_TEST_HOOKS=1`.
   */
  __simulateQuotaOnNextWrite(): Promise<void>;
  /**
   * Test hook (Task 16 perf): drops the snapshot LRU so a following
   * run pays the cold `loadEntry` — store read, hash check, decode.
   * Only functional in builds made with `VITE_TEST_HOOKS=1`.
   */
  __clearSnapshotCache(): Promise<void>;
}

function toDatasetInfo(d: datasets.Dataset): DatasetInfo {
  let key: RecordKey | null = null;
  try {
    key = JSON.parse(d.key) as RecordKey;
  } catch {
    key = null;
  }
  return {
    id: d.id,
    projectId: d.projectId,
    name: d.name,
    key,
    settings: storedSettings(d) as ImportSettings | null,
    currentVersion: d.currentVersion,
    createdAt: d.createdAt,
  };
}

function toVersionInfo(v: versions.DatasetVersion): VersionInfo {
  return { ...v };
}

/** Wraps a handler so rejections reach the client as a wired RpcError. */
const wire =
  <A extends unknown[], R>(f: (...args: A) => Promise<R>) =>
  (...args: A): Promise<R> =>
    f(...args).catch(throwWired);

const now = () => new Date().toISOString();

export const api: DataApi = {
  listProjects: wire(() => withStore((s) => projects.list(s))),
  createProject: wire((name) => withStore((s) => projects.create(s, name, now()))),
  renameProject: wire((id, name) => withStore((s) => projects.rename(s, id, name, now()))),
  deleteProject: wire((id) => withStore((s) => projects.remove(s, id))),
  listDatasets: wire((projectId) =>
    withStore(async (s) => (await datasets.listByProject(s, projectId)).map(toDatasetInfo)),
  ),
  getDataset: wire((id) =>
    withStore(async (s) => {
      const d = await datasets.get(s, id);
      if (d === null) throw rpc('not_found', `dataset ${id} does not exist`);
      return toDatasetInfo(d);
    }),
  ),
  listVersions: wire((datasetId) =>
    withStore(async (s) => (await versions.list(s, datasetId)).map(toVersionInfo)),
  ),
  getVersionPage: wire((datasetId, version, offset, limit) =>
    withStore(async (s) => {
      const entry = await loadEntry(s, datasetId, version);
      return gridPage(entry.bytes, offset, limit);
    }),
  ),
  inspectFile: wire((name, bytes) => importer.inspectFile(name, bytes)),
  previewWithSettings: wire((sessionId, settings) =>
    importer.previewWithSettings(sessionId, settings),
  ),
  diffAgainst: wire((sessionId, datasetId, settings) =>
    withStore((s) => importer.diffAgainst(s, sessionId, datasetId, settings)),
  ),
  commitImport: wire((sessionId, target, settings, accept, onProgress) =>
    withStore((s) =>
      commit.commitImport(s, sessionId, target, settings, accept, onProgress),
    ),
  ),
  cancelImport: wire((sessionId) => importer.cancelImport(sessionId)),
  discardSession: wire((sessionId) => importer.discardSession(sessionId)),
  runFreq: wire((projectId, datasetId, version, options) =>
    withStore((s) => analysis.runFreq(s, projectId, datasetId, version, options)),
  ),
  runTables: wire((projectId, datasetId, version, options) =>
    withStore((s) => analysis.runTables(s, projectId, datasetId, version, options)),
  ),
  listRuns: wire((projectId) => withStore((s) => analysis.listRuns(s, projectId))),
  rerun: wire((runId, version) => withStore((s) => analysis.rerun(s, runId, version))),
  deleteRun: wire((runId) => withStore((s) => analysis.deleteRun(s, runId))),
  engineVersion: wire(async () => (await engine()).engineVersion()),
  storageEstimate: wire(async () => {
    const est = (await navigator.storage?.estimate?.()) ?? {};
    const persisted = (await navigator.storage?.persisted?.()) ?? false;
    return { usage: est.usage ?? 0, quota: est.quota ?? 0, persisted };
  }),
  requestPersistence: wire(async () => (await navigator.storage?.persist?.()) ?? false),
  __simulateQuotaOnNextWrite: wire(() => {
    if (import.meta.env.VITE_TEST_HOOKS !== '1') {
      return Promise.reject(rpc('not_found', 'test hooks are disabled in this build'));
    }
    return withStore(async (s) => {
      s.__simulateQuotaOnNextWrite?.();
    });
  }),
  __clearSnapshotCache: wire(() => {
    if (import.meta.env.VITE_TEST_HOOKS !== '1') {
      return Promise.reject(rpc('not_found', 'test hooks are disabled in this build'));
    }
    clearSnapshotCache();
    return Promise.resolve();
  }),
};
