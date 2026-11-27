import { afterEach, describe, expect, it } from 'vitest';
import {
  fixture,
  openHarness,
  spawn,
  terminateSpawned,
} from './integration-helpers';
import { RpcError } from './errors';
import originalDbUrl from '../../../fixtures/legacy/samples/ei7/Sample/SampleDB.db?url';
import plainMdbUrl from '../../../fixtures/legacy/samples/ei3/HospitalPoints.mdb?url';

afterEach(terminateSpawned);

async function rpcDetail(promise: Promise<unknown>): Promise<string | null> {
  try {
    await promise;
    return null;
  } catch (e) {
    return e instanceof RpcError ? e.detail : String(e);
  }
}

async function expectNothingWritten(db: string): Promise<void> {
  terminateSpawned();
  const store = await openHarness(db);
  try {
    for (const table of [
      'projects',
      'datasets',
      'dataset_versions',
      'records',
      'changes',
      'imports',
      'migration_reports',
    ]) {
      const rows = await store.query(`SELECT COUNT(*) AS n FROM ${table}`);
      expect(Number(rows[0]!.n), table).toBe(0);
    }
  } finally {
    await store.close();
  }
}

describe('legacy import cancellation before commit', () => {
  it('cancel_before_project_commit_first_yield_writes_nothing', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const bytes = await fixture(originalDbUrl);
    const preview = await api.inspectLegacy([
      new File([new Uint8Array(bytes)], 'SampleDB.db'),
    ]);

    await api.beginLegacyImport(preview.previewId);
    await api.cancelImport(preview.previewId);
    expect(await rpcDetail(api.commitLegacy(preview.previewId, () => {}))).toBe('cancelled');
    await expectNothingWritten(db);
  });

  it('cancel_before_plain_commit_first_yield_writes_nothing', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);
    const bytes = await fixture(plainMdbUrl);
    const preview = await api.inspectPlainLegacy([
      new File([new Uint8Array(bytes)], 'HospitalPoints.mdb'),
    ]);

    await api.beginLegacyImport(preview.previewId);
    await api.cancelImport(preview.previewId);
    expect(await rpcDetail(api.commitPlainLegacy(preview.previewId, ['hosppts'], () => {}))).toBe(
      'cancelled',
    );
    await expectNothingWritten(db);
  });

  it('begin_legacy_import_unknown_preview_is_session_not_found', async () => {
    const db = `t-${crypto.randomUUID()}`;
    const api = spawn(db);

    expect(await rpcDetail(api.beginLegacyImport(crypto.randomUUID()))).toBe('session_not_found');
  });
});
