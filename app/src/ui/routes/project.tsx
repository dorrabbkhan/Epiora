import { Link } from '@tanstack/react-router';
import { t } from '../strings';
import styles from './routes.module.css';

export function ProjectScreen({ project }: { project: string }) {
  return (
    <div className={styles.page}>
      <h1>{project}</h1>
      <div className={styles.actions}>
        <Link to="/p/$project/import" params={{ project }}>
          {t.project.importData}
        </Link>
        <Link to="/p/$project/analysis" params={{ project }}>
          {t.project.analysis}
        </Link>
      </div>
      <section>
        <h2>{t.project.datasets}</h2>
        <p>{t.project.emptyDatasets}</p>
      </section>
      <section>
        <h2>{t.project.outputLog}</h2>
        <p>{t.project.emptyLog}</p>
      </section>
    </div>
  );
}
