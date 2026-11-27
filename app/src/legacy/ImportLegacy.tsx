// `/legacy` (v1.1 spec §7.1): choose an Epi Info `.mdb`/`.accdb`/`.db`
// and its optional `.prj`, inspect it entirely in the data worker and
// move to the write-free preview. An unknown-layout Access file offers
// its ordinary tables for plain dataset import instead of failing.
import { useEffect, useRef, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import {
  Button,
  Cell,
  Checkbox,
  Column,
  FileTrigger,
  ProgressBar,
  Row,
  Table,
  TableBody,
  TableHeader,
} from '../ui/components';
import { dataApi } from '../worker/client';
import { RpcError } from '../worker/errors';
import { inTauri } from '../worker/client';
import { rememberLegacyPreview } from './LegacyPreview';
import { SqlServerForm } from './SqlServerForm';
import { legacyT } from './strings';
import type { ImportProgress } from '../worker/api';
import type { PlainPreview } from '../worker/legacy';
import styles from './legacy.module.css';

interface InspectError {
  message: string;
  plain: boolean;
}

function describeError(e: unknown): InspectError {
  const text = e instanceof Error ? e.message : String(e);
  const plain =
    (e instanceof RpcError && e.detail === 'legacy_layout_unknown') ||
    text.includes('unrecognised project layout');
  return {
    plain,
    message: plain
      ? legacyT.plainOfferTitle
      : e instanceof RpcError
        ? e.userMessage
        : e instanceof Error
          ? e.message
          : String(e),
  };
}

function releasePlain(preview: PlainPreview | null): void {
  if (preview !== null) {
    void dataApi.discardPlainPreview(preview.previewId).catch(() => {});
  }
}

export function ImportLegacy() {
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<InspectError | null>(null);
  const [plain, setPlain] = useState<PlainPreview | null>(null);
  const [plainFiles, setPlainFiles] = useState<File[] | null>(null);
  const [chosen, setChosen] = useState<string[]>([]);
  const [choosingTables, setChoosingTables] = useState(false);
  const [progress, setProgress] = useState<ImportProgress | null>(null);
  const plainRef = useRef<PlainPreview | null>(null);
  const mountedRef = useRef(false);
  const importActiveRef = useRef(false);

  const clearPlain = (discard: boolean) => {
    const current = plainRef.current;
    plainRef.current = null;
    setPlain(null);
    setPlainFiles(null);
    setChosen([]);
    setChoosingTables(false);
    if (discard) releasePlain(current);
  };

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
      const current = plainRef.current;
      plainRef.current = null;
      if (current === null) return;
      if (importActiveRef.current) {
        void dataApi.cancelImport(current.previewId).catch(() => {});
      } else {
        releasePlain(current);
      }
    };
  }, []);

  const inspect = async (files: FileList | null) => {
    if (files === null || files.length === 0) return;
    clearPlain(true);
    setBusy(true);
    setError(null);
    try {
      const preview = await dataApi.inspectLegacy(Array.from(files));
      rememberLegacyPreview(preview);
      await navigate({ to: '/legacy/preview/$previewId', params: { previewId: preview.previewId } });
    } catch (e) {
      const described = describeError(e);
      setError(described);
      if (described.plain) {
        try {
          const offer = await dataApi.inspectPlainLegacy(Array.from(files));
          if (!mountedRef.current) {
            releasePlain(offer);
            return;
          }
          plainRef.current = offer;
          setPlain(offer);
          setPlainFiles(Array.from(files));
          setChosen(offer.tables);
        } catch {
          setPlain(null);
          setPlainFiles(null);
        }
      }
    } finally {
      setBusy(false);
    }
  };

  const closePlain = () => {
    clearPlain(true);
    setError(null);
    setProgress(null);
  };

  const importPlain = async () => {
    if (plain === null || chosen.length === 0) return;
    setBusy(true);
    setError(null);
    importActiveRef.current = true;
    try {
      await dataApi.beginLegacyImport(plain.previewId);
      setProgress({ stage: 'write', done: 0, total: chosen.length });
      await new Promise((resolve) => requestAnimationFrame(resolve));
      const projectId = await dataApi.commitPlainLegacy(plain.previewId, chosen, setProgress);
      plainRef.current = null;
      await navigate({ to: '/p/$project', params: { project: projectId } });
    } catch (e) {
      setError({
        plain: false,
        message:
          e instanceof RpcError && e.detail === 'cancelled'
            ? legacyT.cancelled
            : e instanceof RpcError
              ? e.userMessage
              : e instanceof Error
                ? e.message
                : String(e),
      });
    } finally {
      importActiveRef.current = false;
      setBusy(false);
      setProgress(null);
    }
  };

  return (
    <div>
      <h1>{legacyT.title}</h1>
      {!inTauri && <p className={styles.notice}>{legacyT.browserNotice}</p>}
      {error !== null && (
        <div role="alert">
          <p>{error.message}</p>
          {error.plain && plain !== null && plainFiles !== null && (
            <>
              <p>{legacyT.plainOffer}</p>
              <div className={styles.panel}>
                <Button onPress={() => setChoosingTables(true)}>{legacyT.chooseTables}</Button>
                <Button onPress={closePlain}>{legacyT.close}</Button>
              </div>
            </>
          )}
        </div>
      )}
      {choosingTables && plain !== null && (
        <section className={styles.section} aria-label={legacyT.plainTablesTitle}>
          <h2>{legacyT.plainTablesTitle}</h2>
          {plain.skippedColumns.length > 0 && (
            <section className={styles.section} aria-label={legacyT.plainWarnings}>
              <h3>{legacyT.plainWarnings}</h3>
              <p>{legacyT.plainBinaryWarning}</p>
              <Table caption={legacyT.plainWarnings} aria-label={legacyT.plainWarnings}>
                <TableHeader>
                  <Column isRowHeader>{legacyT.headers.table}</Column>
                  <Column>{legacyT.headers.column}</Column>
                  <Column>{legacyT.headers.reason}</Column>
                </TableHeader>
                <TableBody
                  items={plain.skippedColumns.map((warning, i) => ({
                    ...warning,
                    id: `plain-skipped-${i}`,
                  }))}
                >
                  {(warning) => (
                    <Row id={warning.id}>
                      <Cell>{warning.table}</Cell>
                      <Cell>{warning.column}</Cell>
                      <Cell>{warning.reason}</Cell>
                    </Row>
                  )}
                </TableBody>
              </Table>
            </section>
          )}
          {plain.tables.map((table) => (
            <Checkbox
              key={table}
              isSelected={chosen.includes(table)}
              onChange={(selected) =>
                setChosen((current) =>
                  selected ? [...current, table] : current.filter((t) => t !== table),
                )
              }
            >
              {table}
            </Checkbox>
          ))}
          {progress !== null && (
            <div className={styles.progress}>
              <ProgressBar
                aria-label={legacyT.writeProgress}
                value={progress.done}
                maxValue={Math.max(progress.total, 1)}
              />
              <Button onPress={() => void dataApi.cancelImport(plain.previewId)}>
                {legacyT.cancel}
              </Button>
            </div>
          )}
          <div className={styles.panel}>
            <Button onPress={() => void importPlain()} isDisabled={busy || chosen.length === 0}>
              {busy ? legacyT.importingTables : legacyT.importTables}
            </Button>
          </div>
        </section>
      )}
      <FileTrigger allowsMultiple onSelect={(files) => void inspect(files)}>
        <Button isDisabled={busy}>
          {busy ? legacyT.reading : legacyT.chooseFiles}
        </Button>
      </FileTrigger>
      {inTauri && <SqlServerForm />}
    </div>
  );
}
