import { useState } from 'react';
import { Link } from '@tanstack/react-router';
import { useDataset, useVersions } from '../queries';
import { DataGrid } from '../screens/DataGrid';
import { EmptyState } from '../screens/EmptyState';
import { VersionList } from '../screens/VersionList';
import { t } from '../strings';
import styles from './routes.module.css';
import type { DatasetInfo } from '../../worker/api';
import { RpcError } from '../../worker/errors';

export function DatasetScreen({
  project,
  dataset,
}: {
  project: string;
  dataset: string;
}) {
  const info = useDataset(dataset);
  if (info.isPending) {
    return (
      <div className={styles.page}>
        <h1>{t.dataset.title}</h1>
        <p>{t.common.loading}</p>
      </div>
    );
  }
  if (info.isError) {
    return (
      <div className={styles.page}>
        <h1>{t.dataset.title}</h1>
        <EmptyState
          message={
            info.error instanceof RpcError
              ? info.error.userMessage
              : t.dataset.notFound
          }
        />
      </div>
    );
  }
  return <DatasetView project={project} info={info.data} />;
}

function DatasetView({ project, info }: { project: string; info: DatasetInfo }) {
  const versions = useVersions(info.id);
  const [chosen, setChosen] = useState<number | null>(null);
  const version = chosen ?? info.currentVersion;
  return (
    <div className={styles.page}>
      <h1>{info.name}</h1>
      <div className={styles.actions}>
        <Link to="/p/$project" params={{ project }}>
          {t.projects.title}
        </Link>
      </div>
      {(versions.data?.length ?? 0) > 0 && (
        <VersionList
          versions={versions.data ?? []}
          selected={version}
          onSelect={setChosen}
        />
      )}
      {version > 0 ? (
        <DataGrid
          key={version}
          datasetId={info.id}
          version={version}
          label={`${info.name} — v${version}`}
        />
      ) : (
        <EmptyState message={t.dataset.emptyGrid} />
      )}
    </div>
  );
}
