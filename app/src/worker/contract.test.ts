// RPC contract tests (v0.3 plan Task 9, spec §5.3/§11): every detail
// code the worker can put on the wire — from the Rust bindings'
// `code()` lists, `StoreError` and the worker-only codes in errors.ts —
// has a user-facing message, and the API's types are exactly the
// generated binding types (checked structurally in contract.types.ts).

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

describe('worker RPC contract', () => {
  it('every_detail_code_has_a_user_message', () => {
    const messages = t.errors as Record<string, string | undefined>;
    for (const code of ALL_CODES) {
      expect(messages[code], `missing userMessage for ${code}`).toBeTypeOf('string');
      expect(messages[code]!.length).toBeGreaterThan(0);
    }
  });

  it('api_types_match_bindings', () => {
    // The real check runs in `tsc --noEmit`: contract.types.ts assigns
    // the generated binding types to the API's parameter and return
    // types both ways. This asserts the file is part of the suite.
    expect(TYPES_CHECKED).toBe(true);
  });
});
