// Version selector for the dataset screen (spec §8.1): a Menu of the
// dataset's versions, newest first, each labelled with its row count;
// the single selection marks the version the grid is showing.
import {
  Button,
  Menu,
  MenuItem,
  MenuTrigger,
  Popover,
} from '../components';
import type { VersionInfo } from '../../worker/api';
import { t } from '../strings';

export function VersionList({
  versions,
  selected,
  onSelect,
}: {
  versions: VersionInfo[];
  selected: number;
  onSelect: (version: number) => void;
}) {
  const items = [...versions]
    .sort((a, b) => b.version - a.version)
    .map((v) => ({
      id: String(v.version),
      label: t.dataset.versionOption(v.version, v.rowCount),
    }));
  const selectedItem = items.find((i) => i.id === String(selected));
  return (
    <MenuTrigger>
      <Button>{selectedItem?.label ?? t.dataset.version}</Button>
      <Popover>
        <Menu
          items={items}
          selectionMode="single"
          selectedKeys={new Set([String(selected)])}
          onSelectionChange={(keys) => {
            const key = [...keys][0];
            if (key !== undefined) onSelect(Number(key));
          }}
        >
          {(item) => <MenuItem id={item.id}>{item.label}</MenuItem>}
        </Menu>
      </Popover>
    </MenuTrigger>
  );
}
