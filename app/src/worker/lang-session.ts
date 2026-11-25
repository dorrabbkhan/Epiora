// Program runs and the working view's write path (v1.1 plan Task 13,
// spec §5–6): `runProgram` walks a `planProgram` action list —
// cleaning actions extend the session recipe (via `printCommand`,
// the same canonicalization `applyCommand` uses), procedures run on
// the materialized working view through the core engine with a
// `working_view` RunContext stamp (receipt v4) and land in `runs`,
// WRITE writes the view as a new dataset version. Session state
// lives in `./lang`.

import { engine, lang } from './engines';
import { rpc } from './errors';
import {
  getSessionMem,
  persistSession,
  recipeTextOf,
  replaySession,
  type DatasetVersion,
  type ProgramEvent,
  type ProgramRun,
  type SessionMem,
} from './lang';
import * as datasets from '../store/repo/datasets';
import * as projects from '../store/repo/projects';
import * as runs from '../store/repo/runs';
import * as versions from '../store/repo/versions';
import type { ProcOutput } from '@engine-types/ProcOutput';
import type { RunRecord } from './api';
import type { Store, Tx } from '../store/types';
import type { FreqResult } from '@engine-types/FreqResult';
import type { TablesResult } from '@engine-types/TablesResult';
import type { MeansResult } from '@engine-types/MeansResult';

interface ActionJson {
  line?: number;
  manage?: unknown;
  procedure?: {
    line: number;
    name: string;
    options: { vars?: string[]; opts?: Record<string, unknown>; all?: boolean; except?: string[] };
    may_differ?: boolean;
  };
  read?: { name?: string; line?: number };
  write?: { line?: number };
  unsupported?: { line: number; text: string };
  note?: { line: number; text: string };
}

/** `runProgram(sessionId, text, onEvent)` — plans the program and
 *  walks its actions: `manage` extends the recipe, `procedure` runs
 *  on the materialized working view with the `working_view` stamp,
 *  `write` stores the view as a new version, `read`/`list`/`note`/
 *  `unsupported` surface as events. */
export async function runProgram(
  store: Store,
  sessionId: string,
  text: string,
  onEvent: (e: ProgramEvent) => void,
): Promise<ProgramRun> {
  const s = getSessionMem(sessionId);
  const langWasm = await lang();
  const actions = langWasm.planProgram(text) as unknown[];
  const events: ProgramEvent[] = [];
  const emit = (e: ProgramEvent) => {
    events.push(e);
    onEvent(e);
  };
  for (const a of actions as ActionJson[]) {
    const line = a.line ?? a.procedure?.line ?? a.unsupported?.line ?? a.note?.line ?? 0;
    if (a.manage !== undefined) {
      const canonical = langWasm.printCommand(a.manage);
      s.steps.push(canonical);
      try {
        await replaySession(store, s);
      } catch (e) {
        s.steps.pop();
        throw e;
      }
      emit({ kind: 'action', line, detail: canonical });
    } else if (a.procedure !== undefined) {
      emit({ kind: 'action', line, detail: a.procedure.name });
      const run = await runProcedure(store, s, a.procedure);
      emit({ kind: 'run', line, detail: a.procedure.name, run });
    } else if (a.write !== undefined) {
      await writeSessionTo(store, s);
      emit({ kind: 'action', line, detail: 'WRITE' });
    } else if (a.unsupported !== undefined) {
      emit({ kind: 'unsupported', line, detail: a.unsupported.text });
    } else if (a.note !== undefined) {
      emit({ kind: 'note', line, detail: a.note.text });
    } else if (a.read !== undefined) {
      emit({ kind: 'note', line, detail: 'READ handled by the session source' });
    } else {
      emit({ kind: 'note', line, detail: JSON.stringify(a) });
    }
  }
  await persistSession(store, s, sessionId);
  return {
    events,
    state: {
      sessionId,
      steps: [...s.steps],
      recipeText: recipeTextOf(s),
      viewSha256: s.viewSha256,
      recipeSha256: s.recipeSha256,
    },
  };
}

/** The `working_view` stamp procedures on a session carry (spec
 *  §5.2, receipt v4): source stamp + recipe + view hash. */
