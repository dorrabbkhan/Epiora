// RPC contract tests (v0.3 plan Task 9, spec §5.3/§11): every detail
// code the worker can put on the wire — from the Rust bindings'
// `code()` lists, `StoreError` and the worker-only codes in errors.ts —
// has a user-facing message, and the API's types are exactly the
// generated binding types (checked structurally in contract.types.ts).

import { readFileSync } from 'node:fs';
import { describe, expect, it, vi } from 'vitest';
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
import { TYPES_CHECKED } from './contract.types';

// Task 13 fix round: `inspectLegacy`/`planRefresh` take `File[]` per
// the plan's Interfaces block — the test below drives a call with
// real `File` objects. The wasm boundary is mocked (node vitest
// cannot fetch the `?url` wasm; the browser suite drives the same
// call unmocked) — what is asserted is the File→bytes conversion and
// `dataFile` picking the data file over the `.prj` companion.
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
    const messages = t.errors as Record<string, string | undefined>;
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
      'commitLegacy',
      'planRefresh',
      'commitRefresh',
      'openSession',
      'applyCommand',
      'undoLast',
      'runProgram',
      'writeSession',
    ]) {
      expect(typeof a[name], `api.${name} missing`).toBe('function');
    }
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
