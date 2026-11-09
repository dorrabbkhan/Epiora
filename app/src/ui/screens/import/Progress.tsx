// Wizard step 6: one progress bar per commit stage
// (`ImportProgress.stage` on the wire) plus the Cancel that reaches
// the running commit between batches.
import { Button, Label, ProgressBar } from '../../components';
import type { ImportProgress } from '../../../worker/api';
import { t } from '../../strings';
import type { WizardState } from './wizard-state';
import styles from './import.module.css';

const STAGES: { id: ImportProgress['stage']; label: string }[] = [
  { id: 'parse', label: t.import.stageParse },
  { id: 'diff', label: t.import.stageDiff },
  { id: 'write', label: t.import.stageWrite },
  { id: 'snapshot', label: t.import.stageSnapshot },
];

export function Progress({
  state,
  onCancel,
}: {
  state: WizardState;
  onCancel: () => void;
}) {
  const progress = state.progress;
  const currentIndex =
    progress === null ? -1 : STAGES.findIndex((s) => s.id === progress.stage);
  return (
    <section className={styles.section} aria-label={t.import.progressTitle}>
      <h2>{t.import.progressTitle}</h2>
      <div className={styles.stages}>
        {STAGES.map((stage, index) => {
          const active = index === currentIndex;
          const done = index < currentIndex;
          const value = done
            ? 1
            : active && progress !== null && progress.total > 0
              ? progress.done / progress.total
              : 0;
          return (
            <div key={stage.id} className={styles.stage}>
              <ProgressBar
                aria-label={stage.label}
                value={value * 100}
                maxValue={100}
              >
                <Label>{stage.label}</Label>
              </ProgressBar>
              {active && progress !== null && progress.total > 1 && (
                <p>{t.import.progressCount(progress.done, progress.total)}</p>
              )}
            </div>
          );
        })}
      </div>
      <div className={styles.actions}>
        <Button onPress={onCancel}>{t.import.cancelImport}</Button>
      </div>
    </section>
  );
}
