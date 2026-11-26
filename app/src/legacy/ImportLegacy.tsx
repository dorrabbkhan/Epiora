// `/legacy` (v1.1 spec §7.1): choose an Epi Info `.mdb`/`.accdb`/`.db`
// and its optional `.prj`, inspect it entirely in the data worker and
// move to the write-free preview. The browser build carries the
// unencrypted-storage warning; the desktop build additionally offers
// the SQL Server source form.
import { useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { Button, FileTrigger } from '../ui/components';
import { dataApi } from '../worker/client';
import { RpcError } from '../worker/errors';
import { inTauri } from '../worker/client';
import { rememberLegacyPreview } from './LegacyPreview';
import { SqlServerForm } from './SqlServerForm';
import { legacyT } from './strings';
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

  const inspect = async (files: FileList | null) => {
    if (files === null || files.length === 0) return;
    setBusy(true);
    setError(null);
    try {
      const preview = await dataApi.inspectLegacy(Array.from(files));
      rememberLegacyPreview(preview);
      await navigate({ to: '/legacy/preview/$previewId', params: { previewId: preview.previewId } });
    } catch (e) {
      setError(describeError(e));
    } finally {
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
          {error.plain && (
            <>
              <p>{legacyT.plainOffer}</p>
              <Button onPress={() => setError(null)}>{legacyT.chooseTables}</Button>
            </>
          )}
        </div>
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
