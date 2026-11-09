// The output log (spec §8.2): the project's runs newest first as
// result cards; results are announced through the LiveAnnouncer.
import type { RunRecord } from '../../../worker/api';
import { EmptyState } from '../EmptyState';
import { t } from '../../strings';
import { LiveAnnouncer } from './LiveAnnouncer';
import { ResultCard } from './ResultCard';
import styles from './analysis.module.css';

export function OutputLog({
  runs,
  datasetName,
  announce,
  onRunAdded,
}: {
  runs: RunRecord[] | undefined;
  datasetName: (id: string) => string;
  announce: string | null;
  onRunAdded?: ((run: RunRecord) => void) | undefined;
}) {
  return (
    <section aria-label={t.analysis.outputLog} className={styles.log}>
      <h2>{t.analysis.outputLog}</h2>
      <LiveAnnouncer message={announce} />
      {runs === undefined ? (
        <p>{t.common.loading}</p>
      ) : runs.length === 0 ? (
        <EmptyState message={t.analysis.emptyLog} />
      ) : (
        <div>
          {runs.map((run) => (
            <ResultCard
              key={run.id}
              run={run}
              datasetName={datasetName(run.datasetId)}
              onRunAdded={onRunAdded}
            />
          ))}
        </div>
      )}
    </section>
  );
}
