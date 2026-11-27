import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import initLegacy, * as legacyWasm from '@legacy/epiora_legacy_wasm';
import { columnKind, encodeTableEptb, type EncColumn } from './legacy-eptb';
import { snapshotColumns } from './snapshot-codec';

describe('plain legacy binary columns', () => {
  it('eptb_binary_column_is_listed_and_absent_from_the_plain_snapshot', async () => {
    await initLegacy(
      readFileSync(resolve(process.cwd(), '../crates/epiora-legacy-wasm/pkg/epiora_legacy_wasm_bg.wasm')),
    );
    const columns: EncColumn[] = [
      { name: 'id', decl: 'integer', cells: [1, 2] },
      { name: 'payload', decl: 'blob', cells: [new Uint8Array([1, 2]), new Uint8Array([3, 4])] },
      { name: 'note', decl: 'text', cells: ['first', 'second'] },
    ];
    const h = legacyWasm.legacyNew();
    try {
      legacyWasm.legacyPush(
        h,
        'measurements',
        encodeTableEptb('measurements', columns, columns.map(columnKind)),
      );

      expect(legacyWasm.legacyPlainBinaryColumns(h, ['measurements'])).toEqual([
        {
          table: 'measurements',
          column: 'payload',
          reason: 'binary data not imported',
        },
      ]);
      const mapped = legacyWasm.legacyPlainTables(h, ['measurements']) as {
        snapshots: Uint8Array[];
      };
      expect(snapshotColumns(mapped.snapshots[0]!).map((c) => c.name)).toEqual(['id', 'note']);
    } finally {
      h.free();
    }
  });
});
