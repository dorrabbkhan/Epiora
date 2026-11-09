// Re-import step (spec §6.4): added/changed/removed counts, the first
// 20 sample changes per field, and the schema changes — which must be
// accepted explicitly before the wizard can advance.
import type { Value } from '@import-types/Value';
import { Checkbox } from '../../components';
import { t } from '../../strings';
import { hasSchemaChanges, type WizardState } from './wizard-state';
import styles from './import.module.css';

function renderValue(v: Value | null): string {
  return v === null ? '—' : String(v.value);
}

export function DiffReview({
  state,
  onAccept,
}: {
  state: WizardState;
  onAccept: (accept: boolean) => void;
}) {
  const diff = state.diff;
  if (diff === null) return null;
  const schema = diff.schema;
  const schemaChanged = hasSchemaChanges(diff);
  const sampleFields = Object.entries(diff.samples);
  return (
    <section className={styles.section} aria-label={t.import.diffTitle}>
      <h2>{t.import.diffTitle}</h2>
      <div className={styles.counts}>
        <p>{t.import.addedCount(diff.added)}</p>
        <p>{t.import.changedCount(diff.changed)}</p>
        <p>{t.import.removedCount(diff.removed)}</p>
      </div>
      {diff.unchanged && <p>{t.import.diffUnchanged}</p>}
      {schemaChanged && (
        <>
          <h3>{t.import.schemaChangesTitle}</h3>
          {schema.added_columns.length > 0 && (
            <p>{t.import.addedColumns(schema.added_columns)}</p>
          )}
          {schema.removed_columns.length > 0 && (
            <p>{t.import.removedColumns(schema.removed_columns)}</p>
          )}
          {schema.type_changes.length > 0 && (
            <p>{t.import.typeChanges(schema.type_changes)}</p>
          )}
          <Checkbox isSelected={state.acceptSchema} onChange={onAccept}>
            {t.import.acceptSchema}
          </Checkbox>
        </>
      )}
      {sampleFields.length > 0 && (
        <>
          <h3>{t.import.sampleChangesTitle}</h3>
          {sampleFields.map(([field, changes]) => (
            <div key={field} className={styles.stage}>
              <p className={styles.warning}>{field}</p>
              <ul>
                {changes.map((c, i) => (
                  <li key={i}>
                    {c.record_id}: {renderValue(c.old)} → {renderValue(c.new)}
                  </li>
                ))}
              </ul>
            </div>
          ))}
        </>
      )}
    </section>
  );
}
