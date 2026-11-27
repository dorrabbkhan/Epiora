// The typed RPC surface (v0.3 spec §5.3, plan Task 9): `DataApi` is
// what the main thread sees; `api` is the object Comlink exposes from
// data.worker.ts. Store-touching methods run inside `withStore` — the
// one-connection mutex from store-backend.ts — while session and wasm
// calls (inspect, preview, cancel, discard) stay off it so a cancel
// can reach a running commit. Every method wires thrown errors through
// `throwWired` for revival on the client side.

import type { Explanation } from '@engine-types/Explanation';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { FreqResult } from '@engine-types/FreqResult';
import type { MeansOptions } from '@engine-types/MeansOptions';
import type { MeansResult } from '@engine-types/MeansResult';
import type { Receipt } from '@engine-types/Receipt';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { TablesResult } from '@engine-types/TablesResult';
import type { Note } from '@engine-types/Note';
import type { DiffSummary } from '@import-types/DiffSummary';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { Inspection } from '@import-types/Inspection';
import type { KeyCandidate } from '@import-types/KeyCandidate';
import type { RecordKey } from '@import-types/RecordKey';
import type { ValueType } from '@import-types/ValueType';
import * as analysis from './analysis';
import * as commit from './commit';
import { engine } from './engines';
import * as rexport from './export';
import { rpc } from './errors';
import { wire } from './rpc-wire';
import { legacyApi, type LegacyDataApi } from './api-legacy';
import * as importer from './importer';
import * as langWorker from './lang';
import * as langSessionWorker from './lang-session';
import type {
  DatasetVersion,
  ProgramEvent,
  ProgramRun,
  Session,
  SessionState,
} from './lang';
import type { LegacyPreview, PlainPreview, RefreshPreview } from './legacy';
import { clearSnapshotCache, gridPage, loadEntry, storedSettings } from './snapshots';
import { withStore } from './store-backend';
import * as datasets from '../store/repo/datasets';
import * as projects from '../store/repo/projects';
import * as versions from '../store/repo/versions';
import type { Project } from '../store/repo/projects';

export type { Project };
export type { LegacyProjectInfo, MovedSession } from './api-legacy';

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
  procedure: 'freq' | 'tables' | 'means';
  options: FreqOptions | TablesOptions | MeansOptions;
  result: FreqResult | TablesResult | MeansResult;
  receipt: Receipt;
  receiptSha256: string;
  /** The equivalent Epi Info command (v1.0 Task 15, spec §7.3):
   *  stored for new runs; generated on load by `epiInfoCommand` for
   *  rows written before store schema v2 — never written back. */
  command: string;
  /** The run's assumption notes (v1.0 Task 23, spec §6.1): stored at
   *  insert for new runs; `[]` on rows written before store schema
   *  v3 — the notes side channel did not exist when they ran. */
  notes: Note[];
  createdAt: string;
  newerVersion: number | null;
}

export interface StorageEstimate {
  usage: number;
  quota: number;
  persisted: boolean;
}

export type {
  DatasetVersion,
  LegacyPreview,
  ProgramEvent,
  ProgramRun,
  RefreshPreview,
  PlainPreview,
  Session,
  SessionState,
};

