// Unit-level checks for mapError's InvalidStateError context: only the
// sahpool pool/database acquisition step treats that DOMException name
// as lock-significant (WebKit raises it for a held pool); everywhere
// else it is a generic error and maps to `sql`.

import type { Sqlite3Static } from '@sqlite.org/sqlite-wasm';
import { describe, expect, it } from 'vitest';
import { mapError } from './map-error';
import { StoreError } from './types';

// mapError only reads capi.SQLITE_FULL / capi.SQLITE_BUSY, so a stub
// stands in for the wasm module (jsdom has DOMException).
const sqlite3 = {
  capi: { SQLITE_FULL: 13, SQLITE_BUSY: 5 },
} as unknown as Sqlite3Static;

describe('mapError context', () => {
  it('invalid_state_error_maps_to_locked_elsewhere_only_on_acquire', () => {
    const e = new DOMException(
      'The object is in an invalid state.',
      'InvalidStateError',
    );
    const acquire = mapError(sqlite3, e, 'acquire');
    const run = mapError(sqlite3, e, 'run');
    expect(acquire).toBeInstanceOf(StoreError);
    expect(acquire.code).toBe('locked_elsewhere');
    expect(run).toBeInstanceOf(StoreError);
    expect(run.code).toBe('sql');
  });

  it('other_lock_and_quota_names_map_the_same_in_both_contexts', () => {
    const noMod = new DOMException('No modification allowed', 'NoModificationAllowedError');
    const quota = new DOMException('quota exceeded', 'QuotaExceededError');
    expect(mapError(sqlite3, noMod, 'acquire').code).toBe('locked_elsewhere');
    expect(mapError(sqlite3, noMod, 'run').code).toBe('locked_elsewhere');
    expect(mapError(sqlite3, quota, 'run').code).toBe('quota_exceeded');
  });
});
