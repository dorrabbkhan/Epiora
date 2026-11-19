// One output-log entry (spec §8.2): title (procedure, variables,
// dataset vN), timestamp, the rendered result tables, a collapsible
// receipt, a "Rerun on vN" badge when a newer dataset version exists
// (rerunning adds a new card and keeps this one) and a recorded
// delete.
import { useId, useState } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { FreqResult } from '@engine-types/FreqResult';
import type { MeansOptions } from '@engine-types/MeansOptions';
import type { MeansResult } from '@engine-types/MeansResult';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { TablesResult } from '@engine-types/TablesResult';
import { Button } from '../../components';
import { ExplainPanel } from '../../../explain/ExplainPanel';
import { dataApi } from '../../../worker/client';
import type { RunRecord } from '../../../worker/api';
import { t } from '../../strings';
import { CommandLine } from './CommandLine';
import {
  renderFreq,
  renderMeans,
  renderTables,
  type StatRow,
} from './render-result';
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
  if (run.procedure === 'means') {
    const options = run.options as MeansOptions;
    return t.analysis.cardTitleMeans(
      options.variable,
      options.group_by,
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
  // The row being explained — its stats are the concrete
  // `explainProcedure` paths the panel's picker offers.
  const [explaining, setExplaining] = useState<StatRow | null>(null);

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
  // "Export to R" (v1.0 Task 17, spec §8.2): the worker returns the
  // zip; turning bytes into a download is a main-thread job.
  const exportR = useMutation({
    mutationFn: () => dataApi.exportRun(run.id),
    onSuccess: ({ name, bytes }) => {
      const url = URL.createObjectURL(
        new Blob([bytes], { type: 'application/zip' }),
      );
      const a = document.createElement('a');
      a.href = url;
      a.download = name;
      a.click();
      URL.revokeObjectURL(url);
    },
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
          ? renderFreq(run.result as FreqResult, setExplaining)
          : run.procedure === 'means'
            ? renderMeans(run.result as MeansResult, run.notes, setExplaining)
            : renderTables(run.result as TablesResult, run.notes, setExplaining)}
      </div>
      {explaining !== null && explaining.stats !== undefined && (
        <ExplainPanel
          title={explaining.label}
          stats={explaining.stats}
          fetchTrace={(statistic) => dataApi.explainRun(run.id, statistic)}
          keyParts={['run', run.id]}
          notes={run.notes}
          onClose={() => setExplaining(null)}
        />
      )}
      <CommandLine command={run.command} />
      <ReceiptPanel receipt={run.receipt} />
      <div className={styles.actions}>
        <Button
          onPress={() => exportR.mutate()}
          isDisabled={exportR.isPending}
        >
          {t.analysis.exportR}
        </Button>
        <Button onPress={() => del.mutate()} isDisabled={del.isPending}>
          {t.analysis.deleteResult}
        </Button>
      </div>
    </article>
  );
}
