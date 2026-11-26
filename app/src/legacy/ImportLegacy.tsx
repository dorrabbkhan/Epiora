// `/legacy` (v1.1 spec §7.1): choose an Epi Info `.mdb`/`.accdb`/`.db`
// and its optional `.prj`, inspect it entirely in the data worker and
// move to the write-free preview. An unknown-layout Access file offers
// its ordinary tables for plain dataset import instead of failing.
import { useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Button, Checkbox, FileTrigger } from '../ui/components';
import { dataApi } from '../worker/client';
import { RpcError } from '../worker/errors';
import { inTauri } from '../worker/client';
import { rememberLegacyPreview } from './LegacyPreview';
import { SqlServerForm } from './SqlServerForm';
import { legacyT } from './strings';
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

export function ImportLegacy() {
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<InspectError | null>(null);
  const [plain, setPlain] = useState<PlainPreview | null>(null);
  const [plainFiles, setPlainFiles] = useState<File[] | null>(null);
  const [chosen, setChosen] = useState<string[]>([]);
  const [choosingTables, setChoosingTables] = useState(false);

  const inspect = async (files: FileList | null) => {
    if (files === null || files.length === 0) return;
    setBusy(true);
    setError(null);
    setPlain(null);
    setChoosingTables(false);
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

  const importPlain = async () => {
    if (plain === null || chosen.length === 0) return;
    setBusy(true);
    setError(null);
    try {
      const projectId = await dataApi.commitPlainLegacy(plain.previewId, chosen, () => {});
      await navigate({ to: '/p/$project', params: { project: projectId } });
    } catch (e) {
      setError({
        plain: false,
        message: e instanceof RpcError ? e.userMessage : e instanceof Error ? e.message : String(e),
      });
      setBusy(false);
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
              <Button onPress={() => setChoosingTables(true)}>{legacyT.chooseTables}</Button>
            </>
          )}
        </div>
      )}
      {choosingTables && plain !== null && (
        <section className={styles.section} aria-label={legacyT.plainTablesTitle}>
          <h2>{legacyT.plainTablesTitle}</h2>
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
