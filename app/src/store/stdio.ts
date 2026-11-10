// Node-side test backend (v0.3 plan Task 14): `openStdioStore` spawns
// the test-only `store-stdio` binary and speaks its JSON-lines
// protocol, so the shared conformance suite runs against native
// SQLite3MultipleCiphers — never shipped in the app bundle.
//
// Wire values are `{ "t": "i"|"r"|"s"|"b"|"null", "v"? }`: integers as
// decimal strings (i64 lossless), blobs base64 — the mirror of
// `WireValue` in store-stdio.rs.

import { spawn, type ChildProcess } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { migrate } from './migrate';
import { StoreError, type Row, type SqlValue, type Store, type Tx } from './types';

// Vitest runs this project with root `app/`; the binary lives in the
// src-tauri workspace's debug target (built by check-app.sh / CI).
const BIN = join(process.cwd(), 'src-tauri', 'target', 'debug', 'store-stdio');
const MIGRATIONS = [
  {
    version: 1,
    sql: readFileSync(
      join(process.cwd(), '..', 'store', 'migrations', '0001_init.sql'),
      'utf8',
    ),
  },
];

interface WireValue {
  t: 'i' | 'r' | 's' | 'b' | 'null';
  v?: string | number;
}

type WireCell = [string, WireValue];

interface Response {
  id: number;
  ok: boolean;
  code?: string;
  message?: string;
  rows?: WireCell[][];
}

function encodeParam(v: SqlValue): WireValue {
  if (v === null) return { t: 'null' };
  if (typeof v === 'bigint') return { t: 'i', v: v.toString() };
  if (typeof v === 'number') {
    return Number.isInteger(v) ? { t: 'i', v: String(v) } : { t: 'r', v };
  }
  if (typeof v === 'string') return { t: 's', v };
  return { t: 'b', v: Buffer.from(v).toString('base64') };
}

function decodeValue(w: WireValue): SqlValue {
  switch (w.t) {
    case 'null':
      return null;
    case 'i': {
      const n = BigInt(String(w.v));
      return n > BigInt(Number.MAX_SAFE_INTEGER) ||
        n < BigInt(-Number.MAX_SAFE_INTEGER)
        ? n
        : Number(n);
    }
    case 'r':
      return Number(w.v);
    case 's':
      return String(w.v);
    case 'b':
      return new Uint8Array(Buffer.from(String(w.v), 'base64'));
  }
}

/** Spawns `store-stdio`, opens `path` with `keyHex`, then applies the
 *  shared migrations — one process (connection) per call, same database
 *  across calls, mirroring `openOpfsStore`'s contract. */
export async function openStdioStore(
  path: string,
  keyHex: string,
): Promise<Store> {
  const child: ChildProcess = spawn(BIN, [], {
    stdio: ['pipe', 'pipe', 'inherit'],
  });
  let nextId = 0;
  let buf = '';
  const pending = new Map<
    number,
    { resolve: (r: Response) => void; reject: (e: Error) => void }
  >();

  const failAll = (e: Error) => {
    for (const p of pending.values()) p.reject(e);
    pending.clear();
  };
  child.on('error', (e) => failAll(e));
  child.on('exit', (code) => {
    if (pending.size > 0) {
      failAll(new StoreError('sql', `store-stdio exited (${code})`));
    }
  });
  child.stdout?.on('data', (chunk: Buffer) => {
    buf += chunk.toString('utf8');
    for (;;) {
      const nl = buf.indexOf('\n');
      if (nl < 0) break;
      const line = buf.slice(0, nl);
      buf = buf.slice(nl + 1);
      if (line.trim() === '') continue;
      const res = JSON.parse(line) as Response;
      const p = pending.get(res.id);
      if (p !== undefined) {
        pending.delete(res.id);
        p.resolve(res);
      }
    }
  });

  const send = (op: string, extra: Record<string, unknown> = {}) =>
    new Promise<Response>((resolve, reject) => {
      const id = nextId++;
      pending.set(id, { resolve, reject });
      child.stdin?.write(JSON.stringify({ id, op, ...extra }) + '\n');
    });

  const call = async (
    op: string,
    extra: Record<string, unknown> = {},
  ): Promise<Response> => {
    const res = await send(op, extra);
    if (res.ok) return res;
    throw new StoreError(
      (res.code as StoreError['code']) ?? 'sql',
      res.message ?? 'store-stdio error',
    );
  };

  let closed = false;
  const guard = () => {
    if (closed) throw new StoreError('sql', 'store is closed');
  };

  const tx: Tx = {
    exec: async (sql, params) => {
      guard();
      await call('exec', { sql, params: params?.map(encodeParam) });
    },
    query: async (sql, params) => {
      guard();
      const res = await call('query', {
        sql,
        params: params?.map(encodeParam),
      });
      return (res.rows ?? []).map((cells) => {
        const row: Row = {};
        for (const [name, v] of cells) row[name] = decodeValue(v);
        return row;
      });
    },
  };

  const store: Store = {
    exec: tx.exec,
    query: tx.query,
    transaction: async <T>(fn: (tx: Tx) => Promise<T>): Promise<T> => {
      guard();
      await call('begin');
      let result: T;
      try {
        result = await fn(tx);
      } catch (e) {
        await call('rollback').catch(() => {});
        throw e;
      }
      try {
        await call('commit');
      } catch (e) {
        await call('rollback').catch(() => {});
        throw e;
      }
      return result;
    },
    close: async () => {
      if (closed) return;
      closed = true;
      await call('close').catch(() => {});
      // The binary exits after the close reply; kill as a fallback so a
      // hung child cannot keep the test alive.
      setTimeout(() => child.kill('SIGKILL'), 2000).unref();
    },
    __simulateQuotaOnNextWrite: () => {
      // Test hook: `store-stdio` fails the next mutating statement with
      // `quota_exceeded` (op `test_fail_next_write`, `stdio` feature).
      void send('test_fail_next_write').catch(() => {});
    },
  };

  try {
    await call('open', { path, key_hex: keyHex });
    await store.exec('PRAGMA foreign_keys = ON');
    await migrate(store, MIGRATIONS);
  } catch (e) {
    await store.close();
    throw e;
  }
  return store;
}
