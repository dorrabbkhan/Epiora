// The procedure menu (spec §8.2): FREQ, TABLES and MEANS, each opening
// its options dialog.
import {
  Button,
  Menu,
  MenuItem,
  MenuTrigger,
  Popover,
} from '../../components';
import { t } from '../../strings';

export function ProcedureMenu({
  onChoose,
  disabled,
}: {
  onChoose: (procedure: 'freq' | 'tables' | 'means') => void;
  disabled: boolean;
}) {
  return (
    <MenuTrigger>
      <Button isDisabled={disabled}>{t.analysis.procedureMenu}</Button>
      <Popover>
        <Menu
          onAction={(key) =>
            onChoose(
              key === 'freq' ? 'freq' : key === 'means' ? 'means' : 'tables',
            )
          }
        >
          <MenuItem id="freq">{t.analysis.freq}</MenuItem>
          <MenuItem id="tables">{t.analysis.tables}</MenuItem>
          <MenuItem id="means">{t.analysis.means}</MenuItem>
        </Menu>
      </Popover>
    </MenuTrigger>
  );
}
