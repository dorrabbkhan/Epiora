import {
  Menu as RACMenu,
  MenuItem as RACMenuItem,
  MenuTrigger as RACMenuTrigger,
  type MenuItemProps,
  type MenuProps,
  type MenuTriggerProps,
} from 'react-aria-components';
import { renderClass } from './classes';
import styles from './Menu.module.css';

export function MenuTrigger(props: MenuTriggerProps) {
  return <RACMenuTrigger {...props} />;
}

export function Menu<T extends object>({
  className,
  ...props
}: MenuProps<T>) {
  return (
    <RACMenu
      {...props}
      className={renderClass(styles.menu, className)}
    />
  );
}

export function MenuItem<T extends object>({
  className,
  ...props
}: MenuItemProps<T>) {
  return (
    <RACMenuItem<T>
      {...props}
      className={renderClass(styles.item, className)}
    />
  );
}
