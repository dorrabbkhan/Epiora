import type { ComponentProps } from 'react';
import {
  UNSTABLE_Toast as RACToast,
  UNSTABLE_ToastContent as RACToastContent,
  UNSTABLE_ToastList as RACToastList,
  UNSTABLE_ToastQueue as RACToastQueue,
  UNSTABLE_ToastRegion as RACToastRegion,
  type ToastListProps,
  type ToastProps,
  type ToastRegionProps,
} from 'react-aria-components';
import { cx, renderClass } from './classes';
import styles from './Toast.module.css';

export { RACToastQueue as ToastQueue };

export function ToastRegion<T>({
  className,
  ...props
}: ToastRegionProps<T>) {
  return (
    <RACToastRegion
      {...props}
      {...{ 'aria-live': 'polite' }}
      className={renderClass(
        styles.region,
        className,
      )}
    />
  );
}

export function ToastList<T>({ className, ...props }: ToastListProps<T>) {
  return (
    <RACToastList {...props} className={renderClass(styles.list, className)} />
  );
}

export function Toast<T>({ className, ...props }: ToastProps<T>) {
  return (
    <RACToast
      {...props}
      className={renderClass(styles.toast, className)}
    />
  );
}

export function ToastContent({
  className,
  ...props
}: ComponentProps<typeof RACToastContent>) {
  return (
    <RACToastContent {...props} className={cx(styles.content, className)} />
  );
}
