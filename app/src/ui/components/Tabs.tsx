import {
  Tab as RACTab,
  TabList as RACTabList,
  TabPanel as RACTabPanel,
  Tabs as RACTabs,
  type TabListProps,
  type TabPanelProps,
  type TabProps,
  type TabsProps,
} from 'react-aria-components';
import { renderClass } from './classes';
import styles from './Tabs.module.css';

export function Tabs({ className, ...props }: TabsProps) {
  return (
    <RACTabs
      {...props}
      className={renderClass(styles.tabs, className)}
    />
  );
}

export function TabList<T extends object>({
  className,
  ...props
}: TabListProps<T>) {
  return (
    <RACTabList
      {...props}
      className={renderClass(styles.list, className)}
    />
  );
}

export function Tab({ className, ...props }: TabProps) {
  return (
    <RACTab
      {...props}
      className={renderClass(styles.tab, className)}
    />
  );
}

export function TabPanel({ className, ...props }: TabPanelProps) {
  return (
    <RACTabPanel
      {...props}
      className={renderClass(styles.panel, className)}
    />
  );
}
