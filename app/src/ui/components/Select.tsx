import {
  ListBox as RACListBox,
  ListBoxItem as RACListBoxItem,
  Popover as RACPopover,
  Select as RACSelect,
  SelectValue as RACSelectValue,
  type ListBoxItemProps,
  type ListBoxProps,
  type PopoverProps,
  type SelectProps,
  type SelectValueProps,
} from 'react-aria-components';
import { renderClass } from './classes';
import styles from './Select.module.css';

export function Select<T extends object>({
  className,
  ...props
}: SelectProps<T>) {
  return (
    <RACSelect
      {...props}
      className={renderClass(styles.select, className)}
    />
  );
}

export function SelectValue<T extends object>({
  className,
  ...props
}: SelectValueProps<T>) {
  return (
    <RACSelectValue<T>
      {...props}
      className={renderClass(
        styles.value,
        className,
      )}
    />
  );
}

export function Popover({ className, ...props }: PopoverProps) {
  return (
    <RACPopover
      {...props}
      className={renderClass(styles.popover, className)}
    />
  );
}

export function ListBox<T extends object>({
  className,
  ...props
}: ListBoxProps<T>) {
  return (
    <RACListBox
      {...props}
      className={renderClass(styles.listbox, className)}
    />
  );
}

export function ListBoxItem<T extends object>({
  className,
  ...props
}: ListBoxItemProps<T>) {
  return (
    <RACListBoxItem<T>
      {...props}
      className={renderClass(styles.item, className)}
    />
  );
}
