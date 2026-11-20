// Validation page (v1.0 plan Task 24, spec §7.4): the public dossier
// listing every `validation` registry statistic — method, references,
// tolerance class and the worst error the CI `validation-report` job
// measured — plus the engine version, commit and run that produced
// the numbers. Store-less and offline like `/calc`: the page reads a
// static `validation.json` bundled by the app build and falls back to
// the store-less calc worker's `validationRegistry()` when no report
// was bundled; it never touches the project store. Every rendered
// number is a value in that JSON.

import { useQuery } from '@tanstack/react-query';
import { loadValidationData } from '../../validation/data';
import { t } from '../strings';
import { ValidationTable } from '../screens/validation/ValidationTable';
import styles from '../screens/validation/validation.module.css';

export function ValidationScreen() {
  const data = useQuery({
    queryKey: ['validation-data'],
    queryFn: () => loadValidationData(),
    staleTime: Number.POSITIVE_INFINITY,
    retry: false,
  });

  return (
    <div className={styles.layout}>
      <h1 className={styles.title}>{t.validation.title}</h1>
      <p className={styles.intro}>{t.validation.intro}</p>
      {data.isPending && <p>{t.common.loading}</p>}
      {data.isError && (
        <p className={styles.error} role="alert">
          {t.validation.error}
        </p>
      )}
      {data.data !== undefined && data.data.measured && (
        <p className={styles.meta}>
          {t.validation.engine}: {data.data.engine_version} ·{' '}
          {t.validation.commit}: {data.data.commit} ·{' '}
          <a href={data.data.run_url}>{t.validation.run}</a>
        </p>
      )}
      {data.data !== undefined && !data.data.measured && (
        <p className={styles.notice}>{t.validation.notMeasured}</p>
      )}
      {data.data !== undefined && <ValidationTable data={data.data} />}
    </div>
  );
}
