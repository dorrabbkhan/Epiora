// The MEANS dialog (v1.0 plan Task 15, spec §4.2/§7.3): one numeric
// analysis variable (Integer, Real or Date — the engine's contract),
// an optional cross-tabulation variable, and missing handling; the
// run appends a result card, an engine failure shows inline with a
// retry.
import { useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import type { MeansOptions } from '@engine-types/MeansOptions';
import {
  Button,
  Checkbox,
  Dialog,
  Heading,
  Label,
  ListBox,
  ListBoxItem,
  Modal,
  ModalOverlay,
  Popover,
  Select,
  SelectValue,
} from '../../components';
import { dataApi } from '../../../worker/client';
import type { GridPage } from '../../../worker/api';
import { RpcError } from '../../../worker/errors';
import { t } from '../../strings';
import styles from './analysis.module.css';

const NUMERIC = new Set(['integer', 'real', 'date']);

export function MeansDialog({
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
  run: (options: MeansOptions) => Promise<void>;
}) {
  const columns = useQuery({
    queryKey: ['columns', datasetId, version],
    queryFn: async () =>
      (await dataApi.getVersionPage(datasetId, version, 0, 1)).columns,
    staleTime: Infinity,
  });
  const [variable, setVariable] = useState<string | null>(null);
  const [groupBy, setGroupBy] = useState<string>('none');
  const [includeMissing, setIncludeMissing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<{ detail: string; message: string } | null>(
    null,
  );

  const cols = columns.data ?? [];
  const numeric: GridPage['columns'] = cols.filter((c) => NUMERIC.has(c.type));
  const groupItems = [
    { id: 'none', label: t.analysis.noGroup },
    ...cols.map((c) => ({ id: c.name, label: c.name })),
  ];

  const submit = async () => {
    if (variable === null) return;
    setBusy(true);
    setError(null);
    try {
      await run({
        variable,
        group_by: groupBy === 'none' ? null : groupBy,
        include_missing: includeMissing,
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
          <Heading slot="title">{t.analysis.means}</Heading>
          {/* RAC Dialog strips `aria-busy`; the marker rides on the
              form instead. */}
          <div className={styles.form} aria-busy={busy}>
            <Select
              selectedKey={variable}
              onSelectionChange={(key) => setVariable(String(key))}
              aria-label={t.analysis.variable}
              isDisabled={busy}
            >
              <Label>{t.analysis.variable}</Label>
              <Button>
                <SelectValue />
              </Button>
              <Popover>
                <ListBox items={numeric.map((c) => ({ id: c.name, label: c.name }))}>
                  {(item) => (
                    <ListBoxItem id={item.id}>{item.label}</ListBoxItem>
                  )}
                </ListBox>
              </Popover>
            </Select>
            <Select
              selectedKey={groupBy}
              onSelectionChange={(key) => setGroupBy(String(key))}
              aria-label={t.analysis.groupBy}
              isDisabled={busy}
            >
              <Label>{t.analysis.groupBy}</Label>
              <Button>
                <SelectValue />
              </Button>
              <Popover>
                <ListBox items={groupItems}>
                  {(item) => (
                    <ListBoxItem id={item.id}>{item.label}</ListBoxItem>
                  )}
                </ListBox>
              </Popover>
            </Select>
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
                isDisabled={busy || variable === null}
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
