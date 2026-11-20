// The `/validation` registry table (v1.0 plan Task 24, spec §7.4):
// every statistic the loaded data names — the measured report's
// `statistics[]`, or the wasm registry entries when the build bundled
// no `validation.json` — searchable by id or method. Search filters
// rows only; it never computes or displays a number itself.

import { useEffect, useMemo, useState } from 'react';
import { Input, Label, TextField } from '../../components';
import type { ValidationData } from '../../../validation/data';
import { t } from '../../strings';
import { ValidationEntryRow } from './ValidationEntry';
import styles from './validation.module.css';

export function ValidationTable({ data }: { data: ValidationData }) {
  const [query, setQuery] = useState('');
  const entries = useMemo(() => {
    const q = query.trim().toLowerCase();
    return data.statistics.filter(
      (e) =>
        q === '' ||
        e.id.toLowerCase().includes(q) ||
        e.method.toLowerCase().includes(q),
    );
  }, [data.statistics, query]);

  // `/validation#<id>` anchors (spec §7.4): once the rows exist, move
  // to the fragment the URL names — explain links land here.
  useEffect(() => {
    const id = decodeURIComponent(window.location.hash.slice(1));
    if (id !== '') {
      document.getElementById(id)?.scrollIntoView();
    }
  }, [entries]);

  return (
    <>
      <TextField
        className={styles.search ?? ''}
        value={query}
        onChange={setQuery}
        aria-label={t.validation.searchLabel}
      >
        <Label>{t.validation.searchLabel}</Label>
        <Input placeholder={t.validation.searchLabel} />
      </TextField>
      {entries.length === 0 && (
        <p className={styles.notice}>{t.validation.noMatches}</p>
      )}
      {/* The wrap scrolls horizontally on narrow viewports; the
          scrollable region must be keyboard-reachable (axe
          scrollable-region-focusable) — focusable and named by the
          table's caption. */}
      <div
        className={styles.tableWrap}
        role="region"
        aria-labelledby="validation-table-caption"
        tabIndex={0}
      >
        {/* A plain static table, not the interactive Epiora Table:
            ~690 collection rows through React Aria made every row and
            cell a tabbable, data-attributed node and axe's DOM walk
            timed out the a11y e2e. `role="grid"` + the implicit
            rowheader/gridcell roles keep the markup contract the
            page's e2e locators use (`grid` named by the caption,
            `gridcell` per data cell). */}
        <table
          className={styles.table}
          role="grid"
          aria-label={t.validation.caption}
        >
          <caption id="validation-table-caption" className={styles.caption}>
            {t.validation.caption}
          </caption>
          <thead>
            <tr>
              <th scope="col" className={styles.column}>
                {t.validation.statisticCol}
              </th>
              <th scope="col" className={styles.column}>
                {t.validation.methodCol}
              </th>
              <th scope="col" className={styles.column}>
                {t.validation.classCol}
              </th>
              <th scope="col" className={styles.column}>
                {t.validation.referencesCol}
              </th>
              <th scope="col" className={styles.column}>
                {t.validation.casesCol}
              </th>
              <th scope="col" className={styles.column}>
                {t.validation.worstErrorCol}
              </th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry) => (
              <ValidationEntryRow key={entry.id} entry={entry} />
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}
