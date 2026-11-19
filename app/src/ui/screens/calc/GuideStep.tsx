// One step of the "Which calculator?" guide (v1.0 plan Task 23, spec
// §6.1): a question with its "why" line and one link per option —
// each answer becomes a letter of the `?path=` state, so a guide
// state can be linked — or, at a leaf, the recommended calculator
// with its prefilled link. Options are links, so the whole flow is
// keyboard- and offline-reachable.

import { Link } from '@tanstack/react-router';
import { isLeaf, optionLetter, type GuideState } from '../../../calc/guide';
import { ct } from '../../../calc/strings';
import styles from './GuideStep.module.css';

export function GuideStep({
  state,
  path,
}: {
  state: GuideState;
  /** The `?path=` the state was resolved from — option links append
   *  their letter to it. */
  path: string;
}) {
  const { node, trail } = state;
  // The parent's path is the current path minus its last step.
  const back = path.replace(/\.?[a-z]?$/, '');
  return (
    <div className={styles.step}>
      {isLeaf(node) ? (
        <>
          <h2 className={styles.heading}>{ct.guide.recommended}</h2>
          <ul className={styles.options}>
            <li>
              <Link
                to="/calc/$id"
                params={{ id: node.calculator }}
                search={node.prefill ?? {}}
                className={styles.option}
              >
                {ct.guide.open(ct.calculator(node.calculator))}
              </Link>
            </li>
          </ul>
        </>
      ) : (
        <>
          <h2 className={styles.heading}>{node.question}</h2>
          <p className={styles.why}>{node.why}</p>
          <ul className={styles.options}>
            {node.options.map((option, i) => {
              const letter = optionLetter(i);
              const next = path === '' ? letter : `${path}.${letter}`;
              return (
                <li key={option.next}>
                  <Link
                    to="/calc/guide"
                    search={{ path: next }}
                    className={styles.option}
                  >
                    {option.label}
                  </Link>
                </li>
              );
            })}
          </ul>
        </>
      )}
      {trail.length > 0 && (
        <nav className={styles.nav} aria-label={ct.guide.navLabel}>
          <Link to="/calc/guide" search={{ path: back }}>
            {ct.guide.back}
          </Link>
          <Link to="/calc/guide" search={{}}>
            {ct.guide.startOver}
          </Link>
        </nav>
      )}
    </div>
  );
}
