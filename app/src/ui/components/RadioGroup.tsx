import {
  Radio as RACRadio,
  RadioGroup as RACRadioGroup,
  type RadioGroupProps,
  type RadioProps,
  type RadioRenderProps,
} from 'react-aria-components';
import { renderClass, type RenderChildren } from './classes';
import styles from './RadioGroup.module.css';

export function RadioGroup({ className, ...props }: RadioGroupProps) {
  return (
    <RACRadioGroup
      {...props}
      className={renderClass(styles.group, className)}
    />
  );
}

export interface EpioraRadioProps extends Omit<RadioProps, 'children'> {
  children?: RenderChildren<RadioRenderProps>;
}

export function Radio({ className, children, ...props }: EpioraRadioProps) {
  return (
    <RACRadio
      {...props}
      className={renderClass(styles.radio, className)}
    >
      {(renderProps) => (
        <>
          <svg viewBox="0 0 18 18" aria-hidden="true">
            <circle cx="9" cy="9" r="4" />
          </svg>
          {typeof children === 'function' ? children(renderProps) : children}
        </>
      )}
    </RACRadio>
  );
}
