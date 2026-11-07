import { Button } from '../components';
import { t } from '../strings';
import styles from './routes.module.css';

export function ProjectsScreen() {
  return (
    <div className={styles.page}>
      <h1>{t.projects.title}</h1>
      <p>{t.projects.empty}</p>
      <div className={styles.actions}>
        <Button>{t.projects.create}</Button>
      </div>
    </div>
  );
}
