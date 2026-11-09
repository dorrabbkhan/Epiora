// RPC contract tests (v0.3 plan Task 9, spec §5.3/§11): every detail
// code the worker can put on the wire — from the Rust bindings'
// `code()` lists, `StoreError` and the worker-only codes in errors.ts —
// has a user-facing message, and the API's types are exactly the
// generated binding types (checked structurally in contract.types.ts).

import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { t } from '../ui/strings';
import {
  ENGINE_ERROR_CODES,
  IMPORT_ERROR_CODES,
  STORE_ERROR_CODES,
  WORKER_ERROR_CODES,
} from './errors';
import { TYPES_CHECKED } from './contract.types';

const ALL_CODES = [
  ...IMPORT_ERROR_CODES,
  ...ENGINE_ERROR_CODES,
  ...STORE_ERROR_CODES,
  ...WORKER_ERROR_CODES,
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
  });

  it('api_types_match_bindings', () => {
    // The real check runs in `tsc --noEmit`: contract.types.ts assigns
    // the generated binding types to the API's parameter and return
    // types both ways. This asserts the file is part of the suite.
    expect(TYPES_CHECKED).toBe(true);
  });
});
