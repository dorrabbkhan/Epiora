// Wizard step 1: pick the target (new dataset, or re-import into an
// existing one) and the file. Re-import targets are offered only for
// datasets that already have a version.
import { useState } from 'react';
import { Button, FileTrigger, Label, Radio, RadioGroup } from '../../components';
import { t } from '../../strings';
import type { DatasetInfo } from '../../../worker/api';
import type { WizardState } from './wizard-state';
import styles from './import.module.css';

export function ChooseFile({
  state,
  datasets,
  onFile,
}: {
  state: WizardState;
  datasets: DatasetInfo[];
  onFile: (file: File, datasetId: string | null) => void;
}) {
  const [datasetId, setDatasetId] = useState<string | null>(null);
  const updatable = datasets.filter((d) => d.currentVersion > 0);
  return (
    <section className={styles.section} aria-label={t.import.choosePrompt}>
      <p>{t.import.choosePrompt}</p>
      {updatable.length > 0 && (
        <RadioGroup
          value={datasetId ?? 'new'}
          onChange={(v) => setDatasetId(v === 'new' ? null : v)}
        >
          <Label>{t.import.importInto}</Label>
          <Radio value="new">{t.import.newDataset}</Radio>
          {updatable.map((d) => (
            <Radio key={d.id} value={d.id}>
              {t.import.updateDataset(d.name, d.currentVersion)}
            </Radio>
          ))}
        </RadioGroup>
      )}
      <div className={styles.actions}>
        <FileTrigger
          onSelect={(files) => {
            const file = files?.item(0);
            if (file !== null && file !== undefined) onFile(file, datasetId);
          }}
        >
          <Button>{t.import.chooseFile}</Button>
        </FileTrigger>
      </div>
      {state.pending === 'inspect' && <p>{t.import.inspecting}</p>}
    </section>
  );
}
