import {
  Disclosure as RACDisclosure,
  DisclosurePanel as RACDisclosurePanel,
  type DisclosurePanelProps,
  type DisclosureProps,
} from 'react-aria-components';
import { renderClass } from './classes';
import styles from './Disclosure.module.css';

export function Disclosure({ className, ...props }: DisclosureProps) {
  return (
    <RACDisclosure
      {...props}
      className={renderClass(
        styles.disclosure,
        className,
      )}
    />
  );
}

export function DisclosurePanel({
  className,
  ...props
}: DisclosurePanelProps) {
  return (
    <RACDisclosurePanel
      {...props}
      className={renderClass(
        styles.panel,
        className,
      )}
    />
  );
}
