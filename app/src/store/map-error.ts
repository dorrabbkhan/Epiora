// Maps raw sqlite/DOM failures onto StoreError codes. Pure — kept in its
// own module so unit tests exercise it without the wasm / `?raw` module
// graph of opfs.ts.

import type { Sqlite3Static } from '@sqlite.org/sqlite-wasm';
import { StoreError } from './types';

interface SqliteErrorLike {
  resultCode: number;
  message?: string;
}

function isSqliteError(e: unknown): e is SqliteErrorLike {
  return (
    typeof e === 'object' &&
    e !== null &&
    'resultCode' in e &&
    typeof (e as SqliteErrorLike).resultCode === 'number'
  );
}

// WebKit raises `InvalidStateError` (not `NoModificationAllowedError`)
// when a second context opens a held sahpool — but only during pool /
// database acquisition is that name lock-significant. Everywhere else it
// is a generic DOMException and maps to `sql`.
export function mapError(
  sqlite3: Sqlite3Static,
  e: unknown,
  context: 'acquire' | 'run',
): StoreError {
  if (e instanceof StoreError) return e;
  const domName = e instanceof DOMException ? e.name : null;
  const message =
    (domName === null ? '' : `${domName}: `) +
    (e instanceof Error ? e.message : String(e));
  const rc = isSqliteError(e) ? e.resultCode : null;
  if (
    domName === 'QuotaExceededError' ||
    rc === sqlite3.capi.SQLITE_FULL ||
    /quota|storage.{0,20}(full|exceed)|no.{0,10}space/i.test(message)
  ) {
    return new StoreError('quota_exceeded', message, { cause: e });
  }
  if (
    domName === 'NoModificationAllowedError' ||
    (domName === 'InvalidStateError' && context === 'acquire') ||
    rc === sqlite3.capi.SQLITE_BUSY ||
    /Access Handles cannot|already.{0,20}(open|use|lock)/i.test(message)
  ) {
    return new StoreError('locked_elsewhere', message, { cause: e });
  }
  return new StoreError('sql', message, { cause: e });
}
