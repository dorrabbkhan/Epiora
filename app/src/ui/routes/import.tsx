import { Button, FileTrigger } from '../components';
import { t } from '../strings';
import styles from './routes.module.css';

export function ImportScreen() {
  return (
    <div className={styles.page}>
      <h1>{t.import.title}</h1>
      <p>{t.import.placeholder}</p>
      <div className={styles.actions}>
        <FileTrigger>
          <Button>{t.import.chooseFile}</Button>
        </FileTrigger>
      </div>
    </div>
  );
}
