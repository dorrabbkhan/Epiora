// The read-only data grid (spec §8.1, plan Task 10): a manually
// windowed `role="grid"` paged in 100-row chunks via `useVersionPages`
// — the scroll handler keeps only the visible range plus one page of
// lookahead loaded, and only the rows inside that window are mounted.
// `aria-rowcount` is the version's full row count. Column headers
// show name, type and missing count.
//
// v1.0 Task 19: React Aria's `Table` + `Virtualizer` built the whole
// collection — every one of the 1,000,000 `items` became a Row element
// in the hidden collection tree (plus ~11 Cell elements each), and the
// 1M-entry `items` array was rebuilt on every loaded page. The
// renderer ran out of memory just opening the dataset (`Target
// crashed`, React error #520 — CI run 44485826291, `grid_scroll_1m`
// 0/10). The component set's `Table`/`Virtualizer` can only virtualise
// a collection it fully materialises, so the grid renders its rows
// itself inside a fixed-height window. A 1M-row grid at 48 px rows is
// also 48,000,000 px tall — past every engine's element-height cap
// (Chromium clamps scrollTop near 2^25 px ≈ row 698,900), so once
// `total * ROW_HEIGHT` exceeds MAX_SPAN the scroll space compresses:
// the scrollbar maps to a fractional first row and the rendered window
// re-anchors to it while keeping real 48 px spacing.
import {
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type FocusEvent,
  type KeyboardEvent,
  type ReactNode,
  type UIEvent,
} from 'react';
import { useVersionPages } from '../queries';
import { t } from '../strings';
import styles from './DataGrid.module.css';
import type { GridPage } from '../../worker/api';

const PAGE = 100;
// The rendered height of a data row and the column header: rows are
// absolutely positioned at these offsets and onScroll derives the
// visible page range from them, so they must match the real heights
// produced by the cell/column padding and font sizes.
const ROW_HEIGHT = 48;
const HEADING_HEIGHT = 64;
// Cap on the scrollable body height, comfortably below every engine's
// element-height limit (Firefox ≈17.9M px, Chromium/WebKit ≈33.5M px).
// Taller datasets switch to the compressed scroll space described
// above (~166k rows uncompressed).
const MAX_SPAN = 8_000_000;

const cellStyle = {
  flex: '1 0 0',
  minWidth: 0,
  padding: 'var(--space-2) var(--space-3)',
  borderBottom: '1px solid var(--color-border)',
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
  outline: 'none',
} as const;

const focusOutline = {
  outline:
    'var(--focus-ringWidth) var(--focus-ringStyle) var(--color-focus)',
  outlineOffset: 'calc(-1 * var(--focus-ringWidth))',
} as const;

/** The grid's scroll geometry for the current container and row
 *  count. `stride` is the scroll-space height of one row — exactly
 *  `ROW_HEIGHT` uncompressed, less once the space compresses. */
