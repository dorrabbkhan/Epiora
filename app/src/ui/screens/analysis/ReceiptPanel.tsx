// The collapsible receipt on a result card (spec §8.2): engine version,
// dataset content hash, options, receipt hash and "Copy receipt JSON".
import { useState } from 'react';
import type { Receipt } from '@engine-types/Receipt';
import { Button, Disclosure, DisclosurePanel, Heading } from '../../components';
import { t } from '../../strings';
import styles from './analysis.module.css';

export function ReceiptPanel({ receipt }: { receipt: Receipt }) {
  const [copied, setCopied] = useState(false);
  return (
    <Disclosure className={styles.receipt ?? ''}>
      <Heading level={4}>
        <Button slot="trigger">{t.analysis.receipt}</Button>
      </Heading>
      <DisclosurePanel>
        <dl className={styles.receiptGrid}>
          <dt>{t.analysis.receiptEngine}</dt>
          <dd>
            {receipt.engine.name} {receipt.engine.version} (
            {receipt.engine.git_sha})
          </dd>
          <dt>{t.analysis.receiptHash}</dt>
          <dd>{receipt.dataset.sha256}</dd>
          <dt>{t.analysis.receiptOptions}</dt>
          <dd>
            <code>{JSON.stringify(receipt.options)}</code>
          </dd>
          <dt>{t.analysis.receiptSha}</dt>
          <dd>{receipt.receipt_sha256}</dd>
        </dl>
        <div className={styles.actions}>
          <Button
            onPress={async () => {
              await navigator.clipboard.writeText(JSON.stringify(receipt));
              setCopied(true);
            }}
          >
            {t.analysis.copyReceipt}
          </Button>
          {copied && <span>{t.analysis.copied}</span>}
        </div>
      </DisclosurePanel>
    </Disclosure>
  );
}