function workingViewStamp(s: SessionMem): Record<string, unknown> {
  return {
    source: { id: s.datasetId, version: s.sourceVersion, sha256: s.sourceSha256 },
    recipe_text: recipeTextOf(s),
    recipe_sha256: s.recipeSha256,
    view_sha256: s.viewSha256,
  };
}

/** Map an action's procedure name + register options onto
 *  `freq`/`tables`/`means` on the materialized view. The options
 *  JSON shape is the register's `{vars, opts, all, except}`. */
async function runProcedure(
  store: Store,
  s: SessionMem,
  proc: NonNullable<ActionJson['procedure']>,
): Promise<RunRecord> {
  const eng = await engine();
  const handle = eng.datasetFromSnapshot(s.datasetId, s.sourceVersion, s.view!);
  try {
    const name = proc.name.toLowerCase();
    const vars = proc.options.vars ?? [];
    const opts = (proc.options.opts ?? {}) as Record<string, unknown>;
    const project = (await projects.list(store)).find((p) => p.id === s.projectId);
    const ctx = {
      timestamp: new Date().toISOString(),
      project: { id: s.projectId, name: project?.name ?? s.projectId },
      working_view: workingViewStamp(s),
    };
    let procedure: 'freq' | 'tables' | 'means';
    let options: unknown;
    if (name === 'freq') {
      procedure = 'freq';
      options = { variable: vars[0] ?? '', ...opts };
    } else if (name === 'means') {
      procedure = 'means';
      options = {
        variable: vars[0] ?? '',
        ...(vars[1] !== undefined ? { group: vars[1] } : {}),
        ...opts,
      };
    } else {
      procedure = 'tables';
      options = { exposure: vars[0] ?? '', outcome: vars[1] ?? vars[0] ?? '', ...opts };
    }
    const out =
      procedure === 'freq'
        ? (eng.freq(handle, options, ctx) as ProcOutput<FreqResult>)
        : procedure === 'means'
          ? (eng.means(handle, options, ctx) as ProcOutput<MeansResult>)
          : (eng.tables(handle, options, ctx) as ProcOutput<TablesResult>);
    const row = await runs.insert(store, {
      projectId: s.projectId,
      procedure,
      options: JSON.stringify(options),
      datasetId: s.datasetId,
      datasetVersion: s.sourceVersion,
      result: JSON.stringify(out.result),
      receipt: JSON.stringify(out.receipt),
      receiptHash: out.receipt.receipt_sha256,
      command: eng.epiInfoCommand(procedure, options),
      notes: JSON.stringify(out.notes),
      now: new Date().toISOString(),
    });
    return {
      id: row.id,
      projectId: row.projectId,
      datasetId: row.datasetId,
      version: row.datasetVersion,
      procedure,
      options: JSON.parse(row.options),
      result: JSON.parse(row.result),
      receipt: JSON.parse(row.receipt),
      receiptSha256: row.receiptHash,
      command: row.command ?? '',
      notes: JSON.parse(row.notes ?? '[]'),
      createdAt: row.createdAt,
      newerVersion: null,
    };
  } finally {
    handle.free();
  }
}

/** `WRITE`'s store step: the materialized working view becomes a new
 *  version of the session's dataset. */
async function writeSessionTo(store: Store, s: SessionMem): Promise<DatasetVersion> {
  if (s.view === null) {
    await replaySession(store, s);
  }
  const d = await datasets.get(store, s.datasetId);
  if (d === null) throw rpc('not_found', `dataset ${s.datasetId} does not exist`);
  const version = d.currentVersion + 1;
  const now = new Date().toISOString();
  await store.transaction(async (tx: Tx) => {
    await versions.insert(tx, {
      datasetId: s.datasetId,
      version,
      sourceName: s.datasetName,
      sourceSha256: s.viewSha256,
      rowCount: 0,
      contentHash: s.viewSha256,
      snapshot: s.view!,
    });
    await datasets.setCurrentVersion(tx, s.datasetId, version);
    void now;
  });
  return { datasetId: s.datasetId, version, contentHash: s.viewSha256 };
}

/** `writeSession(sessionId)` — the RPC surface for `runProgram`'s
 *  internal `writeSessionTo`. */
export async function writeSession(store: Store, sessionId: string): Promise<DatasetVersion> {
  return writeSessionTo(store, getSessionMem(sessionId));
}
