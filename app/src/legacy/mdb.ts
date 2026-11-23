// Thin typed wrapper over the epiora-mdb wasm module
// (app/public/mdb/mdb.js + mdb.wasm, built by
// crates/epiora-mdb/build-wasm.sh from the same C sources as the
// native crate). Yields raw EPTB v1 batches; `epiora-legacy::batch`
// decodes them on the engine side (v1.1 Task 5).

/** Every failure is typed; `code` matches the Rust `MdbError` codes. */
export class MdbError extends Error {
  constructor(
    public readonly code: string,
    message: string,
    /** The Jet/ACE format detected before the failure, when known. */
    public readonly format?: string,
  ) {
    super(message);
    this.name = 'MdbError';
  }
}

/** An open database. `close()` frees the wasm-side handle and file. */
export interface MdbHandle {
  /** "JET3" | "JET4" — ACE files never open. */
  format(): string;
  /** User tables in catalog order. */
  tables(): string[];
  /**
   * EPTB v1 batches for `table`, `chunkRows` rows each (default
   * 50_000 — spec §9). Iterating to the end reads every row.
   */
  batches(table: string, chunkRows?: number): AsyncIterable<Uint8Array>;
  close(): void;
}

interface MdbModule {
  ccall(
    fn: string,
    returnType: string | null,
    argTypes: string[],
    args: unknown[],
  ): unknown;
  FS: {
    writeFile(path: string, data: Uint8Array): void;
    unlink(path: string): void;
  };
  UTF8ToString(ptr: number): string;
  getValue(ptr: number, type: string): number;
  HEAPU8: Uint8Array;
  _malloc(n: number): number;
  _free(p: number): void;
}

const ERROR_CODES: Record<number, string> = {
  1: 'mdb_corrupt',
  2: 'mdb_truncated',
  3: 'mdb_password_protected',
  4: 'mdb_unsupported_version',
  5: 'mdb_no_table',
  6: 'mdb_no_handle',
  7: 'mdb_encode',
  8: 'mdb_code_page_unsupported',
};

let counter = 0;

function errorAt(M: MdbModule, code: number): MdbError {
  const format = M.ccall('epiora_mdb_last_format', 'string', [], []) as string;
  const codeName = ERROR_CODES[code] ?? 'mdb_error';
  const messages: Record<string, string> = {
    mdb_corrupt: 'not a readable Jet database',
    mdb_truncated: `truncated ${format} database`,
    mdb_password_protected: `password-protected ${format} database`,
    mdb_unsupported_version: `unsupported database version: ${format}`,
    mdb_no_table: 'no such table',
    mdb_no_handle: 'bad database handle',
    mdb_encode: 'batch encode failed',
    mdb_code_page_unsupported: 'unsupported code page',
  };
  return new MdbError(codeName, messages[codeName] ?? `mdb error ${code}`, format);
}

/**
 * Opens an mdb/accdb byte blob. `codePage` is the single-byte page
 * JET3 text is decoded with (default "windows-1252"); it is recorded
 * in every batch header so the report can name it.
 *
 * `moduleUrl` overrides where the ES module is loaded from — tests
 * pass a file:// URL; the app uses the default under /mdb/.
 */
export async function openMdb(
  bytes: Uint8Array,
  codePage?: string,
  moduleUrl = '/mdb/mdb.js',
): Promise<MdbHandle> {
  const init = (await import(/* @vite-ignore */ moduleUrl)).default as (
    opts?: object,
  ) => Promise<MdbModule>;
  // locateFile: resolve the .wasm next to moduleUrl — the module's own
  // import.meta.url is a dead dev-server URL under vitest.
  const M = await init({
    locateFile: (file: string) => new URL(file, moduleUrl).href,
  });

  const vpath = `/epiora-mdb-${++counter}.mdb`;
  M.FS.writeFile(vpath, bytes);
  const h = M.ccall('epiora_mdb_open', 'number', ['string', 'string'], [vpath, codePage ?? '']) as number;
  if (h <= 0) {
    M.FS.unlink(vpath);
    throw errorAt(M, -h);
  }

  const close = () => {
    M.ccall('epiora_mdb_close', null, ['number'], [h]);
    try {
      M.FS.unlink(vpath);
    } catch {
      /* already gone */
    }
  };

  return {
    format: () => M.ccall('epiora_mdb_format', 'string', ['number'], [h]) as string,
    tables: () => {
      const ptr = M.ccall('epiora_mdb_tables', 'number', ['number'], [h]) as number;
      if (!ptr) throw errorAt(M, M.ccall('epiora_mdb_last_error', 'number', [], []) as number);
      const json = M.UTF8ToString(ptr);
      M._free(ptr);
      return JSON.parse(json) as string[];
    },
    batches: async function* (table: string, chunkRows = 50_000) {
      let offset = 0;
      for (;;) {
        const lenPtr = M._malloc(4);
        const ptr = M.ccall(
          'epiora_mdb_batch',
          'number',
          ['number', 'string', 'number', 'number', 'number'],
          [h, table, offset, chunkRows, lenPtr],
        ) as number;
        const len = M.getValue(lenPtr, 'i32');
        M._free(lenPtr);
        if (!ptr) {
          throw errorAt(M, M.ccall('epiora_mdb_last_error', 'number', [], []) as number);
        }
        const batch = M.HEAPU8.slice(ptr, ptr + len);
        M._free(ptr);
        if (batch.length === 0) return;
        yield batch;
        // EPTB header: u32 row_count at byte 6 — advance by the
        // rows actually emitted; a short batch means the table ended.
        const rows = new DataView(batch.buffer, batch.byteOffset).getUint32(6, true);
        if (rows < chunkRows) return;
        offset += rows;
      }
    },
    close,
  };
}