interface View {
  /** Fractional index of the first visible row. */
  topRow: number;
  /** Scroll-space pixels per row (`ROW_HEIGHT` uncompressed). */
  stride: number;
  /** Full data rows the viewport fits (lookahead for `needed`). */
  viewportRows: number;
  /** The rowgroup's rendered height in px (≤ MAX_SPAN). */
  span: number;
  compressed: boolean;
}

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

  // Columns and the row count are identical on every page; keep the
  // last arrived page so neither collapses while the window slides
  // over pages that are still being fetched.
  const meta = useRef<GridPage | null>(null);
  const arrived = pages.find((p) => p.data !== undefined)?.data;
  if (arrived !== undefined) meta.current = arrived;
  const total = meta.current?.total ?? 0;
  const columns = (meta.current?.columns ?? []).map((c) => ({
    ...c,
    id: c.name,
  }));

  const [view, setView] = useState<View>({
    topRow: 0,
    stride: ROW_HEIGHT,
    viewportRows: 0,
    span: 0,
    compressed: false,
  });

  // Loaded rows by absolute index; rows inside the window that have
  // not landed yet render empty cells.
  const loaded = useMemo(() => {
    const map = new Map<number, (string | null)[]>();
    pages.forEach((result, i) => {
      const offset = offsets[i];
      if (offset === undefined) return;
      result.data?.rows.forEach((row, j) => map.set(offset + j, row));
    });
    return map;
  }, [pages, offsets]);

  // The rendered window covers every requested page.
  const renderStart = offsets.length === 0 ? 0 : offsets[0]!;
  const renderEnd =
    offsets.length === 0
      ? 0
      : Math.min(total, offsets[offsets.length - 1]! + PAGE);

  // Roving tabindex: the anchor cell is the single tabbable cell; when
  // it is outside the rendered window the window's first cell takes
  // over so the grid stays reachable by Tab.
  const [anchor, setAnchor] = useState({ r: 0, c: 0 });
  const [focused, setFocused] = useState<{ r: number; c: number } | null>(null);
  const anchorRendered =
    anchor.r >= renderStart && anchor.r < renderEnd && anchor.c < columns.length;
  const tabbable = anchorRendered
    ? anchor
    : { r: renderStart, c: Math.min(anchor.c, Math.max(0, columns.length - 1)) };

  const error = pages.find((p) => p.error !== null)?.error;

  const gridRef = useRef<HTMLDivElement>(null);
  const pendingFocus = useRef<{ r: number; c: number } | null>(null);

  /** Recomputes the scroll geometry from the container and `total`,
   *  then derives the page offsets the viewport needs (one lookahead
   *  page each way). Shared by onScroll and the resize observer. */
  const updateView = (el: HTMLElement) => {
    const compressed = total * ROW_HEIGHT > MAX_SPAN;
    const span = Math.min(total * ROW_HEIGHT, MAX_SPAN);
    const vpH = Math.max(0, el.clientHeight - HEADING_HEIGHT);
    const viewportRows = Math.ceil(vpH / ROW_HEIGHT) + 1;
    // scrollTop maps [0, HEADING + span − clientHeight] onto
    // firstRow [0, total − viewportRows].
    const stride = compressed
      ? Math.max(1, span - vpH) / Math.max(1, total - viewportRows)
      : ROW_HEIGHT;
    const topRow = compressed
      ? Math.max(0, Math.min(total - viewportRows, (el.scrollTop - HEADING_HEIGHT) / stride))
      : Math.max(0, Math.floor((el.scrollTop - HEADING_HEIGHT) / ROW_HEIGHT));
    setView((prev) =>
      prev.topRow === topRow &&
      prev.stride === stride &&
      prev.viewportRows === viewportRows &&
      prev.span === span &&
      prev.compressed === compressed
        ? prev
        : { topRow, stride, viewportRows, span, compressed },
    );
    const needed: number[] = [];
    const lo = Math.max(0, Math.floor(topRow) - PAGE);
    const hi = Math.min(total, Math.ceil(topRow + viewportRows) + PAGE);
    for (let o = Math.floor(lo / PAGE) * PAGE; o < hi; o += PAGE) {
      needed.push(o);
    }
    setOffsets((prev) =>
      needed.length === prev.length && needed.every((o, i) => o === prev[i])
        ? prev
        : needed,
    );
  };

  const onScroll = (e: UIEvent<HTMLElement>) => updateView(e.currentTarget);

  // A resize can expose rows no scroll event ever asked for.
  useLayoutEffect(() => {
    const el = gridRef.current;
    if (el === null) return;
    updateView(el);
    const observer = new ResizeObserver(() => updateView(el));
    observer.observe(el);
    return () => observer.disconnect();
  });

  const cellAt = (r: number, c: number) =>
    gridRef.current?.querySelector<HTMLElement>(
      `[role="gridcell"][data-r="${r}"][data-c="${c}"]`,
    ) ?? null;
  const focusCellAt = (r: number, c: number) => {
    const cell = cellAt(r, c);
    if (cell !== null) {
      cell.focus();
      return;
    }
    // The target row is outside the rendered window: scroll it into
    // view and let the layout effect focus it once it is mounted.
    pendingFocus.current = { r, c };
    const el = gridRef.current;
    if (el !== null) {
      el.scrollTop = view.compressed
        ? HEADING_HEIGHT + r * view.stride
        : HEADING_HEIGHT + (r - 1) * ROW_HEIGHT;
    }
  };
  useLayoutEffect(() => {
    const pending = pendingFocus.current;
    if (pending === null) return;
    const cell = cellAt(pending.r, pending.c);
    if (cell !== null) {
      pendingFocus.current = null;
      cell.focus();
    }
  });

  const onKeyDown = (e: KeyboardEvent<HTMLElement>) => {
    const active = document.activeElement;
    if (!(active instanceof HTMLElement)) return;
    if (active.getAttribute('role') !== 'gridcell') return;
    const r = Number(active.dataset.r);
    const c = Number(active.dataset.c);
    let nr = r;
    let nc = c;
    if (e.key === 'ArrowRight') nc = Math.min(columns.length - 1, c + 1);
    else if (e.key === 'ArrowLeft') nc = Math.max(0, c - 1);
    else if (e.key === 'ArrowDown') nr = Math.min(total - 1, r + 1);
    else if (e.key === 'ArrowUp') nr = Math.max(0, r - 1);
    else return;
    e.preventDefault();
    if (nr !== r || nc !== c) focusCellAt(nr, nc);
  };

  const onFocusCapture = (e: FocusEvent<HTMLElement>) => {
    const target = e.target;
    if (
      target instanceof HTMLElement &&
      target.getAttribute('role') === 'gridcell'
    ) {
      const cell = { r: Number(target.dataset.r), c: Number(target.dataset.c) };
      setAnchor(cell);
      setFocused(cell);
    }
  };
  const onBlurCapture = (e: FocusEvent<HTMLElement>) => {
    if (
      !(e.relatedTarget instanceof Node) ||
      !e.currentTarget.contains(e.relatedTarget)
    ) {
      setFocused(null);
    }
  };

  // A row's rendered top. Uncompressed it is its real offset;
  // compressed it re-anchors to the fractional first row so visible
  // rows always keep the real ROW_HEIGHT spacing.
  const rowTop = (r: number): number =>
    view.compressed
      ? HEADING_HEIGHT + view.topRow * view.stride + (r - view.topRow) * ROW_HEIGHT
      : HEADING_HEIGHT + r * ROW_HEIGHT;

  const rows: ReactNode[] = [];
  for (let r = renderStart; r < renderEnd; r++) {
    rows.push(
      <div
        key={r}
        role="row"
        aria-rowindex={r + 2}
        style={{
          position: 'absolute',
          top: rowTop(r) - HEADING_HEIGHT,
          left: 0,
          right: 0,
          height: ROW_HEIGHT,
          display: 'flex',
        }}
      >
        {columns.map((col, c) => (
          <div
            key={col.id}
            role="gridcell"
            aria-colindex={c + 1}
            data-r={r}
            data-c={c}
            tabIndex={tabbable.r === r && tabbable.c === c ? 0 : -1}
            onClick={(e) => e.currentTarget.focus()}
            style={{
              ...cellStyle,
              ...(focused !== null && focused.r === r && focused.c === c
                ? focusOutline
                : {}),
            }}
          >
            {loaded.get(r)?.[c] ?? ''}
          </div>
        ))}
      </div>,
    );
  }

  return (
    <div className={styles.wrap}>
      {error instanceof Error && <p role="alert">{error.message}</p>}
      {meta.current === null && error === undefined ? (
        <p>{t.common.loading}</p>
      ) : (
        <div
          ref={gridRef}
          className={styles.grid}
          role="grid"
          aria-label={label}
          aria-rowcount={total}
          aria-colcount={columns.length}
          tabIndex={0}
          onScroll={onScroll}
          onKeyDown={onKeyDown}
          onFocusCapture={onFocusCapture}
          onBlurCapture={onBlurCapture}
          style={{
            color: 'var(--color-text)',
            fontSize: 'var(--font-size3)',
          }}
        >
          <div role="rowgroup">
            <div
              role="row"
              aria-rowindex={1}
              style={{
                position: 'sticky',
                top: 0,
                zIndex: 1,
                display: 'flex',
                height: HEADING_HEIGHT,
                background: 'var(--color-bg)',
                borderBottom: '1px solid var(--color-border)',
              }}
            >
              {columns.map((col, c) => (
                <div
                  key={col.id}
                  role="columnheader"
                  aria-colindex={c + 1}
                  style={{
                    ...cellStyle,
                    fontWeight: 'var(--font-weightBold)',
                    textAlign: 'left',
                  }}
                >
                  {col.name}
                  <span className={styles.colMeta}>
                    {col.type} · {col.missing} {t.dataset.missingSuffix}
                  </span>
                </div>
              ))}
            </div>
          </div>
          <div
            role="rowgroup"
            style={{ position: 'relative', height: view.span || total * ROW_HEIGHT }}
          >
            {rows}
          </div>
        </div>
      )}
      {meta.current !== null && total === 0 && <p>{t.dataset.emptyGrid}</p>}
    </div>
  );
}
