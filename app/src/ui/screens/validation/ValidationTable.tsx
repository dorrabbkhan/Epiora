// The `/validation` registry table (v1.0 plan Task 24, spec §7.4):
// every statistic the loaded data names — the measured report's
// `statistics[]`, or the wasm registry entries when the build bundled
// no `validation.json` — searchable by id or method. Search filters
// rows only; it never computes or displays a number itself.

import { useEffect, useMemo, useState } from 'react';
import {
  Column,
  Input,
  Label,
  Table,
  TableBody,
  TableHeader,
  TextField,
} from '../../components';
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
      <div className={styles.tableWrap}>
        <Table caption={t.validation.caption} aria-label={t.validation.caption}>
          <TableHeader>
            <Column isRowHeader>{t.validation.statisticCol}</Column>
            <Column>{t.validation.methodCol}</Column>
            <Column>{t.validation.classCol}</Column>
            <Column>{t.validation.referencesCol}</Column>
            <Column>{t.validation.casesCol}</Column>
            <Column>{t.validation.worstErrorCol}</Column>
          </TableHeader>
          <TableBody>
            {entries.map((entry) => (
              <ValidationEntryRow key={entry.id} entry={entry} />
            ))}
          </TableBody>
        </Table>
      </div>
    </>
  );
}
