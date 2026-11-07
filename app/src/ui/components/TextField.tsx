import {
  FieldError as RACFieldError,
  Input as RACInput,
  Label as RACLabel,
  TextArea as RACTextArea,
  TextField as RACTextField,
  type FieldErrorProps,
  type InputProps,
  type LabelProps,
  type TextAreaProps,
  type TextFieldProps,
} from 'react-aria-components';
import { cx, renderClass } from './classes';
import styles from './TextField.module.css';

export function TextField({ className, ...props }: TextFieldProps) {
  return (
    <RACTextField
      {...props}
      className={renderClass(styles.field, className)}
    />
  );
}

export function Label({ className, ...props }: LabelProps) {
  return <RACLabel {...props} className={cx(styles.label, className)} />;
}

export function Input({ className, ...props }: InputProps) {
  return (
    <RACInput
      {...props}
      className={renderClass(styles.input, className)}
    />
  );
}

export function TextArea({ className, ...props }: TextAreaProps) {
  return (
    <RACTextArea
      {...props}
      className={renderClass(styles.input, className)}
    />
  );
}

export function FieldError({ className, ...props }: FieldErrorProps) {
  return (
    <RACFieldError
      {...props}
      className={renderClass(styles.error, className)}
    />
  );
}
