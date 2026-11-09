// The FREQ dialog (spec §8.2): one or more variables and the missing
// handling; the run appends a result card, an engine failure shows
// inline with a retry.
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import type { FreqOptions } from '@engine-types/FreqOptions';
import {
  Button,
  Checkbox,
  Dialog,
  Heading,
  Modal,
  ModalOverlay,
} from '../../components';
import { dataApi } from '../../../worker/client';
import { RpcError } from '../../../worker/errors';
import { t } from '../../strings';
import styles from './analysis.module.css';

export function FreqDialog({
  open,
  onOpenChange,
  datasetId,
  version,
  run,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  datasetId: string;
  version: number;
  run: (options: FreqOptions) => Promise<void>;
}) {
  const columns = useQuery({
    queryKey: ['columns', datasetId, version],
    queryFn: async () =>
      (await dataApi.getVersionPage(datasetId, version, 0, 1)).columns,
    staleTime: Infinity,
  });
  const [variables, setVariables] = useState<ReadonlySet<string>>(new Set());
  const [includeMissing, setIncludeMissing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<{ detail: string; message: string } | null>(
    null,
  );

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      await run({
        variables: [...variables],
        stratify_by: null,
        include_missing: includeMissing,
        ci_method: 'clopper_pearson',
      });
      onOpenChange(false);
    } catch (e) {
      setError(
        e instanceof RpcError
          ? { detail: e.detail, message: e.message }
          : { detail: 'internal_error', message: String(e) },
      );
    } finally {
      setBusy(false);
    }
  };

  return (
    <ModalOverlay isOpen={open} onOpenChange={onOpenChange} isDismissable>
      <Modal>
        <Dialog>
          <Heading slot="title">{t.analysis.freq}</Heading>
          {/* RAC Dialog strips `aria-busy`; the marker rides on the
              form instead. */}
          <div className={styles.form} aria-busy={busy}>
            <fieldset className={styles.variables}>
              <legend>{t.analysis.variables}</legend>
              {(columns.data ?? []).map((c) => (
                <Checkbox
                  key={c.name}
                  isSelected={variables.has(c.name)}
                  isDisabled={busy}
                  onChange={(selected) => {
                    const next = new Set(variables);
                    if (selected) {
                      next.add(c.name);
                    } else {
                      next.delete(c.name);
                    }
                    setVariables(next);
                  }}
                >
                  {c.name}
                </Checkbox>
              ))}
            </fieldset>
            <Checkbox
              isSelected={includeMissing}
              isDisabled={busy}
              onChange={setIncludeMissing}
            >
              {t.analysis.includeMissing}
            </Checkbox>
            {error !== null && (
              <div role="alert">
                <p className={styles.error}>
                  {(t.errors as Record<string, string>)[error.detail] ??
                    t.errors.internal_error}
                </p>
                <p>{error.message}</p>
                <Button onPress={() => void submit()} isDisabled={busy}>
                  {t.analysis.retry}
                </Button>
              </div>
            )}
            <div className={styles.actions}>
              <Button
                onPress={() => void submit()}
                isDisabled={busy || variables.size === 0}
              >
                {t.analysis.run}
              </Button>
              <Button slot="close">{t.common.cancel}</Button>
            </div>
          </div>
        </Dialog>
      </Modal>
    </ModalOverlay>
  );
}
