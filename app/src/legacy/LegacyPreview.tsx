// Legacy project preview (v1.1 spec §7.1): every fact the import will
// write is visible before the first store write — views and their
// fields, links, programs, check-code block counts and the migration
// report. Confirm commits in one transaction; cancel uses the same
// `cancelImport` RPC as a CSV import and rolls the transaction back.
import { useMemo, useState } from 'react';
import { Link, useNavigate } from '@tanstack/react-router';
import { useQueryClient } from '@tanstack/react-query';
import {
  Button,
  Cell,
  Column,
  ProgressBar,
  Row,
  Table,
  TableBody,
  TableHeader,
} from '../ui/components';
import { dataApi } from '../worker/client';
import { RpcError } from '../worker/errors';
import type { ImportProgress } from '../worker/api';
import type { LegacyPreview as LegacyPreviewData, LegacyProjectJson } from '../worker/legacy';
import { legacyT } from './strings';
import styles from './legacy.module.css';

// `inspectLegacy` and the preview route are separate components, so
// the returned preview is held for the in-app navigation. A browser
// reload cannot rehydrate it (the worker RPC is one-shot); the route
// explains that and sends the user back instead of inventing a stale
// preview.
const cache = new Map<string, LegacyPreviewData>();

export function rememberLegacyPreview(preview: LegacyPreviewData): void {
  cache.set(preview.previewId, preview);
}

type Project = LegacyProjectJson;

function rowsWithId<T extends object>(rows: T[], prefix: string): (T & { id: string })[] {
  return rows.map((row, i) => ({ ...row, id: `${prefix}-${i}` }));
}

function FieldsTable({ view }: { view: Project['views'][number] }) {
  const label = legacyT.fields(view.name);
  const rows = useMemo(
    () => rowsWithId(view.fields, `field-${view.name}`),
    [view.fields, view.name],
  );
  return (
    <section className={styles.section}>
      <h3>{label}</h3>
      <div className={styles.tableWrap}>
        <Table caption={label} aria-label={label}>
          <TableHeader>
            <Column isRowHeader>Field</Column>
            <Column>Legacy type</Column>
            <Column>Epiora type</Column>
          </TableHeader>
          <TableBody items={rows}>
            {(field) => (
              <Row id={field.id}>
                <Cell>{field.name}</Cell>
                <Cell>{field.legacyType ?? '—'}</Cell>
                <Cell>{field.type}</Cell>
              </Row>
            )}
          </TableBody>
        </Table>
      </div>
    </section>
  );
}

