// SQL Server provenance across the real UI + worker path (fix round 4):
// sentinel connection values are entered in the form, the bridge returns
// the committed sample EPTB batches, the real worker imports them, and
// every store table plus the migration report is scanned for the
// sentinels. The Tauri bridge itself is mocked because browser tests
// have no SQL Server; everything after it is the production path.
import { createElement } from 'react';
import type { ReactNode } from 'react';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { afterAll, describe, expect, it, vi } from 'vitest';
import { SqlServerForm } from './SqlServerForm';
import { LegacyPreview } from './LegacyPreview';
import { dataWorker, workerName } from '../worker/client';
import { openHarness, terminateSpawned } from '../worker/integration-helpers';
import type { SqlValue } from '../store/types';

const bridgeCalls = vi.hoisted(() => ({ conns: [] as unknown[] }));
const navigation = vi.hoisted(() => ({
  calls: [] as { params?: { previewId?: string; project?: string } }[],
}));

vi.mock('@tanstack/react-router', async () => {
  return {
    useNavigate: () => (args: { params?: { previewId?: string; project?: string } }) => {
      navigation.calls.push(args);
      return Promise.resolve();
    },
    Link: ({ children }: { children?: ReactNode }) => children,
  };
});

const batchUrls = import.meta.glob('../../../crates/epiora-mdb/tests/data/batches/sample/*.eptb', {
  eager: true,
  query: '?url',
  import: 'default',
}) as Record<string, string>;
const tables = Object.entries(batchUrls).map(([path, url]) => ({
  name: /\/([^/]+)\.eptb$/.exec(path)![1]!,
  url,
}));

vi.mock('../tauri/sqlserver', () => ({
  legacySqlServerTables: async (conn: unknown) => {
    bridgeCalls.conns.push(conn);
    return tables.map((t) => t.name);
  },
  legacySqlServerBatches: async (_conn: unknown, table: string) => {
    const urls = tables.filter((t) => t.name === table).map((t) => t.url);
    return Promise.all(
      urls.map(async (url) => new Uint8Array(await (await fetch(url)).arrayBuffer())),
    );
  },
  sqlErrorText: () => 'sqlserver_query',
}));

const sentinel = {
  host: 'sentinel-host-a1b2c3d4e5',
  database: 'sentinel-database-f6e7d8c9b0',
  user: 'sentinel-user-1122334455',
  password: 'sentinel-password-6677889900',
};

function valueText(value: SqlValue): string {
  if (value instanceof Uint8Array) return new TextDecoder().decode(value);
  if (value === null) return '';
  if (typeof value === 'object') return JSON.stringify(value);
  return String(value);
}

describe('SQL Server connection provenance', () => {
  afterAll(() => {
    dataWorker().worker.terminate();
    terminateSpawned();
  });

  it('imports_without_persisting_connection_fields', async () => {
    const first = render(createElement(SqlServerForm));
    fireEvent.change(screen.getByLabelText('Server'), { target: { value: sentinel.host } });
    fireEvent.change(screen.getByLabelText('Database'), { target: { value: sentinel.database } });
    fireEvent.change(screen.getByLabelText('User'), { target: { value: sentinel.user } });
    fireEvent.change(screen.getByLabelText('Password'), { target: { value: sentinel.password } });
    fireEvent.click(screen.getByRole('button', { name: 'Read project' }));

    await waitFor(() => expect(navigation.calls.length).toBe(1), { timeout: 60_000 });
    const previewId = navigation.calls[0]!.params!.previewId!;
    expect(bridgeCalls.conns[0]).toMatchObject({
      host: sentinel.host,
      database: sentinel.database,
      auth: { sql: { user: sentinel.user, password: sentinel.password } },
    });
    first.unmount();

    const queryClient = new QueryClient();
    const second = render(
      createElement(
        QueryClientProvider,
        { client: queryClient },
        createElement(LegacyPreview, { previewId }),
      ),
    );
    const confirm = await screen.findByRole('button', { name: 'Confirm import' }, { timeout: 60_000 });
    fireEvent.click(confirm);
    await waitFor(() => expect(navigation.calls.length).toBe(2), { timeout: 60_000 });
    const projectId = navigation.calls[1]!.params!.project!;
    second.unmount();

    // Release the production worker's OPFS/lock handle, then inspect the
    // same store through the raw-SQL conformance harness.
    dataWorker().worker.terminate();
    const store = await openHarness(workerName.split(':')[0]!);
    try {
      const tableRows = await store.query(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
      );
      const sentinels = Object.values(sentinel);
      for (const row of tableRows) {
        const table = String(row.name).replaceAll('"', '""');
        const rows = await store.query(`SELECT * FROM "${table}"`);
        for (const r of rows) {
          for (const value of Object.values(r)) {
            const text = valueText(value);
            for (const needle of sentinels) expect(text).not.toContain(needle);
          }
        }
      }
      const reports = await store.query('SELECT json FROM migration_reports WHERE project_id = ?', [projectId]);
      expect(reports.length).toBeGreaterThan(0);
      for (const report of reports) {
        for (const needle of sentinels) expect(String(report.json)).not.toContain(needle);
      }
    } finally {
      await store.close();
    }
  }, 120_000);
});
