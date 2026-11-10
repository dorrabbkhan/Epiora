// Dataset and version picker for the analysis workspace (spec §8.2
// left pane).
import {
  Button,
  Label,
  ListBox,
  ListBoxItem,
  Popover,
  Select,
  SelectValue,
} from '../../components';
import type { DatasetInfo, VersionInfo } from '../../../worker/api';
import { t } from '../../strings';
import styles from './analysis.module.css';

export function DatasetPicker({
  datasets,
  versions,
  datasetId,
  version,
  onDataset,
  onVersion,
  busy,
}: {
  datasets: DatasetInfo[];
  versions: VersionInfo[];
  datasetId: string | null;
  version: number | null;
  onDataset: (id: string) => void;
  onVersion: (version: number) => void;
  busy: boolean;
}) {
  const datasetItems = datasets.map((d) => ({ id: d.id, label: d.name }));
  const versionItems = [...versions]
    .sort((a, b) => b.version - a.version)
    .map((v) => ({
      id: String(v.version),
      label: t.analysis.versionOption(v.version, v.rowCount),
    }));
  return (
    <div className={styles.picker}>
      <Select
        selectedKey={datasetId}
        onSelectionChange={(key) => onDataset(String(key))}
        aria-label={t.analysis.datasetLabel}
        isDisabled={busy || datasetItems.length === 0}
      >
        <Label>{t.analysis.datasetLabel}</Label>
        <Button>
          <SelectValue />
        </Button>
        <Popover>
          <ListBox items={datasetItems}>
            {(item) => <ListBoxItem id={item.id}>{item.label}</ListBoxItem>}
          </ListBox>
        </Popover>
      </Select>
      <Select
        selectedKey={version === null ? null : String(version)}
        onSelectionChange={(key) => onVersion(Number(key))}
        aria-label={t.analysis.versionLabel}
        isDisabled={busy || versionItems.length === 0}
      >
        <Label>{t.analysis.versionLabel}</Label>
        <Button>
          <SelectValue />
        </Button>
        <Popover>
          <ListBox items={versionItems}>
            {(item) => <ListBoxItem id={item.id}>{item.label}</ListBoxItem>}
          </ListBox>
        </Popover>
      </Select>
    </div>
  );
}
