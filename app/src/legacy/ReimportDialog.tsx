// `/p/:project/reimport` (v1.1 spec §4.5/§7.1): a newer copy of the
// same legacy project is diffed keyed on the stable record key. The
// screen shows per-view record and schema changes before the one
// transaction that creates the new versions.
import { useState } from 'react';
import { Link, useNavigate } from '@tanstack/react-router';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import {
  Button,
  Cell,
  Column,
  FileTrigger,
  Row,
  Table,
  TableBody,
  TableHeader,
} from '../ui/components';
import { dataApi } from '../worker/client';
import { RpcError } from '../worker/errors';
import { legacyT } from './strings';
import styles from './legacy.module.css';

interface DiffFieldChange {
  record_id?: unknown;
}

interface DiffPayload {
  added: unknown[];
  removed: unknown[];
  changed: DiffFieldChange[];
  unchanged: number;
  schema: {
    added_columns: string[];
    removed_columns: string[];
    type_changes: { name: string; from: string; to: string }[];
  };
}

function diffOf(outcome: unknown): DiffPayload | null {
  if (typeof outcome !== 'object' || outcome === null) return null;
  const changed = (outcome as { changed?: unknown }).changed;
  return typeof changed === 'object' && changed !== null ? (changed as DiffPayload) : null;
}

function outcomeText(outcome: unknown): string {
  if (typeof outcome === 'string') return outcome.replaceAll('_', ' ');
  return 'changed';
}

function changedRecords(diff: DiffPayload): number {
  return new Set(diff.changed.map((c) => String(c.record_id))).size;
}

function schemaText(diff: DiffPayload): string | null {
  const parts = [];
  if (diff.schema.added_columns.length > 0) {
    parts.push(`Added columns: ${diff.schema.added_columns.join(', ')}`);
  }
  if (diff.schema.removed_columns.length > 0) {
    parts.push(`Removed columns: ${diff.schema.removed_columns.join(', ')}`);
  }
  for (const change of diff.schema.type_changes) {
    parts.push(`Type change: ${change.name} ${change.from} → ${change.to}`);
  }
  return parts.length === 0 ? null : parts.join('; ');
}

export function ReimportDialog({ project }: { project: string }) {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const legacy = useQuery({
    queryKey: ['legacyProject', project],
    queryFn: () => dataApi.getLegacyProject(project),
  });
  const [preview, setPreview] = useState<Awaited<ReturnType<typeof dataApi.planRefresh>> | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const notRefreshable = legacy.data?.sourceFormat === 'ei7_sql_server';

  const inspect = async (files: FileList | null) => {
    if (files === null || files.length === 0) return;
    setBusy(true);
    setError(null);
    try {
      setPreview(await dataApi.planRefresh(project, Array.from(files)));
    } catch (e) {
      setError(e instanceof RpcError ? e.userMessage : e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const confirm = async () => {
    if (preview === null) return;
    setBusy(true);
    setError(null);
    try {
      await dataApi.commitRefresh(preview.previewId);
      sessionStorage.setItem(`epiora-reimported:${project}`, '1');
      await qc.invalidateQueries();
      await navigate({ to: '/p/$project', params: { project } });
    } catch (e) {
      setError(e instanceof RpcError ? e.userMessage : e instanceof Error ? e.message : String(e));
      setBusy(false);
    }
  };

  const rows = (preview?.plan.views ?? []).map((view, i) => {
    const diff = diffOf(view.outcome);
    return {
      id: `${view.name}-${i}`,
      name: view.name,
      outcome: outcomeText(view.outcome),
      added: diff?.added.length ?? 0,
      removed: diff?.removed.length ?? 0,
      changed: diff === null ? 0 : changedRecords(diff),
      schema: diff === null ? null : schemaText(diff),
    };
  });
  const anyChanged = rows.some((row) => row.outcome === 'changed' || row.outcome === 'new');

  return (
    <div>
      <h1>{legacyT.reimportTitle}</h1>
      <p>{legacyT.reimportIntro}</p>
      {notRefreshable ? (
        <p role="alert">SQL Server projects are not refreshable; choose a newer `.mdb` or `.db` copy of a file-backed project.</p>
      ) : (
        <FileTrigger onSelect={(files) => void inspect(files)}>
          <Button isDisabled={busy}>{legacyT.chooseNewer}</Button>
        </FileTrigger>
      )}
      {error !== null && <p role="alert">{error}</p>}
      {preview !== null && (
        <section className={styles.section}>
          <h2>{legacyT.diffTitle}</h2>
          {!anyChanged && <p>{legacyT.noChanges}</p>}
          <Table caption={legacyT.diffTitle} aria-label={legacyT.diffTitle}>
            <TableHeader>
              <Column isRowHeader>View</Column>
              <Column>Outcome</Column>
              <Column>Added</Column>
              <Column>Removed</Column>
              <Column>Changed</Column>
            </TableHeader>
            <TableBody items={rows}>
              {(row) => (
                <Row id={row.id}>
                  <Cell>{row.name}</Cell>
                  <Cell>{row.outcome}</Cell>
                  <Cell>{row.added === 0 ? '0' : `${row.added} added`}</Cell>
                  <Cell>{row.removed === 0 ? '0' : `${row.removed} removed`}</Cell>
                  <Cell>{row.changed === 0 ? '0' : `${row.changed} changed`}</Cell>
                </Row>
              )}
            </TableBody>
          </Table>
          {rows.map((row) => row.schema !== null && (
            <p key={row.id} className={styles.schemaNote}>{row.schema}</p>
          ))}
          <div className={styles.panel}>
            <Button onPress={() => void confirm()} isDisabled={busy}>
              {legacyT.confirmReimport}
            </Button>
            <Link to="/p/$project" params={{ project }}>{legacyT.cancel}</Link>
          </div>
        </section>
      )}
    </div>
  );
}
