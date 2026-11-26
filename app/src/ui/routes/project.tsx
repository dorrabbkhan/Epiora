import { Link } from '@tanstack/react-router';
import {
  Button,
  Cell,
  Column,
  Row,
  Table,
  TableBody,
  TableHeader,
} from '../components';
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import { dataApi } from '../../worker/client';
import { useDatasets, useProjects, useRuns } from '../queries';
import { EmptyState } from '../screens/EmptyState';
import { OutputLog } from '../screens/analysis/OutputLog';
import { t } from '../strings';
import styles from './routes.module.css';

export function ProjectScreen({ project }: { project: string }) {
  const projects = useProjects();
  const info = projects.data?.find((p) => p.id === project);

  if (projects.data === undefined) {
    return (
      <div className={styles.page}>
        <h1>{t.projects.title}</h1>
        <p>{t.common.loading}</p>
      </div>
    );
  }
  if (info === undefined) {
    return (
      <div className={styles.page}>
        <h1>{t.projects.title}</h1>
        <EmptyState message={t.project.notFound} />
      </div>
    );
  }
  return <ProjectView projectId={info.id} name={info.name} />;
}

function ProjectView({ projectId, name }: { projectId: string; name: string }) {
  const datasets = useDatasets(projectId);
  const legacy = useQuery({
    queryKey: ['legacyProject', projectId],
    queryFn: () => dataApi.getLegacyProject(projectId),
  });
  const [sessionBanner] = useState(
    () => sessionStorage.getItem(`epiora-reimported:${projectId}`) === '1',
  );
  const [moveHelp, setMoveHelp] = useState(false);
  const runs = useRuns(projectId);
  const list = datasets.data;
  const runList = runs.data;
  const nameOf = new Map((list ?? []).map((d) => [d.id, d.name]));
  return (
    <div className={styles.page}>
      <h1>{name}</h1>
      <div className={styles.actions}>
        <Link to="/p/$project/import" params={{ project: projectId }}>
          {t.project.importData}
        </Link>
        <Link to="/p/$project/analysis" params={{ project: projectId }}>
          {t.project.analysis}
        </Link>
        {legacy.data !== null && legacy.data !== undefined && legacy.data.sourceFormat !== 'ei7_sql_server' && (
          <Link to="/p/$project/reimport" params={{ project: projectId }}>
            {t.project.reimport}
          </Link>
        )}
      </div>
      {sessionBanner && (
        <section aria-label="Working views">
          <p>Open working views stayed on their source versions.</p>
          {moveHelp ? (
            <p>Open the analysis workspace to move a working view by replaying its recipe.</p>
          ) : (
            <Button
              onPress={() => {
                sessionStorage.removeItem(`epiora-reimported:${projectId}`);
                setMoveHelp(true);
              }}
            >
              Move working views to new versions
            </Button>
          )}
        </section>
      )}
      <section>
        <h2>{t.project.datasets}</h2>
        {list === undefined ? (
          <p>{t.common.loading}</p>
        ) : list.length === 0 ? (
          <EmptyState message={t.project.emptyDatasets} />
        ) : (
          <Table caption={t.project.datasets} aria-label={t.project.datasets}>
            <TableHeader>
              <Column isRowHeader>{t.projects.nameColumn}</Column>
              <Column>{t.project.versionColumn}</Column>
              <Column>{t.project.createdColumn}</Column>
            </TableHeader>
            <TableBody items={list}>
              {(d) => (
                <Row id={d.id}>
                  <Cell>
                    <Link
                      to="/p/$project/d/$dataset"
                      params={{ project: projectId, dataset: d.id }}
                    >
                      {d.name}
                    </Link>
                  </Cell>
                  <Cell>
                    {d.currentVersion === 0
                      ? t.project.noVersion
                      : `v${d.currentVersion}`}
                  </Cell>
                  <Cell>{d.createdAt}</Cell>
                </Row>
              )}
            </TableBody>
          </Table>
        )}
      </section>
      <OutputLog
        runs={runList}
        datasetName={(id) => nameOf.get(id) ?? id}
        announce={null}
        emptyMessage={t.project.emptyLog}
        emptyAction={
          <Link to="/p/$project/analysis" params={{ project: projectId }}>
            {t.project.emptyLogAction}
          </Link>
        }
      />
    </div>
  );
}
