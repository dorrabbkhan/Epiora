// Wizard step 5: the final summary before the commit starts.
import { Button } from '../../components';
import { t } from '../../strings';
import type { WizardState } from './wizard-state';
import styles from './import.module.css';

export function Confirm({
  state,
  datasetName,
  busy,
  onImport,
}: {
  state: WizardState;
  /** The target dataset's name on re-import; unused for new imports. */
  datasetName: string | null;
  busy: boolean;
  onImport: () => void;
}) {
  const rows = state.inspection?.row_count ?? 0;
  const summary =
    state.target.kind === 'reimport'
      ? t.import.confirmReimport(datasetName ?? state.target.datasetId, rows)
      : t.import.confirmNew(state.fileName ?? '', rows);
  return (
    <section className={styles.section} aria-label={t.import.confirmTitle}>
      <h2>{t.import.confirmTitle}</h2>
      <p>{summary}</p>
      <div className={styles.actions}>
        <Button onPress={onImport} isDisabled={busy}>
          {t.import.importButton}
        </Button>
      </div>
    </section>
  );
}
