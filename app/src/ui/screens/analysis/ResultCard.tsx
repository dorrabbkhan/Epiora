// One output-log entry (spec §8.2): title (procedure, variables,
// dataset vN), timestamp, the rendered result tables, a collapsible
// receipt, a "Rerun on vN" badge when a newer dataset version exists
// (rerunning adds a new card and keeps this one) and a recorded
// delete.
import { useId } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { FreqResult } from '@engine-types/FreqResult';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { TablesResult } from '@engine-types/TablesResult';
import { Button } from '../../components';
import { dataApi } from '../../../worker/client';
import type { RunRecord } from '../../../worker/api';
import { t } from '../../strings';
import { renderFreq, renderTables } from './render-result';
import { ReceiptPanel } from './ReceiptPanel';
import styles from './analysis.module.css';

/** The card title spec §8.2 fixes: procedure, variables, dataset vN. */
export function cardTitle(run: RunRecord, datasetName: string): string {
  if (run.procedure === 'freq') {
    const options = run.options as FreqOptions;
    return t.analysis.cardTitleFreq(
      options.variables.join(', '),
      datasetName,
      run.version,
    );
  }
  const options = run.options as TablesOptions;
  return t.analysis.cardTitleTables(
    options.exposure.name,
    options.outcome.name,
    datasetName,
    run.version,
  );
}

export function ResultCard({
  run,
  datasetName,
  onRunAdded,
}: {
  run: RunRecord;
  datasetName: string;
  onRunAdded?: ((run: RunRecord) => void) | undefined;
}) {
  const qc = useQueryClient();
  const titleId = useId();
  const title = cardTitle(run, datasetName);

  const invalidate = () =>
    qc.invalidateQueries({ queryKey: ['runs', run.projectId] });
  const rerun = useMutation({
    mutationFn: (version: number) => dataApi.rerun(run.id, version),
    onSuccess: (added) => {
      void invalidate();
      onRunAdded?.(added);
    },
  });
  const del = useMutation({
    mutationFn: () => dataApi.deleteRun(run.id),
    onSuccess: invalidate,
  });

  return (
    <article aria-labelledby={titleId} className={styles.card}>
      <h3 id={titleId}>{title}</h3>
      <p className={styles.timestamp}>{run.createdAt}</p>
      {run.newerVersion !== null && (
        <p className={styles.badge}>
          {t.analysis.newerVersion}
          <Button
            onPress={() => rerun.mutate(run.newerVersion ?? 0)}
            isDisabled={rerun.isPending}
          >
            {t.analysis.rerunOn(run.newerVersion)}
          </Button>
        </p>
      )}
      <div className={styles.result}>
        {run.procedure === 'freq'
          ? renderFreq(run.result as FreqResult)
          : renderTables(run.result as TablesResult)}
      </div>
      <ReceiptPanel receipt={run.receipt} />
      <div className={styles.actions}>
        <Button onPress={() => del.mutate()} isDisabled={del.isPending}>
          {t.analysis.deleteResult}
        </Button>
      </div>
    </article>
  );
}
