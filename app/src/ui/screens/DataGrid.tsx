// The read-only data grid (spec §8.1, plan Task 10): a React Aria
// `Table` inside its `Virtualizer`, paged in 100-row chunks via
// `useVersionPages` — the scroll handler keeps the visible range plus
// one page of lookahead loaded. `aria-rowcount` is the version's full
// row count. Column headers show name, type and missing count.
import { useLayoutEffect, useMemo, useRef, useState, type UIEvent } from 'react';
import { TableLayout, Virtualizer } from 'react-aria-components';
import {
  Cell,
  Column,
  Row,
  Table,
  TableBody,
  TableHeader,
} from '../components';
import { useVersionPages } from '../queries';
import { t } from '../strings';
import styles from './DataGrid.module.css';

const PAGE = 100;
// Keep these in sync with --row-height / --heading-height in
// DataGrid.module.css — the layout computes row positions from them.
const ROW_HEIGHT = 48;
const HEADING_HEIGHT = 64;

export function DataGrid({
  datasetId,
  version,
  label,
}: {
  datasetId: string;
  version: number;
  label: string;
}) {
  const [offsets, setOffsets] = useState<number[]>([0]);
  const pages = useVersionPages(datasetId, version, offsets, PAGE);
  const first = pages[0]?.data;
  const total = first?.total ?? 0;
  const columns = useMemo(
    () => (first?.columns ?? []).map((c) => ({ ...c, id: c.name })),
    [first],
  );

  // Loaded rows by absolute index; unloaded rows render empty cells.
  const loaded = useMemo(() => {
    const map = new Map<number, (string | null)[]>();
    pages.forEach((result, i) => {
      const offset = offsets[i];
      if (offset === undefined) return;
      result.data?.rows.forEach((row, j) => map.set(offset + j, row));
    });
    return map;
  }, [pages, offsets]);

  const items = useMemo(
    () =>
      Array.from({ length: total }, (_, i) => ({
        id: i,
        cells: loaded.get(i) ?? null,
      })),
    [total, loaded],
  );

  const error = pages.find((p) => p.error !== null)?.error;

  const onScroll = (e: UIEvent<HTMLElement>) => {
    const el = e.currentTarget;
    const firstRow = Math.max(
      0,
      Math.floor((el.scrollTop - HEADING_HEIGHT) / ROW_HEIGHT),
    );
    const lastRow = Math.min(
      total,
      Math.ceil((el.scrollTop + el.clientHeight - HEADING_HEIGHT) / ROW_HEIGHT),
    );
    const needed: number[] = [];
    for (
      let o = Math.floor(firstRow / PAGE) * PAGE - PAGE;
      o <= Math.floor(lastRow / PAGE) * PAGE;
      o += PAGE
    ) {
      if (o >= 0 && o < total) needed.push(o);
    }
    setOffsets((prev) =>
      needed.every((o) => prev.includes(o))
        ? prev
        : [...new Set([...prev, ...needed])].sort((a, b) => a - b),
    );
  };

  // RAC's useTable overwrites aria-rowcount (collection size + header
  // rows) when virtualised; the plan fixes it at the data row count,
  // so it is re-asserted after every render.
  const wrapRef = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    wrapRef.current
      ?.querySelector('[role="grid"]')
      ?.setAttribute('aria-rowcount', String(total));
  });

  return (
    <div className={styles.wrap} ref={wrapRef}>
      {error instanceof Error && <p role="alert">{error.message}</p>}
      {first === undefined && error === undefined ? (
        <p>{t.common.loading}</p>
      ) : (
        <Virtualizer
          layout={TableLayout}
          layoutOptions={{ rowHeight: ROW_HEIGHT, headingHeight: HEADING_HEIGHT }}
        >
          <Table
            aria-label={label}
            aria-rowcount={total}
            onScroll={onScroll}
            className={styles.grid ?? ''}
          >
            <TableHeader columns={columns}>
              {(col) => (
                <Column>
                  {col.name}
                  <span className={styles.colMeta}>
                    {col.type} · {col.missing} {t.dataset.missingSuffix}
                  </span>
                </Column>
              )}
            </TableHeader>
            <TableBody items={items}>
              {(item) => (
                <Row>
                  {columns.map((col, i) => (
                    <Cell key={col.id}>{item.cells?.[i] ?? ''}</Cell>
                  ))}
                </Row>
              )}
            </TableBody>
          </Table>
        </Virtualizer>
      )}
      {first !== undefined && total === 0 && <p>{t.dataset.emptyGrid}</p>}
    </div>
  );
}
