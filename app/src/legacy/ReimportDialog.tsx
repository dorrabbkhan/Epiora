// `/p/:project/reimport` (v1.1 spec §4.5/§7.1): only a legacy project
// can refresh, and only from a newer copy of the same legacy project.
// The preview shows per-view counts plus bounded record-level samples
// before one transaction creates the new versions.
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
import type { RefreshDiff } from '../worker/legacy';
import { legacyT } from './strings';
import { t } from '../ui/strings';
import styles from './legacy.module.css';

const SAMPLE_LIMIT = 20;

function diffOf(outcome: unknown): RefreshDiff | null {
  if (typeof outcome !== 'object' || outcome === null) return null;
  const changed = (outcome as { changed?: unknown }).changed;
  return typeof changed === 'object' && changed !== null ? (changed as RefreshDiff) : null;
}

function outcomeText(outcome: unknown): string {
  if (typeof outcome === 'string') return outcome.replaceAll('_', ' ');
  return 'changed';
}

function changedRecords(diff: RefreshDiff): number {
  return new Set(diff.changed.map((c) => c.record_id)).size;
}

function renderValue(v: unknown): string {
  if (v === null || v === undefined) return '—';
  if (typeof v === 'object' && 'value' in v) return String((v as { value: unknown }).value);
  return String(v);
}

function schemaText(diff: RefreshDiff): string | null {
  const parts = [];
  if (diff.schema.added_columns.length > 0) {
    parts.push(t.import.addedColumns(diff.schema.added_columns));
  }
  if (diff.schema.removed_columns.length > 0) {
    parts.push(t.import.removedColumns(diff.schema.removed_columns));
  }
  if (diff.schema.type_changes.length > 0) {
    parts.push(t.import.typeChanges(diff.schema.type_changes));
  }
  return parts.length === 0 ? null : parts.join('; ');
}

function RecordSamples({ viewName, diff }: { viewName: string; diff: RefreshDiff }) {
  const added = diff.added.slice(0, SAMPLE_LIMIT);
  const removed = diff.removed.slice(0, SAMPLE_LIMIT);
  const changed = diff.changed.slice(0, SAMPLE_LIMIT);
  if (added.length === 0 && removed.length === 0 && changed.length === 0) return null;
  return (
    <section className={styles.section} aria-label={`${legacyT.recordSamplesTitle} ${viewName}`}>
      <h3>{legacyT.recordSamplesTitle}: {viewName}</h3>
      {added.length > 0 && (
        <>
          <h4>{legacyT.addedRecordsTitle}</h4>
          <ul>
            {added.map((row, i) => (
              <li key={`a-${row}`}>{diff.added_records?.[i] ?? legacyT.addedRow(row)}</li>
            ))}
          </ul>
        </>
      )}
      {removed.length > 0 && (
        <>
          <h4>{legacyT.removedRecordsTitle}</h4>
          <ul>
            {removed.map((id) => <li key={`r-${id}`}>{id}</li>)}
          </ul>
        </>
      )}
      {changed.length > 0 && (
        <>
          <h4>{legacyT.changedRecordsTitle}</h4>
          <ul>
            {changed.map((c, i) => (
              <li key={`c-${c.record_id}-${c.field}-${i}`}>
                {c.record_id} {c.field}: {renderValue(c.old)} → {renderValue(c.new)}
              </li>
            ))}
          </ul>
        </>
      )}
    </section>
  );
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
  const notLegacy = legacy.data === null;

  const inspect = async (files: FileList | null) => {
    if (files === null || files.length === 0 || notLegacy || notRefreshable) return;
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
      diff,
    };
  });
  const anyChanged = rows.some((row) => row.outcome === 'changed' || row.outcome === 'new');

  return (
    <div>
      <h1>{legacyT.reimportTitle}</h1>
      <p>{legacyT.reimportIntro}</p>
      {legacy.isPending && <p>{t.common.loading}</p>}
      {notLegacy && <p role="alert">{legacyT.notLegacyProject}</p>}
      {!notLegacy && notRefreshable && <p role="alert">{legacyT.sqlServerNotRefreshable}</p>}
      {!notLegacy && !notRefreshable && !legacy.isPending && (
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
              <Column isRowHeader>{legacyT.tableColumnView}</Column>
              <Column>{legacyT.tableColumnOutcome}</Column>
              <Column>{legacyT.tableColumnAdded}</Column>
              <Column>{legacyT.tableColumnRemoved}</Column>
              <Column>{legacyT.tableColumnChanged}</Column>
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
          {rows.map((row) => row.diff !== null && (
            <RecordSamples key={`${row.id}-samples`} viewName={row.name} diff={row.diff} />
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
