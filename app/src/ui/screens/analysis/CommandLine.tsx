// The equivalent Epi Info command on every result card (v1.0 plan
// Task 15, spec §7.3/G4): the worker's `epiInfoCommand` output, stored
// with the run (or generated on load for pre-v2 rows), shown verbatim
// with a Copy button.
import { useState, type CSSProperties } from 'react';
import { Button } from '../../components';
import { t } from '../../strings';
import styles from './analysis.module.css';

/** `* Epiora:` comment lines must not reflow into the command. */
const PRE: CSSProperties = { whiteSpace: 'pre-wrap', margin: 0 };

export function CommandLine({ command }: { command: string }) {
  const [copied, setCopied] = useState(false);
  return (
    <div className={styles.receipt}>
      <code style={PRE}>{command}</code>
      <div className={styles.actions}>
        <Button
          onPress={async () => {
            await navigator.clipboard.writeText(command);
            setCopied(true);
          }}
        >
          {t.analysis.copyCommand}
        </Button>
        {copied && <span>{t.analysis.commandCopied}</span>}
      </div>
    </div>
  );
}
