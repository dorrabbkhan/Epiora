// The one-tab lock screen (spec §7.1): shown when this tab's worker
// could not take the store lock (another tab holds it) or lost it to
// a take-over. "Use here" steals the lock back and reloads — the
// reload is the user's explicit handover, never silent.
import { Button, Heading } from './components';
import { cx } from './components/classes';
import { takeOver } from '../worker/tab-lock';
import { t } from './strings';
import styles from './LockScreen.module.css';

export function LockScreen({
  reason,
}: {
  reason: 'held-elsewhere' | 'lost';
}) {
  return (
    <main className={styles.screen}>
      <Heading level={1} className={cx(styles.title)}>
        {t.lock.title}
      </Heading>
      <p className={styles.body}>
        {reason === 'lost' ? t.lock.movedBody : t.lock.openBody}
      </p>
      <Button
        onPress={() => {
          void takeOver().then(() => window.location.reload());
        }}
      >
        {t.lock.useHere}
      </Button>
    </main>
  );
}
