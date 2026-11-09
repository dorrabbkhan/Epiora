// Import sessions, inspect/preview and diff (v0.3 spec §4.2, §5.3,
// §6.4). `inspectFile` keeps the file bytes and a cached
// `ImportedHandle` keyed by the exact settings JSON until
// `discardSession` or the commit in commit.ts; the handle is what
// `recordBatch`, `toSnapshot`, `contentHash` and `diffAgainst` page
// over.
//
// `getSession`, `importedFor` and `destroySession` are exported for
// commit.ts — the single-transaction commit lives there.

import type { ImportedHandle } from '@import/epiora_import_wasm';
import type { DiffSummary } from '@import-types/DiffSummary';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { Inspection } from '@import-types/Inspection';
import type { KeyCandidate } from '@import-types/KeyCandidate';
import { importer } from './engines';
import { rpc } from './errors';
import { loadEntry } from './snapshots';
import * as datasets from '../store/repo/datasets';
import type { Store } from '../store/types';

export interface Session {
  name: string;
  bytes: Uint8Array;
  imported: { settingsKey: string; handle: ImportedHandle } | null;
  cancelled: boolean;
}

const sessions = new Map<string, Session>();

export function getSession(sessionId: string): Session {
  const s = sessions.get(sessionId);
  if (s === undefined) {
    throw rpc('session_not_found', `import session ${sessionId} does not exist`);
  }
  return s;
}

export function destroySession(sessionId: string): void {
  sessions.get(sessionId)?.imported?.handle.free();
  sessions.delete(sessionId);
}

/** The ImportedHandle for `settings`, reusing the session's cached one
 *  when the settings are byte-identical. */
export async function importedFor(
  s: Session,
  settings: ImportSettings,
): Promise<ImportedHandle> {
  const settingsKey = JSON.stringify(settings);
  if (s.imported?.settingsKey === settingsKey) return s.imported.handle;
  const imp = await importer();
  const handle = imp.importFile(s.bytes, s.name, settings);
  s.imported?.handle.free();
  s.imported = { settingsKey, handle };
  return handle;
}

export async function inspectFile(
  name: string,
  bytes: ArrayBuffer,
): Promise<{ sessionId: string; inspection: Inspection; keyCandidates: KeyCandidate[] }> {
  const imp = await importer();
  const data = new Uint8Array(bytes);
  const inspection = imp.inspect(data, name) as Inspection;
  const handle = imp.importFile(data, name, inspection.settings);
  const sessionId = crypto.randomUUID();
  sessions.set(sessionId, {
    name,
    bytes: data,
    imported: { settingsKey: JSON.stringify(inspection.settings), handle },
    cancelled: false,
  });
  return { sessionId, inspection, keyCandidates: handle.keyCandidates() as KeyCandidate[] };
}

export async function previewWithSettings(
  sessionId: string,
  settings: ImportSettings,
): Promise<{ inspection: Inspection; keyCandidates: KeyCandidate[] }> {
  const s = getSession(sessionId);
  const imp = await importer();
  const inspection = imp.inspectWith(s.bytes, s.name, settings) as Inspection;
  const handle = await importedFor(s, inspection.settings);
  return { inspection, keyCandidates: handle.keyCandidates() as KeyCandidate[] };
}

export async function diffAgainst(
  store: Store,
  sessionId: string,
  datasetId: string,
  settings: ImportSettings,
): Promise<DiffSummary> {
  const s = getSession(sessionId);
  const dataset = await datasets.get(store, datasetId);
  if (dataset === null) throw rpc('not_found', `dataset ${datasetId} does not exist`);
  const handle = await importedFor(s, settings);
  const prev = await loadEntry(store, datasetId, dataset.currentVersion);
  const imp = await importer();
  const diff = imp.diffAgainst(prev.bytes, handle);
  const summary = diff.summary() as DiffSummary;
  diff.free();
  return summary;
}

export async function cancelImport(sessionId: string): Promise<void> {
  getSession(sessionId).cancelled = true;
}

export async function discardSession(sessionId: string): Promise<void> {
  const s = getSession(sessionId);
  s.cancelled = true;
  destroySession(sessionId);
}
