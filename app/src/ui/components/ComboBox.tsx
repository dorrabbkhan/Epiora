import {
  ComboBox as RACComboBox,
  type ComboBoxProps,
} from 'react-aria-components';
import { renderClass } from './classes';
import styles from './ComboBox.module.css';

export function ComboBox<T extends object>({
  className,
  ...props
}: ComboBoxProps<T>) {
  return (
    <RACComboBox
      {...props}
      className={renderClass(styles.combobox, className)}
    />
  );
}
