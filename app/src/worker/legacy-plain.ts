// Plain-table import for an unknown-layout legacy file (Review Focus
// 2): the file is not marked as an Epi Info project at all. Selected
// tables become ordinary datasets in a new Epiora project, in one
// cancellable transaction.

import { importer, legacy } from './engines';
import { dataFile, pushFile } from './legacy-eptb';
import { rpc } from './errors';
import {
  cancelScopeRequested,
  clearCancelScope,
  registerCancelScope,
  throwWhenCancelled,
} from './importer';
import { writeLegacyVersion } from './legacy-records';
import * as datasets from '../store/repo/datasets';
import * as projects from '../store/repo/projects';
import type { Store, Tx } from '../store/types';
import type { ImportProgress } from './api';
import {
  fileStem,
  plainTableNames,
  recordKeyJson,
  toNameBytes,
  type FileInput,
  type PlainPreview,
  type PlainTableInfo,
} from './legacy-shared';
import type { LegacyHandle } from '@legacy/epiora_legacy_wasm';

interface PlainState {
  sourceName: string;
  sourceSha256: string;
  tables: string[];
  handle: LegacyHandle;
}

const plainPreviews = new Map<string, PlainState>();

/** Reads the unknown-layout file once into a legacy batch source and
 *  offers its non-metadata tables. The handle stays with the preview so
 *  confirming does not decode the file a second time. */
export async function inspectPlainLegacy(
  files: FileInput[],
  codePage?: string,
): Promise<PlainPreview> {
  const leg = await legacy();
  const h = leg.legacyNew();
  try {
    const f = dataFile(await toNameBytes(files));
    await pushFile(leg, h, f.name, f.bytes, codePage);
    const tables = plainTableNames(leg.legacyTables(h) as string[]);
    if (tables.length === 0) {
      throw rpc('not_found', `${f.name} has no plain tables to import`);
    }
    const previewId = crypto.randomUUID();
    const sourceSha256 = (await importer()).sha256(f.bytes);
    plainPreviews.set(previewId, { sourceName: f.name, sourceSha256, tables, handle: h });
    registerCancelScope(previewId);
    return { previewId, sourceName: f.name, tables };
  } catch (e) {
    h.free();
    throw e;
  }
}

/** Imports the chosen tables as plain datasets. There is deliberately
 *  no `legacy_projects` row: the source was not an Epi Info project, so
 *  the result is an ordinary Epiora project. */
export async function commitPlainLegacy(
  store: Store,
  previewId: string,
  tables: string[],
  onProgress: (p: ImportProgress) => void,
): Promise<string> {
  const state = plainPreviews.get(previewId);
  if (state === undefined) {
    throw rpc('session_not_found', `plain table preview ${previewId} does not exist`);
  }
  const chosen = tables.filter((t) => state.tables.includes(t));
  if (chosen.length === 0) {
    throw rpc('not_found', 'choose at least one listed table to import');
  }
  const emit = (stage: ImportProgress['stage'], done: number, total: number) => {
    try {
      void onProgress({ stage, done, total });
    } catch {
      // A released progress proxy must not fail the import.
    }
  };
  const checkCancel = () => throwWhenCancelled(() => cancelScopeRequested(previewId));
  try {
    await checkCancel();
    const leg = await legacy();
    const mapped = leg.legacyPlainTables(state.handle, chosen) as {
      datasets: PlainTableInfo[];
      snapshots: Uint8Array[];
    };
    const now = new Date().toISOString();
    return await store.transaction(async (tx: Tx) => {
      await checkCancel();
      const project = await projects.create(tx, fileStem(state.sourceName), now);
      emit('write', 0, chosen.length);
      for (const [i, table] of mapped.datasets.entries()) {
        await checkCancel();
        const d = await datasets.create(
          tx,
          { projectId: project.id, name: table.name, key: recordKeyJson(''), settings: '{}' },
          now,
        );
        await writeLegacyVersion({
          tx,
          datasetId: d.id,
          version: 1,
          now,
          sourceName: state.sourceName,
          sourceSha256: state.sourceSha256,
          snapshot: new Uint8Array(mapped.snapshots[i]!),
          rowCount: table.records,
          contentHash: table.hash,
          keyColumn: '',
          diff: null,
          checkCancel,
        });
        emit('write', i + 1, chosen.length);
      }
      emit('snapshot', 1, 1);
      await checkCancel();
      return project.id;
    });
  } finally {
    state.handle.free();
    plainPreviews.delete(previewId);
    clearCancelScope(previewId);
  }
}

export function discardPlainPreview(previewId: string): void {
  plainPreviews.get(previewId)?.handle.free();
  plainPreviews.delete(previewId);
  clearCancelScope(previewId);
}
