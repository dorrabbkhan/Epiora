// Main-thread side of the Tauri store relay (v0.3 plan Task 14):
// Tauri's IPC does not reach Web Workers, so the worker's relay store
// (app/src/store/relay.ts) posts `{ epiora: 'store-relay', id, op, sql,
// params }` messages here; each is invoked against the matching Rust
// store command and the result posted back. Bodies cross to Rust as one
// length-prefixed binary frame — blob params and result cells travel as
// raw bytes (`Uint8Array` request body, `tauri::ipc::Response`), never
// JSON arrays.

import { invoke } from '@tauri-apps/api/core';
import { RELAY_TAG, type RelayOp, type RelayRequest } from '../store/relay';
import type { Row, SqlValue } from '../store/types';

const COMMANDS: Record<RelayOp, string> = {
  exec: 'store_exec',
  query: 'store_query',
  begin: 'store_tx_begin',
  commit: 'store_tx_commit',
  rollback: 'store_tx_rollback',
};

function encodeBody(sql: string | undefined, params: SqlValue[] | undefined): Uint8Array {
  if (sql === undefined) return new Uint8Array(0);
  const enc = new TextEncoder();
  const chunks: Uint8Array[] = [];
  const u32 = (n: number) => {
    const b = new Uint8Array(4);
    new DataView(b.buffer).setUint32(0, n, true);
    chunks.push(b);
  };
  const sqlBytes = enc.encode(sql);
  u32(sqlBytes.length);
  chunks.push(sqlBytes);
  u32(params?.length ?? 0);
  for (const p of params ?? []) {
    if (p === null) {
      chunks.push(new Uint8Array([0]));
    } else if (typeof p === 'bigint') {
      const b = new Uint8Array(9);
      b[0] = 1;
      new DataView(b.buffer).setBigInt64(1, p, true);
      chunks.push(b);
    } else if (typeof p === 'number' && Number.isInteger(p)) {
      const b = new Uint8Array(9);
      b[0] = 1;
      new DataView(b.buffer).setBigInt64(1, BigInt(p), true);
      chunks.push(b);
    } else if (typeof p === 'number') {
      const b = new Uint8Array(9);
      b[0] = 2;
      new DataView(b.buffer).setFloat64(1, p, true);
      chunks.push(b);
    } else if (typeof p === 'string') {
      const b = enc.encode(p);
      const h = new Uint8Array(5);
      h[0] = 3;
      new DataView(h.buffer).setUint32(1, b.length, true);
      chunks.push(h, b);
    } else {
      const h = new Uint8Array(5);
      h[0] = 4;
      new DataView(h.buffer).setUint32(1, p.length, true);
      chunks.push(h, p);
    }
  }
  const out = new Uint8Array(chunks.reduce((n, c) => n + c.length, 0));
  let off = 0;
  for (const c of chunks) {
    out.set(c, off);
    off += c.length;
  }
  return out;
}

interface Decoded {
  ok: boolean;
  code?: string;
  message?: string;
  rows?: Row[];
  blobs: ArrayBuffer[];
}

function decodeResponse(buf: Uint8Array): Decoded {
  const view = new DataView(buf.buffer, buf.byteOffset, buf.byteLength);
  const dec = new TextDecoder();
  let pos = 0;
  const take = (n: number): Uint8Array => {
    const s = buf.subarray(pos, pos + n);
    pos += n;
    return s;
  };
  const blobs: ArrayBuffer[] = [];
  if (take(1)[0] === 1) {
    const codeLen = view.getUint16(pos, true);
    pos += 2;
    const code = dec.decode(take(codeLen));
    const msgLen = view.getUint32(pos, true);
    pos += 4;
    const message = dec.decode(take(msgLen));
    return { ok: false, code, message, blobs };
  }
  const nrows = view.getUint32(pos, true);
  pos += 4;
  const rows: Row[] = [];
  for (let r = 0; r < nrows; r++) {
    const ncols = view.getUint32(pos, true);
    pos += 4;
    const row: Row = {};
    for (let c = 0; c < ncols; c++) {
      const nameLen = view.getUint16(pos, true);
      pos += 2;
      const name = dec.decode(take(nameLen));
      const tag = take(1)[0]!;
      if (tag === 0) {
        row[name] = null;
      } else if (tag === 1) {
        const n = view.getBigInt64(pos, true);
        pos += 8;
        row[name] =
          n > BigInt(Number.MAX_SAFE_INTEGER) || n < BigInt(-Number.MAX_SAFE_INTEGER)
            ? n
            : Number(n);
      } else if (tag === 2) {
        row[name] = view.getFloat64(pos, true);
        pos += 8;
      } else if (tag === 3) {
        const len = view.getUint32(pos, true);
        pos += 4;
        row[name] = dec.decode(take(len));
      } else {
        const len = view.getUint32(pos, true);
        pos += 4;
        const bytes = take(len);
        row[name] = bytes;
        blobs.push(bytes.buffer as ArrayBuffer);
      }
    }
    rows.push(row);
  }
  return { ok: true, rows, blobs };
}

async function handle(worker: Worker, req: RelayRequest): Promise<void> {
  const cmd = COMMANDS[req.op];
  try {
    const body = encodeBody(req.sql, req.params);
    const raw = (await invoke(cmd, body)) as ArrayBuffer;
    const res = decodeResponse(new Uint8Array(raw));
    worker.postMessage(
      {
        epiora: RELAY_TAG,
        id: req.id,
        ok: res.ok,
        code: res.code,
        message: res.message,
        rows: res.rows,
      },
      { transfer: res.blobs },
    );
  } catch (e) {
    worker.postMessage({
      epiora: RELAY_TAG,
      id: req.id,
      ok: false,
      code: 'sql',
      message: e instanceof Error ? e.message : String(e),
    });
  }
}

/** Relays the worker's store ops to the Rust commands. Call once at
 *  startup when `window.__TAURI_INTERNALS__` exists (main.tsx), before
 *  the worker can need the store. */
export function startTauriBridge(worker: Worker): void {
  // Convention flag: lets main-thread code tell the desktop shell is
  // driving the UI (the worker's marker is its `epiora-tauri:` name).
  (self as unknown as Record<string, unknown>).__EPIORA_TAURI__ = true;
  worker.addEventListener('message', (e: MessageEvent) => {
    const d = e.data as Partial<RelayRequest>;
    if (d?.epiora !== RELAY_TAG || typeof d.id !== 'string' || typeof d.op !== 'string') {
      return;
    }
    void handle(worker, d as RelayRequest);
  });
}
