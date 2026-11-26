// RPC contract tests (v0.3 plan Task 9, spec §5.3/§11): every detail
// code the worker can put on the wire — from the Rust bindings'
// `code()` lists, `StoreError` and the worker-only codes in errors.ts —
// has a user-facing message, and the API's types are exactly the
// generated binding types (checked structurally in contract.types.ts).

import { readFileSync } from 'node:fs';
import { describe, expect, it, vi } from 'vitest';
import { legacyErrorT } from '../legacy/strings';
import { t } from '../ui/strings';
import {
  ENGINE_ERROR_CODES,
  IMPORT_ERROR_CODES,
  LANG_ERROR_CODES,
  LEGACY_ERROR_CODES,
  STORE_ERROR_CODES,
  WORKER_ERROR_CODES,
} from './errors';
import { api } from './api';
import * as langWorker from './lang';
import type { Row, SqlValue, Store, Tx } from '../store/types';
import { TYPES_CHECKED } from './contract.types';

// Task 13 fix round: `inspectLegacy`/`planRefresh` take `File[]` per
// the plan's Interfaces block — the test below drives a call with
// real `File` objects. The wasm boundary is mocked (node vitest
// cannot fetch the `?url` wasm; the browser suite drives the same
// call unmocked) — what is asserted is the File→bytes conversion and
// `dataFile` picking the data file over the `.prj` companion.
vi.mock('@import/epiora_import_wasm', () => ({
  default: vi.fn().mockResolvedValue(undefined),
  sha256: vi.fn(() => 'file-sha256'),
}));
vi.mock('@legacy/epiora_legacy_wasm', () => ({
  default: vi.fn().mockResolvedValue(undefined),
  legacyNew: vi.fn(() => ({ free: vi.fn() })),
  legacyPush: vi.fn(),
  legacyMap: vi.fn(() => ({
    project: {
      name: 'file-preview',
      legacy_id: 'x',
      source_format: 'ei7_access',
      views: [],
      datasets: [],
      links: [],
      programs: [],
      check_code: [],
      report: [],
    },
    snapshots: [],
  })),
}));
vi.mock('../legacy/mdb', () => ({
  openMdb: vi.fn(async () => ({
    format: () => 'JET4',
    tables: () => ['T'],
    batches: async function* () {
      yield new Uint8Array([0]);
    },
    close: vi.fn(),
  })),
}));

// Task 13b: the session tests below drive `openSession`/`applyCommand`
// against a fake `Store`; both wasm boundaries are mocked — `engine()`
// only verifies the snapshot's stored hash (returned verbatim),
// `lang()`'s `replay` is the call under test.
vi.mock('@engine/epiora_wasm', () => ({
  default: vi.fn().mockResolvedValue(undefined),
  datasetFromSnapshot: vi.fn(() => ({ free: vi.fn() })),
  datasetHash: vi.fn(() => 'hash-of-source'),
}));
vi.mock('@lang/epiora_lang_wasm', () => ({
  default: vi.fn().mockResolvedValue(undefined),
  parseProgram: vi.fn((text: string) => ({
    statements: [
      {
        node: {
          manage: text.trim() === 'CANCEL READ' ? 'cancel_read' : { select: { lit: { integer: 1 } } },
        },
      },
    ],
  })),
  printCommand: vi.fn((cmd: unknown) => (cmd === 'cancel_read' ? 'CANCEL READ' : 'SELECT v > 15')),
  planProgram: vi.fn(() => []),
  register: vi.fn(() => ({})),
  replay: vi.fn(() => ({
    view: new Uint8Array([9]),
    viewSha256: 'view-hash',
    recipeSha256: 'recipe-hash',
  })),
}));

/** A `Store` answering the session repos' SQL by table — one parent
 *  dataset with one snapshot and one stored `dataset_links` row. */
function fakeStore(): Store {
  const dataset: Row = {
    id: 'ds1',
    project_id: 'p1',
    name: 'parent',
    record_key: '"row_position"',
    settings: '{}',
    current_version: 1,
    created_at: '2026-11-25T00:00:00Z',
  };
  const version: Row = {
    dataset_id: 'ds1',
    version: 1,
    source_name: 'parent',
    source_sha256: 'hash-of-source',
    row_count: 0,
    content_hash: 'hash-of-source',
    snapshot_available: 1,
  };
  const link: Row = {
    project_id: 'p1',
    parent_dataset: 'parent',
    child_dataset: 'child',
    child_key: 'ck',
    parent_key: 'k',
    kind: 'relate',
  };
  const store: Store = {
    query: async (sql: string) => {
      if (sql.includes('FROM dataset_links')) return [link];
      if (sql.includes('SELECT snapshot FROM dataset_versions')) {
        return [{ snapshot: new Uint8Array([7]) }];
      }
      if (sql.includes('FROM dataset_versions')) return [version];
      if (sql.includes('FROM datasets WHERE project_id')) return [dataset];
      if (sql.includes('FROM datasets WHERE id')) return [dataset];
      return [];
    },
    exec: async () => {},
    transaction: async <T>(fn: (tx: Tx) => Promise<T>) => fn(store),
    close: async () => {},
  };
  return store;
}

