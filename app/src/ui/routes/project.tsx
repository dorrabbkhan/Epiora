import { Link } from '@tanstack/react-router';
import {
  Cell,
  Column,
  Row,
  Table,
  TableBody,
  TableHeader,
} from '../components';
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
      </div>
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
      />
    </div>
  );
}
