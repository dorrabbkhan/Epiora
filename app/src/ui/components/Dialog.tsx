import {
  Dialog as RACDialog,
  DialogTrigger as RACDialogTrigger,
  Modal as RACModal,
  ModalOverlay as RACModalOverlay,
  type DialogProps,
  type DialogTriggerProps,
  type ModalOverlayProps,
} from 'react-aria-components';
import { cx, renderClass } from './classes';
import styles from './Dialog.module.css';

export function DialogTrigger(props: DialogTriggerProps) {
  return <RACDialogTrigger {...props} />;
}

export function ModalOverlay({ className, ...props }: ModalOverlayProps) {
  return (
    <RACModalOverlay
      {...props}
      className={renderClass(styles.overlay, className)}
    />
  );
}

export function Modal({ className, ...props }: ModalOverlayProps) {
  return (
    <RACModal
      {...props}
      className={renderClass(styles.modal, className)}
    />
  );
}

export function Dialog({ className, ...props }: DialogProps) {
  return <RACDialog {...props} className={cx(styles.dialog, className)} />;
}

export { Heading } from 'react-aria-components';
