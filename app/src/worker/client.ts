// Main-thread client (v0.3 spec §5.2–5.3): `dataApi` is the typed
// `Remote<DataApi>` the UI uses. `wrapDataApi` does three things the
// raw Comlink proxy cannot: revives wired error objects into real
// `RpcError`s (Comlink drops custom properties off thrown Errors),
// `Comlink.proxy`s the `commitImport` progress callback, and hides the
// worker plumbing. Tests use `wrapDataApi` to drive workers with
// per-test database names.

import * as Comlink from 'comlink';
import type { Remote } from 'comlink';
import type { DataApi, ImportProgress } from './api';
import { isWiredRpcError, RpcError } from './errors';

/** Awaits `p`, reviving a wired RpcError into the real class. */
async function call<T>(p: Promise<T>): Promise<T> {
  try {
    return await p;
  } catch (e) {
    if (isWiredRpcError(e)) throw new RpcError(e.code, e.detail, e.message);
    throw e;
  }
}

export function wrapDataApi(worker: Worker): Remote<DataApi> {
  const r = Comlink.wrap<DataApi>(worker);
  const api: DataApi = {
    listProjects: () => call(r.listProjects()),
    createProject: (name) => call(r.createProject(name)),
    renameProject: (id, name) => call(r.renameProject(id, name)),
    deleteProject: (id) => call(r.deleteProject(id)),
    listDatasets: (projectId) => call(r.listDatasets(projectId)),
    getDataset: (id) => call(r.getDataset(id)),
    listVersions: (datasetId) => call(r.listVersions(datasetId)),
    getVersionPage: (datasetId, version, offset, limit) =>
      call(r.getVersionPage(datasetId, version, offset, limit)),
    inspectFile: (name, bytes) => call(r.inspectFile(name, bytes)),
    previewWithSettings: (sessionId, settings) =>
      call(r.previewWithSettings(sessionId, settings)),
    diffAgainst: (sessionId, datasetId, settings) =>
      call(r.diffAgainst(sessionId, datasetId, settings)),
    commitImport: (
      sessionId: string,
      target: { projectId: string; datasetName: string } | { datasetId: string },
      settings: Parameters<DataApi['commitImport']>[2],
      acceptSchemaChanges: boolean,
      onProgress: (p: ImportProgress) => void,
    ) => {
      // Spec §7.1: persistent storage is requested on first import.
      // The call is idempotent and the promise intentionally dropped.
      void navigator.storage?.persist?.();
      return call(
        r.commitImport(
          sessionId,
          target,
          settings,
          acceptSchemaChanges,
          Comlink.proxy(onProgress),
        ),
      );
    },
    cancelImport: (sessionId) => call(r.cancelImport(sessionId)),
    discardSession: (sessionId) => call(r.discardSession(sessionId)),
    runFreq: (projectId, datasetId, version, options) =>
      call(r.runFreq(projectId, datasetId, version, options)),
    runTables: (projectId, datasetId, version, options) =>
      call(r.runTables(projectId, datasetId, version, options)),
    listRuns: (projectId) => call(r.listRuns(projectId)),
    rerun: (runId, version) => call(r.rerun(runId, version)),
    deleteRun: (runId) => call(r.deleteRun(runId)),
    engineVersion: () => call(r.engineVersion()),
    storageEstimate: () => call(r.storageEstimate()),
    requestPersistence: () => call(r.requestPersistence()),
    __simulateQuotaOnNextWrite: () => call(r.__simulateQuotaOnNextWrite()),
  };
  return api as Remote<DataApi>;
}

/** The worker's `name` option: `<db>:<tab id>` — the db name feeds
 *  `openStore`, the tab id tags this tab's lock-channel messages
 *  (Task 13). Under Tauri the name is `epiora-tauri:<tab id>`: Tauri's
 *  IPC does not reach workers, so the name is how the store backend
 *  learns to pick the native relay (Task 14, store-backend.ts). */
export const inTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
export const workerName = `${inTauri ? 'epiora-tauri' : 'epiora'}:${crypto.randomUUID()}`;

/** The app's one data worker. */
export const dataWorker = new Worker(
  new URL('./data.worker.ts', import.meta.url),
  { type: 'module', name: workerName },
);
export const dataApi: Remote<DataApi> = wrapDataApi(dataWorker);
