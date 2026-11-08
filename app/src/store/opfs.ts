// OPFS backend for the browser (v0.3 plan Task 8): sqlite-wasm's
// opfs-sahpool VFS. Worker-only — createSyncAccessHandle does not exist
// on the main thread — so UI code reaches this through the data worker.

import sqlite3InitModule, {
  type OpfsSAHPoolDatabase,
  type SAHPoolUtil,
  type Sqlite3Static,
} from '@sqlite.org/sqlite-wasm';
import sqlite3WasmUrl from '@sqlite.org/sqlite-wasm/sqlite3.wasm?url';
import initSql from '../../../store/migrations/0001_init.sql?raw';
import { mapError } from './map-error';
import { migrate } from './migrate';
import { StoreError, type Row, type SqlValue, type Store, type Tx } from './types';

const MIGRATIONS: { version: number; sql: string }[] = [{ version: 1, sql: initSql }];

const WRITE_RE =
  /^\s*(?:insert|update|delete|replace|create|drop|alter|vacuum|reindex|attach|detach|truncate)\b/i;

let sqlite3P: Promise<Sqlite3Static> | null = null;
let poolP: Promise<SAHPoolUtil> | null = null;

function getSqlite(): Promise<Sqlite3Static> {
  // The bundled .d.ts declares init() with no parameters, but the
  // implementation accepts an optional config object; locateFile must
  // point at the ?url-rewritten wasm asset under Vite.
  const init = sqlite3InitModule as unknown as (cfg?: {
    locateFile?: (file: string) => string;
  }) => Promise<Sqlite3Static>;
  sqlite3P ??= init({ locateFile: () => sqlite3WasmUrl });
  return sqlite3P;
}

function getPool(): Promise<SAHPoolUtil> {
  poolP ??= getSqlite()
    .then((sqlite3) =>
      sqlite3.installOpfsSAHPoolVfs(
        // The bundled .d.mts omits this option, but without it a retry
        // re-throws the internally cached init failure.
        { forceReinitIfPreviouslyFailed: true } as Parameters<
          Sqlite3Static['installOpfsSAHPoolVfs']
        >[0],
      ),
    )
    .catch((e: unknown) => {
      poolP = null;
      throw e;
    });
  return poolP;
}

function makeStore(sqlite3: Sqlite3Static, db: OpfsSAHPoolDatabase): Store {
  let closed = false;
  let failNextWrite = false;

  const guard = () => {
    if (closed) throw new StoreError('sql', 'store is closed');
  };

  const execRaw = (sql: string, params?: SqlValue[]): void => {
    db.exec(params === undefined ? { sql } : { sql, bind: params });
  };

  const tx: Tx = {
    exec: (sql, params) => {
      try {
        guard();
        if (failNextWrite && WRITE_RE.test(sql)) {
          failNextWrite = false;
          throw new StoreError('quota_exceeded', 'simulated quota exhaustion on write');
        }
        execRaw(sql, params);
        return Promise.resolve();
      } catch (e) {
        return Promise.reject(mapError(sqlite3, e, 'run'));
      }
    },
    query: (sql, params) => {
      try {
        guard();
        const rows = db.exec({
          sql,
          ...(params === undefined ? {} : { bind: params }),
          rowMode: 'object',
          returnValue: 'resultRows',
        }) as Record<string, SqlValue>[];
        return Promise.resolve(rows as Row[]);
      } catch (e) {
        return Promise.reject(mapError(sqlite3, e, 'run'));
      }
    },
  };

  return {
    exec: tx.exec,
    query: tx.query,
    transaction: async <T>(fn: (tx: Tx) => Promise<T>): Promise<T> => {
      guard();
      try {
        execRaw('BEGIN');
      } catch (e) {
        throw mapError(sqlite3, e, 'run');
      }
      let result: T;
      try {
        result = await fn(tx);
      } catch (e) {
        try {
          execRaw('ROLLBACK');
        } catch {
          // The connection may already have dropped the transaction.
        }
        throw e;
      }
      try {
        execRaw('COMMIT');
      } catch (e) {
        try {
          execRaw('ROLLBACK');
        } catch {
          // The connection may already have dropped the transaction.
        }
        throw mapError(sqlite3, e, 'run');
      }
      return result;
    },
    close: () => {
      if (!closed) {
        closed = true;
        db.close();
      }
      return Promise.resolve();
    },
    __simulateQuotaOnNextWrite: () => {
      failNextWrite = true;
    },
  };
}

/**
 * Opens (creating if needed) the OPFS database `name`, turns on
 * foreign keys and applies pending migrations in one transaction.
 */
export async function openOpfsStore(name: string): Promise<Store> {
  const sqlite3 = await getSqlite().catch((e: unknown) => {
    const message = e instanceof Error ? e.message : String(e);
    throw new StoreError('sql', `sqlite-wasm init failed: ${message}`, { cause: e });
  });
  let db: OpfsSAHPoolDatabase;
  try {
    const pool = await getPool();
    db = new pool.OpfsSAHPoolDb(name);
  } catch (e) {
    throw mapError(sqlite3, e, 'acquire');
  }
  const store = makeStore(sqlite3, db);
  try {
    await store.exec('PRAGMA foreign_keys = ON');
    await migrate(store, MIGRATIONS);
  } catch (e) {
    await store.close();
    throw e;
  }
  return store;
}
