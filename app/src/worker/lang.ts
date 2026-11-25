// Cleaning sessions, recipes and program runs (v1.1 plan Task 13,
// spec §5–6): `openSession` replays `AS-OF <now>` plus the project's
// PERMANENT DEFINEs over the stored source snapshot; `applyCommand`
// appends one parsed-and-reprinted command, `undoLast` drops it.
// `runProgram`/`writeSession` live in `./lang-session` — cleaning
// actions extend the recipe, procedures run on the materialized
// working view through the core engine with a `working_view`
// RunContext stamp (receipt v4) and land in `runs`, WRITE writes the
// view as a new dataset version. Session rows persist so a reload
// can resume — `sessions` holds the recipe text, `sessionState` the
// replayed view.
//
// `linked` is every dataset snapshot in the project keyed by name —
// a RELATE command finds its child there; `links` carries the
// project's stored `dataset_links` rows as `DatasetLink` JSON so a
// *keyless* RELATE resolves its key pair (v1.1 Task 13b — `relate_no_link`
// only when no stored link names the source and child).

import type { DatasetLink } from '@engine-types/DatasetLink';
import { lang } from './engines';
import { rpc } from './errors';
import { loadEntry } from './snapshots';
import * as datasets from '../store/repo/datasets';
import * as legacyRepo from '../store/repo/legacy';
import * as sessionsRepo from '../store/repo/sessions';
import * as versions from '../store/repo/versions';
import type { RunRecord } from './api';
import type { Store } from '../store/types';

/** `openSession`'s return — enough for the UI to show the empty
 *  recipe and the source the commands will replay over. */
export interface Session {
  sessionId: string;
  projectId: string;
  datasetId: string;
  sourceVersion: number;
  asOf: string;
  recipeText: string;
}

/** `applyCommand`/`undoLast`'s return — the recipe as the session
 *  keeps it plus the replayed view's stamp. */
export interface SessionState {
  sessionId: string;
  steps: string[];
  recipeText: string;
  viewSha256: string;
  recipeSha256: string;
}

/** One streamed `runProgram` event. */
export interface ProgramEvent {
  kind: 'action' | 'run' | 'note' | 'unsupported' | 'error';
  /** 1-based line the action came from. */
  line: number;
  /** The action's label (command text, procedure name, message). */
  detail: string;
  run?: RunRecord;
}

/** `runProgram`'s return — the events produced plus the session's
 *  state after the run. */
export interface ProgramRun {
  events: ProgramEvent[];
  state: SessionState;
}

/** `writeSession`'s return — the new dataset version row. */
export interface DatasetVersion {
  datasetId: string;
  version: number;
  contentHash: string;
}

export interface SessionMem {
  projectId: string;
  datasetId: string;
  datasetName: string;
  sourceVersion: number;
  sourceSha256: string;
  /** Source snapshot bytes — the recipe's replay input. */
  source: Uint8Array;
  asOf: string;
  /** Canonical printed commands, one per cleaning step. */
  steps: string[];
  /** Last `replay` result; `null` until the first command lands. */
  view: Uint8Array | null;
  viewSha256: string;
  recipeSha256: string;
}

const sessionState = new Map<string, SessionMem>();

export function getSessionMem(sessionId: string): SessionMem {
  const s = sessionState.get(sessionId);
  if (s === undefined) {
    throw rpc('session_not_found', `session ${sessionId} does not exist`);
  }
  return s;
}

/** `AS-OF YYYY-MM-DD HH:MM:SS` — `Recipe::parse`'s header format,
 *  spelled in UTC so the stored text is the instant it names. */
function asOfLine(iso: string): string {
  const d = new Date(iso);
  const p = (n: number) => String(n).padStart(2, '0');
  return (
    `AS-OF ${d.getUTCFullYear()}-${p(d.getUTCMonth() + 1)}-${p(d.getUTCDate())} ` +
    `${p(d.getUTCHours())}:${p(d.getUTCMinutes())}:${p(d.getUTCSeconds())}`
  );
}

/** The session's current canonical recipe text: the header plus one
 *  line per step. `Recipe::canonical_text` separates steps with
 *  blank-line-free newlines — each step is itself one printed
 *  command block. */