/** A `Store` with four datasets — `parent` (the session's source),
 *  `other`, `child`, `child2` — and two stored links: `parent →
 *  child` and `other → child2`. Queries filter on the bound
 *  parameters like the real SQL does. */
function fakeStoreLinked(): Store {
  const ds = (id: string, name: string): Row => ({
    id,
    project_id: 'p1',
    name,
    record_key: '"row_position"',
    settings: '{}',
    current_version: 1,
    created_at: '2026-11-25T00:00:00Z',
  });
  const datasets = [ds('ds1', 'parent'), ds('ds2', 'other'), ds('ds3', 'child'), ds('ds4', 'child2')];
  const links: Row[] = [
    {
      project_id: 'p1',
      parent_dataset: 'parent',
      child_dataset: 'child',
      child_key: 'pid',
      parent_key: 'id',
      kind: 'relate',
    },
    {
      project_id: 'p1',
      parent_dataset: 'other',
      child_dataset: 'child2',
      child_key: 'ck2',
      parent_key: 'k2',
      kind: 'relate',
    },
  ];
  const store: Store = {
    query: async (sql: string, params?: SqlValue[]) => {
      if (sql.includes('FROM dataset_links')) return links;
      if (sql.includes('SELECT snapshot FROM dataset_versions')) {
        return [{ snapshot: new Uint8Array([7]) }];
      }
      if (sql.includes('FROM dataset_versions')) {
        return [
          {
            dataset_id: params?.[0] ?? '',
            version: 1,
            source_name: datasets.find((d) => d.id === params?.[0])?.name ?? '',
            source_sha256: 'hash-of-source',
            row_count: 0,
            content_hash: 'hash-of-source',
            snapshot_available: 1,
          },
        ];
      }
      if (sql.includes('FROM datasets WHERE project_id')) return datasets;
      if (sql.includes('FROM datasets WHERE id')) {
        return datasets.filter((d) => d.id === params?.[0]);
      }
      return [];
    },
    exec: async () => {},
    transaction: async <T>(fn: (tx: Tx) => Promise<T>) => fn(store),
    close: async () => {},
  };
  return store;
}

const ALL_CODES = [
  ...IMPORT_ERROR_CODES,
  ...ENGINE_ERROR_CODES,
  ...STORE_ERROR_CODES,
  ...WORKER_ERROR_CODES,
  ...LEGACY_ERROR_CODES,
  ...LANG_ERROR_CODES,
];

/** The string literals returned by `fn code()` in a Rust error enum,
 *  read straight from the crate source. The bindings export no code
 *  lists, so IMPORT_/ENGINE_ERROR_CODES in errors.ts are hand-copied —
 *  this is what fails when a Rust `code()` gains a code. A zero-count
 *  extraction is an error, never a pass. */
function rustCodes(path: string): string[] {
  const src = readFileSync(new URL(path, import.meta.url), 'utf8');
  const start = src.indexOf('fn code(');
  expect(start, `${path}: fn code() not found`).toBeGreaterThanOrEqual(0);
  const codes = [...src.slice(start).matchAll(/=>\s*"([a-z_0-9]+)"/g)].map((m) => m[1]!);
  expect(codes.length, `${path}: extracted zero codes from fn code()`).toBeGreaterThan(0);
  return codes;
}

