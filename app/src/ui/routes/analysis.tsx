import { t } from '../strings';
import styles from './routes.module.css';

export function AnalysisScreen({ project }: { project: string }) {
  return (
    <div className={styles.page}>
      <h1>{t.analysis.title}</h1>
      <p>
        {project} — {t.analysis.placeholder}
      </p>
    </div>
  );
}
