import {
  Checkbox as RACCheckbox,
  type CheckboxProps,
  type CheckboxRenderProps,
} from 'react-aria-components';
import { renderClass, type RenderChildren } from './classes';
import styles from './Checkbox.module.css';

export interface EpioraCheckboxProps
  extends Omit<CheckboxProps, 'children'> {
  children?: RenderChildren<CheckboxRenderProps>;
}

export function Checkbox({
  className,
  children,
  ...props
}: EpioraCheckboxProps) {
  return (
    <RACCheckbox
      {...props}
      className={renderClass(styles.checkbox, className)}
    >
      {(renderProps) => (
        <>
          <svg viewBox="0 0 18 18" aria-hidden="true">
            <polyline points="1,9 7,14 15,4" />
          </svg>
          {typeof children === 'function' ? children(renderProps) : children}
        </>
      )}
    </RACCheckbox>
  );
}
