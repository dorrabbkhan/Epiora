// Calculator receipt (v1.0 plan Task 14, spec §5.4): the v3 receipt
// on a calculator run — engine stamp, the calculator's canonical
// inputs, receipt hash and "Copy receipt JSON" — same shape as the
// analysis receipt panel but driven by the `calculator` stamp (a
// calculator run has no dataset stamp).

import { useState } from 'react';
import type { Receipt } from '@engine-types/Receipt';
import { Button, Disclosure, DisclosurePanel, Heading } from '../../components';
import { ct } from '../../../calc/strings';
import { t } from '../../strings';
import styles from './calc.module.css';

export function CalcReceipt({ receipt }: { receipt: Receipt }) {
  const [copied, setCopied] = useState(false);
  return (
    <Disclosure className={styles.receipt ?? ''}>
      <Heading level={3}>
        <Button slot="trigger">{ct.receipt}</Button>
      </Heading>
      <DisclosurePanel>
        <dl className={styles.receiptGrid}>
          <dt>{ct.engineLabel}</dt>
          <dd>
            {receipt.engine.name} {receipt.engine.version} (
            {receipt.engine.git_sha})
          </dd>
          <dt>{ct.inputsUsed}</dt>
          <dd>
            <code>{JSON.stringify(receipt.calculator?.inputs ?? null)}</code>
          </dd>
          <dt>{ct.receiptHash}</dt>
          <dd>{receipt.receipt_sha256}</dd>
        </dl>
        <div className={styles.actions}>
          <Button
            onPress={async () => {
              await navigator.clipboard.writeText(JSON.stringify(receipt));
              setCopied(true);
            }}
          >
            {ct.copyReceipt}
          </Button>
          {copied && <span>{t.analysis.copied}</span>}
        </div>
      </DisclosurePanel>
    </Disclosure>
  );
}
