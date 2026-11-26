// Working-view moves after a legacy re-import (v1.1 Task 15 Review
// Focus 5): replay each persisted recipe onto its dataset's current
// version through the Task 13 replay machinery. All replays complete
// before the first write, so one failed move leaves every session on
// its old version.

import { lang } from './engines';
import { loadEntry } from './snapshots';
import * as datasets from '../store/repo/datasets';
import * as sessionsRepo from '../store/repo/sessions';
import * as versions from '../store/repo/versions';
import type { Store } from '../store/types';
import {
  manageCommand,
  persistSession,
  recipeTextOf,
  replaySession,
  sessionState,
  type SessionMem,
  type SessionState,
} from './lang';

/** One session moved onto its dataset's current version. */
export interface MovedSession {
  sessionId: string;
  datasetId: string;
  fromVersion: number;
  toVersion: number;
  state: SessionState;
}

function recipeSteps(langWasm: Awaited<ReturnType<typeof lang>>, recipeText: string): string[] {
  const body = recipeText.split('\n').slice(1).join('\n').trim();
  if (body === '') return [];
  const parsed = langWasm.parseProgram(body) as { statements: { node: unknown }[] };
  return parsed.statements.map((st) => langWasm.printCommand(manageCommand(st.node)));
}

/** Moves every persisted session of `projectId` whose source dataset
 *  advanced onto the current version by replaying its unchanged recipe
 *  through the same Task 13 replay path `openSession`/`applyCommand`
 *  use. Replays all sessions before the first write: if one recipe no
 *  longer applies, nothing moves. */
export async function moveSessionsToCurrent(
  store: Store,
  projectId: string,
): Promise<MovedSession[]> {
  const langWasm = await lang();
  const pending: {
    row: sessionsRepo.Session;
    mem: SessionMem;
    dataset: datasets.Dataset;
  }[] = [];
  for (const row of await sessionsRepo.sessions(store, projectId)) {
    const d = await datasets.get(store, row.sourceDataset);
    if (d === null || d.currentVersion <= row.sourceVersion) continue;
    const vrow = await versions.get(store, d.id, d.currentVersion);
    if (vrow === null) continue;
    const entry = await loadEntry(store, d.id, d.currentVersion);
    const existing = sessionState.get(row.id);
    const mem: SessionMem = {
      projectId,
      datasetId: d.id,
      datasetName: d.name,
      sourceVersion: d.currentVersion,
      sourceSha256: vrow.contentHash,
      source: entry.bytes,
      asOf: row.asOf,
      steps: existing !== undefined && existing.sourceVersion === row.sourceVersion
        ? [...existing.steps]
        : recipeSteps(langWasm, row.recipeText),
      view: null,
      viewSha256: '',
      recipeSha256: '',
    };
    await replaySession(store, mem);
    pending.push({ row, mem, dataset: d });
  }
  const moved: MovedSession[] = [];
  for (const { row, mem } of pending) {
    sessionState.set(row.id, mem);
    await persistSession(store, mem, row.id);
    moved.push({
      sessionId: row.id,
      datasetId: mem.datasetId,
      fromVersion: row.sourceVersion,
      toVersion: mem.sourceVersion,
      state: {
        sessionId: row.id,
        steps: [...mem.steps],
        recipeText: recipeTextOf(mem),
        viewSha256: mem.viewSha256,
        recipeSha256: mem.recipeSha256,
      },
    });
  }
  return moved;
}