export function recipeTextOf(s: SessionMem): string {
  const head = asOfLine(s.asOf);
  return s.steps.length === 0 ? head : `${head}\n${s.steps.join('\n')}`;
}

/** Every dataset in the project other than the source, keyed by name
 *  for `LinkedDatasets::get` (which lower-cases lookups itself). */
async function linkedSnapshots(
  store: Store,
  s: SessionMem,
): Promise<Record<string, Uint8Array>> {
  const out: Record<string, Uint8Array> = {};
  for (const d of await datasets.listByProject(store, s.projectId)) {
    if (d.id === s.datasetId) continue;
    const entry = await loadEntry(store, d.id, d.currentVersion);
    out[d.name] = entry.bytes;
  }
  return out;
}

/** The store's `DatasetLink` rows as `replay`'s `DatasetLinkJson[]`
 *  (camelCase → the serde field names). */
function linkJson(l: legacyRepo.DatasetLink): DatasetLink {
  return {
    parent: l.parentDataset,
    child: l.childDataset,
    child_key: l.childKey,
    parent_key: l.parentKey,
    kind: l.kind,
  };
}

/** Replay the recipe over the stored source — the one code path all
 *  mutating session calls funnel through. */
export async function replaySession(store: Store, s: SessionMem): Promise<void> {
  const langWasm = await lang();
  const linked = await linkedSnapshots(store, s);
  const links = (await sessionLinks(store, s.projectId)).map(linkJson);
  const out = langWasm.replay(s.source, linked, recipeTextOf(s), links) as {
    view: Uint8Array;
    viewSha256: string;
    recipeSha256: string;
  };
  s.view = new Uint8Array(out.view);
  s.viewSha256 = out.viewSha256;
  s.recipeSha256 = out.recipeSha256;
}

export async function persistSession(store: Store, s: SessionMem, sessionId: string): Promise<void> {
  await sessionsRepo.upsertSession(store, {
    id: sessionId,
    projectId: s.projectId,
    sourceDataset: s.datasetId,
    sourceVersion: s.sourceVersion,
    recipeText: recipeTextOf(s),
    asOf: s.asOf,
    updatedAt: new Date().toISOString(),
  });
}

/** `openSession(projectId, datasetId, version)` — captures the as-of
 *  instant, loads the source snapshot and starts the recipe with the
 *  project's PERMANENT DEFINEs (spec §6.1). */
export async function openSession(
  store: Store,
  projectId: string,
  datasetId: string,
  version: number,
): Promise<Session> {
  const d = await datasets.get(store, datasetId);
  if (d === null || d.projectId !== projectId) {
    throw rpc('not_found', `dataset ${datasetId} does not exist in project ${projectId}`);
  }
  const vrow = await versions.get(store, datasetId, version);
  if (vrow === null) {
    throw rpc('snapshot_unavailable', `dataset ${datasetId} has no version ${version}`);
  }
  const entry = await loadEntry(store, datasetId, version);
  const sessionId = crypto.randomUUID();
  const s: SessionMem = {
    projectId,
    datasetId,
    datasetName: d.name,
    sourceVersion: version,
    sourceSha256: vrow.contentHash,
    source: entry.bytes,
    asOf: new Date().toISOString(),
    steps: [],
    view: null,
    viewSha256: '',
    recipeSha256: '',
  };
  // PERMANENT-scope DEFINEs open every session's recipe.
  for (const pv of await sessionsRepo.permanentVars(store, projectId)) {
    s.steps.push(
      pv.exprText === null
        ? `DEFINE ${pv.name} ${pv.type} PERMANENT`
        : `DEFINE ${pv.name} = ${pv.exprText} PERMANENT`,
    );
  }
  await replaySession(store, s);
  sessionState.set(sessionId, s);
  await persistSession(store, s, sessionId);
  return {
    sessionId,
    projectId,
    datasetId,
    sourceVersion: version,
    asOf: s.asOf,
    recipeText: recipeTextOf(s),
  };
}

/** The `manage` node's command JSON out of a parsed single-statement
 *  program — `applyCommand` takes exactly one cleaning command. */
function manageCommand(node: unknown): unknown {
  const n = node as { manage?: unknown };
  if (n.manage === undefined) {
    throw rpc('internal_error', 'applyCommand expects a cleaning command');
  }
  return n.manage;
}

