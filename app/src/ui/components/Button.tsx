import {
  Button as RACButton,
  type ButtonProps,
} from 'react-aria-components';
import { renderClass } from './classes';
import styles from './Button.module.css';

export function Button({ className, ...props }: ButtonProps) {
  return (
    <RACButton
      {...props}
      className={renderClass(styles.button, className)}
    />
  );
}
