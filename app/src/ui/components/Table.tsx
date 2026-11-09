import { useEffect, useId, useState, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import {
  Cell as RACCell,
  Column as RACColumn,
  Row as RACRow,
  Table as RACTable,
  TableBody as RACTableBody,
  TableHeader as RACTableHeader,
  type CellProps,
  type ColumnProps,
  type RowProps,
  type TableBodyProps,
  type TableHeaderProps,
  type TableProps,
} from 'react-aria-components';
import { renderClass } from './classes';
import styles from './Table.module.css';

export interface EpioraTableProps extends TableProps {
  // Visible table caption. React Aria's collection does not render a
  // <caption> child, so it is mounted through a portal and moved to be
  // the table's first child; it also provides the accessible name.
  caption?: ReactNode;
}

export function Table({
  caption,
  className,
  children,
  ...props
}: EpioraTableProps) {
  const captionId = useId();
  const [tableEl, setTableEl] = useState<HTMLElement | null>(null);
  const [captionEl, setCaptionEl] = useState<HTMLElement | null>(null);

  useEffect(() => {
    if (tableEl && captionEl && tableEl.firstElementChild !== captionEl) {
      tableEl.insertBefore(captionEl, tableEl.firstChild);
    }
  }, [tableEl, captionEl]);

  const labelled =
    props['aria-label'] !== undefined || props['aria-labelledby'] !== undefined;
  const captionLabelProps =
    caption !== undefined && !labelled
      ? { 'aria-labelledby': captionId }
      : {};
  return (
    <>
      <RACTable
        ref={setTableEl}
        {...props}
        {...captionLabelProps}
        className={renderClass(styles.table, className)}
      >
        {children}
      </RACTable>
      {tableEl !== null &&
        caption !== undefined &&
        createPortal(
          <caption id={captionId} ref={setCaptionEl} className={styles.caption}>
            {caption}
          </caption>,
          tableEl,
        )}
    </>
  );
}

export function TableHeader<T extends object>({
  className,
  ...props
}: TableHeaderProps<T>) {
  return (
    <RACTableHeader
      {...props}
      className={renderClass(styles.header, className)}
    />
  );
}

export function Column({ className, ...props }: ColumnProps) {
  return (
    <RACColumn
      {...props}
      className={renderClass(styles.column, className)}
    />
  );
}

export function TableBody<T extends object>({
  className,
  ...props
}: TableBodyProps<T>) {
  return (
    <RACTableBody
      {...props}
      className={renderClass(styles.body, className)}
    />
  );
}

export function Row<T extends object>({
  className,
  ...props
}: RowProps<T>) {
  return (
    <RACRow<T>
      {...props}
      className={renderClass(styles.row, className)}
    />
  );
}

export function Cell({ className, ...props }: CellProps) {
  return (
    <RACCell
      {...props}
      className={renderClass(styles.cell, className)}
    />
  );
}

export { TableLayout, Virtualizer } from 'react-aria-components';