export interface DataApi extends LegacyDataApi {
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
  /** Cleaning sessions and program runs (v1.1 spec §5–6): session
   *  replays carry the project's stored `dataset_links` (Task 13b),
   *  and `CANCEL READ` closes the session's view, clearing the recipe. */
  openSession(
    projectId: string,
    datasetId: string,
    version: number,
  ): Promise<Session>;
  applyCommand(sessionId: string, commandText: string): Promise<SessionState>;
  undoLast(sessionId: string): Promise<SessionState>;
  runProgram(
    sessionId: string,
    text: string,
    onEvent: (e: ProgramEvent) => void,
  ): Promise<ProgramRun>;
  writeSession(sessionId: string): Promise<DatasetVersion>;
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
  runMeans(
    projectId: string,
    datasetId: string,
    version: number,
    options: MeansOptions,
  ): Promise<RunRecord>;
  listRuns(projectId: string): Promise<RunRecord[]>;
  rerun(runId: string, version: number): Promise<RunRecord>;
  explainRun(runId: string, statistic: string): Promise<Explanation>;
  /**
   * "Export to R" (v1.0 Task 17, spec §8.2): the stored run's
   * `analysis.R` + `data.csv` + `receipt.json` zipped in the worker —
   * the suggested filename's hash is the run's receipt hash prefix.
   */
  exportRun(
    runId: string,
  ): Promise<{ name: string; bytes: Uint8Array<ArrayBuffer> }>;
  deleteRun(runId: string): Promise<void>;
  engineVersion(): Promise<string>;
  storageEstimate(): Promise<StorageEstimate>;
  requestPersistence(): Promise<boolean>;
  /**
   * Test hook: arms a one-shot hold immediately before the next CSV or
   * legacy import commit's transaction boundary; the promise resolves
   * when the worker reaches it. Only functional in builds made with
   * `VITE_TEST_HOOKS=1`.
   */
  __holdNextImportCommit(): Promise<void>;
  /**
   * Test hook: releases the armed import commit hold. Only functional in
   * builds made with `VITE_TEST_HOOKS=1`.
   */
  __releaseImportCommitHold(): Promise<void>;
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
  /**
   * Test hook (v1.0 Task 15 e2e `old_run_gets_generated_command`):
   * nulls a run's stored command, leaving the row exactly as a pre-v2
   * database would — the card's command then has to come from
   * `epiInfoCommand` on load. Only functional in builds made with
   * `VITE_TEST_HOOKS=1`.
   */
  __clearRunCommand(runId: string): Promise<void>;
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
  ...legacyApi,
  openSession: wire((projectId, datasetId, version) =>
    withStore((s) => langWorker.openSession(s, projectId, datasetId, version)),
  ),
  applyCommand: wire((sessionId, commandText) =>
    withStore((s) => langWorker.applyCommand(s, sessionId, commandText)),
  ),
  undoLast: wire((sessionId) => withStore((s) => langWorker.undoLast(s, sessionId))),
  runProgram: wire((sessionId, text, onEvent) =>
    withStore((s) => langSessionWorker.runProgram(s, sessionId, text, onEvent)),
  ),
  writeSession: wire(
    (sessionId) => withStore((s) => langSessionWorker.writeSession(s, sessionId)),
  ),
  runFreq: wire((projectId, datasetId, version, options) =>
    withStore((s) => analysis.runFreq(s, projectId, datasetId, version, options)),
  ),
  runTables: wire((projectId, datasetId, version, options) =>
    withStore((s) => analysis.runTables(s, projectId, datasetId, version, options)),
  ),
  runMeans: wire((projectId, datasetId, version, options) =>
    withStore((s) => analysis.runMeans(s, projectId, datasetId, version, options)),
  ),
  listRuns: wire((projectId) => withStore((s) => analysis.listRuns(s, projectId))),
  rerun: wire((runId, version) => withStore((s) => analysis.rerun(s, runId, version))),
  explainRun: wire((runId, statistic) =>
    withStore((s) => analysis.explainRun(s, runId, statistic)),
  ),
  exportRun: wire((runId) => withStore((s) => rexport.exportRun(s, runId))),
  deleteRun: wire((runId) => withStore((s) => analysis.deleteRun(s, runId))),
  engineVersion: wire(async () => (await engine()).engineVersion()),
  storageEstimate: wire(async () => {
    const est = (await navigator.storage?.estimate?.()) ?? {};
    const persisted = (await navigator.storage?.persisted?.()) ?? false;
    return { usage: est.usage ?? 0, quota: est.quota ?? 0, persisted };
  }),
  requestPersistence: wire(async () => (await navigator.storage?.persist?.()) ?? false),
  __holdNextImportCommit: wire(() => {
    if (import.meta.env.VITE_TEST_HOOKS !== '1') {
      return Promise.reject(rpc('not_found', 'test hooks are disabled in this build'));
    }
    return importer.holdNextImportCommit();
  }),
  __releaseImportCommitHold: wire(() => {
    if (import.meta.env.VITE_TEST_HOOKS !== '1') {
      return Promise.reject(rpc('not_found', 'test hooks are disabled in this build'));
    }
    return importer.releaseImportCommitHold();
  }),
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
  __clearRunCommand: wire((runId) => {
    if (import.meta.env.VITE_TEST_HOOKS !== '1') {
      return Promise.reject(rpc('not_found', 'test hooks are disabled in this build'));
    }
    return withStore(async (s) => {
      await s.exec('UPDATE runs SET command = NULL WHERE id = ?', [runId]);
    });
  }),
};