describe('worker RPC contract', () => {
  it('every_detail_code_has_a_user_message', () => {
    const messages = {
      ...(t.errors as Record<string, string | undefined>),
      ...(legacyErrorT as Record<string, string | undefined>),
    };
    for (const code of ALL_CODES) {
      expect(messages[code], `missing userMessage for ${code}`).toBeTypeOf('string');
      expect(messages[code]!.length).toBeGreaterThan(0);
    }
  });

  it('error_code_lists_match_rust', () => {
    // `Self::Engine(e) => e.code()` yields no literal, so the extracted
    // set is exactly the import codes errors.ts lists — the transparent
    // wrap surfaces under ENGINE_ERROR_CODES there on purpose.
    const importCodes = rustCodes('../../../crates/epiora-import/src/error.rs');
    const engineCodes = rustCodes('../../../crates/epiora-core/src/error.rs');
    expect(importCodes.slice().sort()).toEqual([...IMPORT_ERROR_CODES].sort());
    expect(engineCodes.slice().sort()).toEqual([...ENGINE_ERROR_CODES].sort());
    // v1.1 Task 13: the two new wasm crates' codes have their own
    // lists — `legacy_no_table` lives in source.rs as a
    // `Source { code }` literal, not a `=>` arm, so it is listed in
    // errors.ts but excluded here.
    const legacyCodes = rustCodes('../../../crates/epiora-legacy/src/error.rs');
    const langCodes = rustCodes('../../../crates/epiora-lang/src/lib.rs');
    expect(legacyCodes.slice().sort()).toEqual(
      [...LEGACY_ERROR_CODES].filter((c) => c !== 'legacy_no_table').sort(),
    );
    expect(langCodes.slice().sort()).toEqual([...LANG_ERROR_CODES].sort());
  });

  it('exposes_every_new_rpc', () => {
    // v1.1 Task 13's worker RPC surface — `wire` keeps them as
    // functions on the `api` object.
    const a = api as unknown as Record<string, unknown>;
    for (const name of [
      'inspectLegacy',
      'inspectLegacySqlServer',
      'getLegacyProject',
      'commitLegacy',
      'discardLegacyPreview',
      'planRefresh',
      'commitRefresh',
      'discardRefreshPreview',
      'inspectPlainLegacy',
      'commitPlainLegacy',
      'moveSessionsToCurrent',
      'openSession',
      'applyCommand',
      'undoLast',
      'runProgram',
      'writeSession',
    ]) {
      expect(typeof a[name], `api.${name} missing`).toBe('function');
    }
  });

  it('replay_passes_stored_links', async () => {
    // Task 13b: `openSession`'s replay carries the project's stored
    // `dataset_links` as the serde `DatasetLink` shape — parent/child
    // dataset names, snake_case key fields, the kind as-is.
    const store = fakeStore();
    await langWorker.openSession(store, 'p1', 'ds1', 1);
    const replay = vi.mocked((await import('@lang/epiora_lang_wasm')).replay);
    expect(replay).toHaveBeenCalled();
    const args = replay.mock.calls[0]!;
    expect(args[1]).toEqual({}); // the source is skipped in `linked`
    expect(args[3]).toEqual([
      { parent: 'parent', child: 'child', child_key: 'ck', parent_key: 'k', kind: 'relate' },
    ]);
  });

  it('replay_passes_only_source_links', async () => {
    // Task 13b fix round: `replay` gets only the stored links whose
    // `parent` names the session's source dataset — the `other →
    // child2` link never crosses the wasm boundary.
    const store = fakeStoreLinked();
    const session = await langWorker.openSession(store, 'p1', 'ds1', 1);
    await langWorker.applyCommand(store, session.sessionId, 'SELECT v > 15');
    const replay = vi.mocked((await import('@lang/epiora_lang_wasm')).replay);
    expect(replay.mock.calls.at(-1)![3]).toEqual([
      { parent: 'parent', child: 'child', child_key: 'pid', parent_key: 'id', kind: 'relate' },
    ]);
  });

  it('apply_cancel_read_returns_source_state', async () => {
    // Task 13b: `applyCommand('CANCEL READ')` clears the recipe —
    // the command is never stored as a step and the session replays
    // the bare AS-OF header over the source.
    const store = fakeStore();
    const session = await langWorker.openSession(store, 'p1', 'ds1', 1);
    const after = await langWorker.applyCommand(store, session.sessionId, 'SELECT v > 15');
    expect(after.steps).toEqual(['SELECT v > 15']);
    const state = await langWorker.applyCommand(store, session.sessionId, 'CANCEL READ');
    expect(state.steps).toEqual([]);
    expect(state.recipeText.startsWith('AS-OF ')).toBe(true);
    expect(state.recipeText.split('\n')).toHaveLength(1);
    const replay = vi.mocked((await import('@lang/epiora_lang_wasm')).replay);
    expect(replay.mock.calls.at(-1)![2]).toBe(state.recipeText);
  });

  it('inspect_legacy_accepts_real_file_objects', async () => {
    // The interface's `File[]` spelling driven for real: `toNameBytes`
    // must read each `File` via `arrayBuffer()` and `dataFile` must
    // pick the `.mdb` over the `.prj` companion.
    const preview = await api.inspectLegacy([
      new File([new Uint8Array([0xde, 0xad])], 'project.prj'),
      new File([new Uint8Array([0xbe, 0xef])], 'project.mdb'),
    ]);
    expect(preview.project.name).toBe('file-preview');
    const openMdb = vi.mocked((await import('../legacy/mdb')).openMdb);
    expect(openMdb).toHaveBeenCalledTimes(1);
    expect(Array.from(openMdb.mock.calls[0]![0] as Uint8Array)).toEqual([0xbe, 0xef]);
  });

  it('api_types_match_bindings', () => {
    // The real check runs in `tsc --noEmit`: contract.types.ts assigns
    // the generated binding types to the API's parameter and return
    // types both ways. This asserts the file is part of the suite.
    expect(TYPES_CHECKED).toBe(true);
  });
});