export function LegacyPreview({ previewId }: { previewId: string }) {
  const navigate = useNavigate();
  const qc = useQueryClient();
  const cached = cache.get(previewId);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  if (cached === undefined) {
    return (
      <div className={styles.panel}>
        <p role="alert">{legacyT.missingPreview}</p>
        <Link to="/legacy">{legacyT.back}</Link>
      </div>
    );
  }
  const project = cached.project;
  const views = rowsWithId(project.views, 'view');
  const links = rowsWithId(project.links, 'link');
  const programs = rowsWithId(project.programs, 'program');
  const checkCode = rowsWithId(project.check_code, 'check');
  const report = rowsWithId(project.report, 'report');

  const confirm = async () => {
    setBusy(true);
    setError(null);
    // Render the cancel control before the first progress event; very
    // small projects otherwise complete before cancellation can even
    // be offered.
    setProgress({ stage: 'write', done: 0, total: Math.max(project.views.length + project.datasets.length, 1) });
    try {
      const projectId = await dataApi.commitLegacy(previewId, setProgress);
      await qc.invalidateQueries();
      cache.delete(previewId);
      await navigate({ to: '/p/$project', params: { project: projectId } });
    } catch (e) {
      setError(e instanceof RpcError && e.detail === 'cancelled' ? legacyT.cancelled : e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
      setProgress(null);
    }
  };

  return (
    <div>
      <h1>{legacyT.previewTitle(project.name)}</h1>
      <p>{legacyT.previewIntro}</p>
      {error !== null && <p role="alert">{error}</p>}
      {progress !== null && (
        <div className={styles.progress}>
          <ProgressBar
            aria-label={legacyT.writeProgress}
            value={progress.done}
            maxValue={Math.max(progress.total, 1)}
          />
          <Button onPress={() => void dataApi.cancelImport(previewId)}>{legacyT.cancel}</Button>
        </div>
      )}
      {progress === null && (
        <div className={styles.panel}>
          <Button onPress={() => void confirm()} isDisabled={busy}>
            {legacyT.confirm}
          </Button>
          <Link to="/legacy">{legacyT.cancel}</Link>
        </div>
      )}

      <section className={styles.section}>
        <h2>{legacyT.views}</h2>
        <Table caption={legacyT.views} aria-label={legacyT.views}>
          <TableHeader>
            <Column isRowHeader>View</Column>
            <Column>Records</Column>
            <Column>Deleted records</Column>
            <Column>Key column</Column>
          </TableHeader>
          <TableBody items={views}>
            {(view) => (
              <Row id={view.id}>
                <Cell>{view.name}</Cell>
                <Cell>{String(view.records)}</Cell>
                <Cell>{String(view.deleted_records)}</Cell>
                <Cell>{view.key_column || '—'}</Cell>
              </Row>
            )}
          </TableBody>
        </Table>
      </section>
      {project.views.map((view) => <FieldsTable key={view.name} view={view} />)}

      {links.length > 0 && (
        <section className={styles.section}>
          <h2>{legacyT.links}</h2>
          <Table caption={legacyT.links} aria-label={legacyT.links}>
            <TableHeader>
              <Column isRowHeader>Parent</Column>
              <Column>Child</Column>
              <Column>Child key</Column>
              <Column>Parent key</Column>
              <Column>Kind</Column>
            </TableHeader>
            <TableBody items={links}>
              {(link) => (
                <Row id={link.id}>
                  <Cell>{link.parent}</Cell>
                  <Cell>{link.child}</Cell>
                  <Cell>{link.child_key}</Cell>
                  <Cell>{link.parent_key}</Cell>
                  <Cell>{link.kind}</Cell>
                </Row>
              )}
            </TableBody>
          </Table>
        </section>
      )}

      {programs.length > 0 && (
        <section className={styles.section}>
          <h2>{legacyT.programs}</h2>
          <Table caption={legacyT.programs} aria-label={legacyT.programs}>
            <TableHeader>
              <Column isRowHeader>Name</Column>
              <Column>SHA-256</Column>
            </TableHeader>
            <TableBody items={programs}>
              {(program) => (
                <Row id={program.id}>
                  <Cell>{program.name}</Cell>
                  <Cell>{program.text_sha256.slice(0, 16)}</Cell>
                </Row>
              )}
            </TableBody>
          </Table>
        </section>
      )}

      {checkCode.length > 0 && (
        <section className={styles.section}>
          <h2>{legacyT.checkCode}</h2>
          <Table caption={legacyT.checkCode} aria-label={legacyT.checkCode}>
            <TableHeader>
              <Column isRowHeader>View</Column>
              <Column>Blocks</Column>
            </TableHeader>
            <TableBody items={checkCode}>
              {(entry) => (
                <Row id={entry.id}>
                  <Cell>{entry.view}</Cell>
                  <Cell>{String(entry.blocks)}</Cell>
                </Row>
              )}
            </TableBody>
          </Table>
        </section>
      )}

      <section className={styles.section}>
        <h2>{legacyT.report}</h2>
        <Table caption={legacyT.report} aria-label={legacyT.report}>
          <TableHeader>
            <Column isRowHeader>Kind</Column>
            <Column>Item</Column>
            <Column>Reason</Column>
          </TableHeader>
          <TableBody items={report}>
            {(entry) => (
              <Row id={entry.id}>
                <Cell>{entry.kind}</Cell>
                <Cell>{entry.item}</Cell>
                <Cell>{entry.reason}</Cell>
              </Row>
            )}
          </TableBody>
        </Table>
      </section>
    </div>
  );
}
