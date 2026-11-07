import { t } from '../strings';
import styles from './routes.module.css';

export function DatasetScreen({
  project,
  dataset,
}: {
  project: string;
  dataset: string;
}) {
  return (
    <div className={styles.page}>
      <h1>
        {t.dataset.title} {dataset}
      </h1>
      <p>
        {project} — {t.dataset.placeholder}
      </p>
    </div>
  );
}
