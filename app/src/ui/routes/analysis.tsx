// The analysis workspace (v0.3 plan Task 12, spec §8.2): left — the
// dataset/version picker and the procedure menu; right — the output
// log of result cards. Dialogs run the engine through the worker RPC;
// every appended card is announced and persisted via `runs`.
import { useState } from 'react';
import { Link } from '@tanstack/react-router';
import { useQueryClient } from '@tanstack/react-query';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { TablesOptions } from '@engine-types/TablesOptions';
import { useDatasets, useRuns, useVersions } from '../queries';
import { dataApi } from '../../worker/client';
import type { RunRecord } from '../../worker/api';
import { EmptyState } from '../screens/EmptyState';
import { DatasetPicker } from '../screens/analysis/DatasetPicker';
import { FreqDialog } from '../screens/analysis/FreqDialog';
import { OutputLog } from '../screens/analysis/OutputLog';
import { ProcedureMenu } from '../screens/analysis/ProcedureMenu';
import { cardTitle } from '../screens/analysis/ResultCard';
import { TablesDialog } from '../screens/analysis/TablesDialog';
import { t } from '../strings';
import styles from './routes.module.css';
import analysisStyles from '../screens/analysis/analysis.module.css';

export function AnalysisScreen({ project }: { project: string }) {
  const datasets = useDatasets(project);
  const runs = useRuns(project);
  const qc = useQueryClient();
  const [datasetId, setDatasetId] = useState<string | null>(null);
  const [version, setVersion] = useState<number | null>(null);
  const [dialog, setDialog] = useState<'freq' | 'tables' | null>(null);
  const [announce, setAnnounce] = useState<string | null>(null);

  const all = datasets.data ?? [];
  const analysable = all.filter((d) => d.currentVersion > 0);
  const dataset =
    analysable.find((d) => d.id === datasetId) ?? analysable[0];
  const versions = useVersions(dataset?.id ?? '');
  const versionNumber = version ?? dataset?.currentVersion ?? 0;
  const nameOf = new Map(all.map((d) => [d.id, d.name]));

  const onRan = (run: RunRecord) => {
    void qc.invalidateQueries({ queryKey: ['runs', project] });
    setAnnounce(
      t.analysis.announce(
        cardTitle(run, nameOf.get(run.datasetId) ?? run.datasetId),
      ),
    );
  };

  const makeRun =
    (procedure: 'freq' | 'tables') =>
    async (options: FreqOptions | TablesOptions) => {
      if (dataset === undefined) return;
      const run =
        procedure === 'freq'
          ? await dataApi.runFreq(
              project,
              dataset.id,
              versionNumber,
              options as FreqOptions,
            )
          : await dataApi.runTables(
              project,
              dataset.id,
              versionNumber,
              options as TablesOptions,
            );
      onRan(run);
    };

  return (
    <div className={styles.page}>
      <h1>{t.analysis.title}</h1>
      <div className={styles.actions}>
        <Link to="/p/$project" params={{ project }}>
          {t.analysis.backToProject}
        </Link>
      </div>
      {datasets.data !== undefined && analysable.length === 0 ? (
        <EmptyState message={t.analysis.emptyDatasets}>
          <Link to="/p/$project/import" params={{ project }}>
            {t.project.importData}
          </Link>
        </EmptyState>
      ) : (
        <div className={analysisStyles.workspace}>
          <section
            className={analysisStyles.controls}
            aria-label={t.analysis.controls}
          >
            <DatasetPicker
              datasets={analysable}
              versions={versions.data ?? []}
              datasetId={dataset?.id ?? null}
              version={versionNumber === 0 ? null : versionNumber}
              onDataset={(id) => {
                setDatasetId(id);
                setVersion(null);
              }}
              onVersion={setVersion}
              busy={false}
            />
            <ProcedureMenu
              disabled={dataset === undefined || versionNumber === 0}
              onChoose={setDialog}
            />
          </section>
          <OutputLog
            runs={runs.data}
            datasetName={(id) => nameOf.get(id) ?? id}
            announce={announce}
            onRunAdded={onRan}
          />
        </div>
      )}
      {dataset !== undefined && (
        <>
          <FreqDialog
            open={dialog === 'freq'}
            onOpenChange={(open) => {
              if (!open) setDialog(null);
            }}
            datasetId={dataset.id}
            version={versionNumber}
            run={makeRun('freq')}
          />
          <TablesDialog
            open={dialog === 'tables'}
            onOpenChange={(open) => {
              if (!open) setDialog(null);
            }}
            datasetId={dataset.id}
            version={versionNumber}
            run={makeRun('tables')}
          />
        </>
      )}
    </div>
  );
}
