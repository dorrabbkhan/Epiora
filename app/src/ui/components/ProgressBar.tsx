import {
  ProgressBar as RACProgressBar,
  type ProgressBarProps,
  type ProgressBarRenderProps,
} from 'react-aria-components';
import { renderClass, type RenderChildren } from './classes';
import styles from './ProgressBar.module.css';

export interface EpioraProgressBarProps
  extends Omit<ProgressBarProps, 'children'> {
  children?: RenderChildren<ProgressBarRenderProps>;
}

export function ProgressBar({
  className,
  children,
  ...props
}: EpioraProgressBarProps) {
  return (
    <RACProgressBar
      {...props}
      className={renderClass(styles.bar, className)}
    >
      {(renderProps) => (
        <>
          {typeof children === 'function' ? children(renderProps) : children}
          <span className={styles.track}>
            <span
              className={styles.fill}
              style={{
                width: `${renderProps.isIndeterminate ? 100 : (renderProps.percentage ?? 0)}%`,
              }}
            />
          </span>
        </>
      )}
    </RACProgressBar>
  );
}