export async function applyCommand(
  store: Store,
  sessionId: string,
  commandText: string,
): Promise<SessionState> {
  const s = getSessionMem(sessionId);
  const langWasm = await lang();
  const program = langWasm.parseProgram(commandText) as {
    statements: { node: unknown }[];
  };
  if (program.statements.length !== 1) {
    throw rpc('internal_error', 'applyCommand takes exactly one command');
  }
  const canonical = langWasm.printCommand(manageCommand(program.statements[0]!.node));
  const prior = s.steps;
  // `CANCEL READ` closes the working view and discards the recipe —
  // the steps clear, the command is never stored (plan Task 13b).
  s.steps = canonical === 'CANCEL READ' ? [] : [...s.steps, canonical];
  try {
    await replaySession(store, s);
  } catch (e) {
    s.steps = prior;
    throw e;
  }
  await persistSession(store, s, sessionId);
  return {
    sessionId,
    steps: [...s.steps],
    recipeText: recipeTextOf(s),
    viewSha256: s.viewSha256,
    recipeSha256: s.recipeSha256,
  };
}

export async function undoLast(store: Store, sessionId: string): Promise<SessionState> {
  const s = getSessionMem(sessionId);
  if (s.steps.length > 0) {
    const popped = s.steps.pop()!;
    try {
      await replaySession(store, s);
    } catch (e) {
      s.steps.push(popped);
      throw e;
    }
  }
  await persistSession(store, s, sessionId);
  return {
    sessionId,
    steps: [...s.steps],
    recipeText: recipeTextOf(s),
    viewSha256: s.viewSha256,
    recipeSha256: s.recipeSha256,
  };
}

/** The project's stored `dataset_links` — `replaySession` maps them
 *  onto `replay`'s `DatasetLink` JSON so a keyless RELATE resolves
 *  its key pair. */
export async function sessionLinks(
  store: Store,
  projectId: string,
): Promise<legacyRepo.DatasetLink[]> {
  return legacyRepo.links(store, projectId);
}

/** `discardSession` — drops the in-memory state and the row. */
export async function discardLangSession(store: Store, sessionId: string): Promise<void> {
  sessionState.delete(sessionId);
  await sessionsRepo.removeSession(store, sessionId);
}

/** Re-open a persisted session (reload path): replays the stored
 *  recipe against the source dataset. Not on the RPC surface yet —
 *  kept for Task 16's resume flow. */
export async function resumeSession(store: Store, sessionId: string): Promise<SessionState> {
  const row = await sessionsRepo.session(store, sessionId);
  if (row === null) {
    throw rpc('session_not_found', `session ${sessionId} does not exist`);
  }
  const d = await datasets.get(store, row.sourceDataset);
  if (d === null) {
    throw rpc('not_found', `dataset ${row.sourceDataset} does not exist`);
  }
  const entry = await loadEntry(store, d.id, row.sourceVersion);
  const vrow = await versions.get(store, d.id, row.sourceVersion);
  // Steps re-derive by parsing the stored recipe and reprinting each
  // command — the canonical spellings `recipeTextOf` rejoins (a
  // step's own text can span lines, so no naive split).
  const langWasm = await lang();
  const body = row.recipeText.split('\n').slice(1).join('\n').trim();
  const parsed = body === '' ? { statements: [] } :
    (langWasm.parseProgram(body) as { statements: { node: unknown }[] });
  const steps = parsed.statements.map((st) =>
    langWasm.printCommand(manageCommand(st.node)),
  );
  const s: SessionMem = {
    projectId: row.projectId,
    datasetId: d.id,
    datasetName: d.name,
    sourceVersion: row.sourceVersion,
    sourceSha256: vrow?.contentHash ?? '',
    source: entry.bytes,
    asOf: row.asOf,
    steps,
    view: null,
    viewSha256: '',
    recipeSha256: '',
  };
  sessionState.set(sessionId, s);
  await replaySession(store, s);
  return {
    sessionId,
    steps: [...s.steps],
    recipeText: recipeTextOf(s),
    viewSha256: s.viewSha256,
    recipeSha256: s.recipeSha256,
  };
}
