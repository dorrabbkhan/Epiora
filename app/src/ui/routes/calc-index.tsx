// Calculator index (v1.0 plan Task 14, spec §7.2): every registry
// calculator listed under its family heading, searchable by title —
// the entry point the `/calc` links fan out from.

import { useMemo, useState } from 'react';
import { Link } from '@tanstack/react-router';
import { useQuery } from '@tanstack/react-query';
import type { CalculatorDef } from '@engine-types/CalculatorDef';
import type { Family } from '@engine-types/Family';
import { Input, Label, TextField } from '../components';
import { calcApi } from '../../calc/client';
import { ct } from '../../calc/strings';
import { t } from '../strings';
import styles from '../screens/calc/calc.module.css';

export function CalcIndexScreen() {
  const [query, setQuery] = useState('');
  const registry = useQuery<CalculatorDef[]>({
    queryKey: ['calc-registry'],
    queryFn: () => calcApi().listCalculators(),
    staleTime: Number.POSITIVE_INFINITY,
  });

  const grouped = useMemo(() => {
    const q = query.trim().toLowerCase();
    const defs = (registry.data ?? []).filter(
      (d) =>
        q === '' ||
        d.id.toLowerCase().includes(q) ||
        ct.calculator(d.id).toLowerCase().includes(q),
    );
    const byFamily = new Map<Family, CalculatorDef[]>();
    for (const d of defs) {
      const list = byFamily.get(d.family) ?? [];
      list.push(d);
      byFamily.set(d.family, list);
    }
    return ct.familyOrder
      .filter((f) => (byFamily.get(f)?.length ?? 0) > 0)
      .map((f) => ({ family: f, defs: byFamily.get(f) ?? [] }));
  }, [registry.data, query]);

  return (
    <div className={styles.layout}>
      <h1 className={styles.title}>{ct.indexTitle}</h1>
      <p className={styles.intro}>{ct.indexIntro}</p>
      <TextField
        className={styles.search ?? ''}
        value={query}
        onChange={setQuery}
        aria-label={ct.searchLabel}
      >
        <Label>{ct.searchLabel}</Label>
        <Input placeholder={ct.searchLabel} />
      </TextField>
      {registry.isPending && <p>{t.common.loading}</p>}
      {registry.isError && (
        <p className={styles.error} role="alert">
          {ct.runFailed}
        </p>
      )}
      {registry.data !== undefined && grouped.length === 0 && (
        <p className={styles.empty}>{ct.noMatches}</p>
      )}
      <div className={styles.families}>
        {grouped.map(({ family, defs }) => (
          <section key={family} className={styles.family}>
            <h2>{ct.familyLabel(family)}</h2>
            <ul className={styles.list}>
              {defs.map((d) => (
                <li key={d.id}>
                  <Link
                    to="/calc/$id"
                    params={{ id: d.id }}
                    className={styles.card}
                  >
                    {ct.calculator(d.id)}
                  </Link>
                </li>
              ))}
            </ul>
          </section>
        ))}
      </div>
    </div>
  );
}
