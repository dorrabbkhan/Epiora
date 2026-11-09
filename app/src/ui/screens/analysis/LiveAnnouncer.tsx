// Live region for result announcements (spec §8.2, §9.3): appended
// results are announced politely; the region stays mounted so the
// announcement is not missed.
import styles from './analysis.module.css';

export function LiveAnnouncer({ message }: { message: string | null }) {
  return (
    <div role="status" aria-live="polite" className={styles.announce}>
      {message}
    </div>
  );
}
