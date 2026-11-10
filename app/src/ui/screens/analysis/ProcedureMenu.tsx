// The procedure menu (spec §8.2): FREQ and TABLES, each opening its
// options dialog.
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
  onChoose: (procedure: 'freq' | 'tables') => void;
  disabled: boolean;
}) {
  return (
    <MenuTrigger>
      <Button isDisabled={disabled}>{t.analysis.procedureMenu}</Button>
      <Popover>
        <Menu onAction={(key) => onChoose(key === 'freq' ? 'freq' : 'tables')}>
          <MenuItem id="freq">{t.analysis.freq}</MenuItem>
          <MenuItem id="tables">{t.analysis.tables}</MenuItem>
        </Menu>
      </Popover>
    </MenuTrigger>
  );
}
