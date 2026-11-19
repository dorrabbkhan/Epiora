// The "Which calculator?" guide route (v1.0 plan Task 23, spec §6.1):
// `/calc/guide?path=a.b.c` resolves the answered letters against the
// guide tree and shows the next question or the recommended
// calculator. Store-less like the rest of `/calc` — the tree is app
// data, and a leaf only links to a calculator.

import { useMemo } from 'react';
import { Link, useSearch } from '@tanstack/react-router';
import { resolveGuide } from '../../calc/guide';
import { ct } from '../../calc/strings';
import { GuideStep } from '../screens/calc/GuideStep';
import styles from '../screens/calc/calc.module.css';

export function CalcGuideScreen() {
  const search = useSearch({ strict: false }) as Record<string, unknown>;
  const path = typeof search.path === 'string' ? search.path : '';
  // A malformed path walks only as far as it can — resolveGuide
  // stops at the last valid node.
  const state = useMemo(() => resolveGuide(path), [path]);
  return (
    <div className={styles.layout}>
      <h1 className={styles.title}>{ct.guide.title}</h1>
      <p className={styles.intro}>
        <Link to="/calc">{ct.backToIndex}</Link>
        {' · '}
        {ct.guide.intro}
      </p>
      <GuideStep state={state} path={path} />
    </div>
  );
}
