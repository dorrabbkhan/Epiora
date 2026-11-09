// The TABLES dialog (spec §8.2): exposure and outcome variables, level
// pickers when a variable is two-level Text (filling `VarSpec.levels`;
// the engine never guesses orientation), optional `stratify_by` and
// `include_missing`. While the run RPC is in flight the controls are
// disabled under `aria-busy`; a failure shows inline with a retry.
//
// The pickers default to the category-order pair; under the
// `VITE_TEST_HOOKS=1` e2e build they start empty so the
// `ambiguous_levels` engine error is reachable (plan Task 12 test
// `engine_error_shows_inline_with_retry`).
import { useEffect, useState } from 'react';
import { useQuery } from '@tanstack/react-query';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { Value } from '@engine-types/Value';
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

const TEST_HOOKS = import.meta.env.VITE_TEST_HOOKS === '1';

const PAGE = 5000;

/** Distinct values of a text column, or `null` when it is not
 *  exactly two-level (missing values don't count as a level). */
async function textLevels(
  datasetId: string,
  version: number,
  name: string,
): Promise<string[] | null> {
  const found = new Set<string>();
  let offset = 0;
  for (;;) {
    const page = await dataApi.getVersionPage(datasetId, version, offset, PAGE);
    const index = page.columns.findIndex((c) => c.name === name);
    if (index < 0) return null;
    for (const row of page.rows) {
      const value = row[index];
      if (value !== null && value !== undefined) found.add(value);
      if (found.size > 2) return null;
    }
    offset += page.rows.length;
    if (offset >= page.total || page.rows.length === 0) {
      return found.size === 2 ? [...found].sort() : null;
    }
  }
}

function ColumnSelect({
  label,
  columns,
  selected,
  onSelect,
  busy,
}: {
  label: string;
  columns: GridPage['columns'];
  selected: string | null;
  onSelect: (name: string) => void;
  busy: boolean;
}) {
  const items = columns.map((c) => ({ id: c.name, label: c.name }));
  return (
    <Select
      selectedKey={selected}
      onSelectionChange={(key) => onSelect(String(key))}
      aria-label={label}
      isDisabled={busy}
    >
      <Label>{label}</Label>
      <Button>
        <SelectValue />
      </Button>
      <Popover>
        <ListBox items={items}>
          {(item) => <ListBoxItem id={item.id}>{item.label}</ListBoxItem>}
        </ListBox>
      </Popover>
    </Select>
  );
}

function LevelPicker({
  label,
  levels,
  selected,
  onSelect,
  busy,
}: {
  label: string;
  levels: string[];
  selected: string | null;
  onSelect: (level: string) => void;
  busy: boolean;
}) {
  const items = levels.map((l) => ({ id: l, label: l }));
  return (
    <Select
      selectedKey={selected}
      onSelectionChange={(key) => onSelect(String(key))}
      aria-label={label}
      isDisabled={busy}
    >
      <Label>{label}</Label>
      <Button>
        <SelectValue />
      </Button>
      <Popover>
        <ListBox items={items}>
          {(item) => <ListBoxItem id={item.id}>{item.label}</ListBoxItem>}
        </ListBox>
      </Popover>
    </Select>
  );
}

export function TablesDialog({
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
  run: (options: TablesOptions) => Promise<void>;
}) {
  const columns = useQuery({
    queryKey: ['columns', datasetId, version],
    queryFn: async () =>
      (await dataApi.getVersionPage(datasetId, version, 0, 1)).columns,
    staleTime: Infinity,
  });
  const [exposure, setExposure] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<string | null>(null);
  const [expLevels, setExpLevels] = useState<[string | null, string | null]>([
    null,
    null,
  ]);
  const [outLevels, setOutLevels] = useState<[string | null, string | null]>([
    null,
    null,
  ]);
  const [stratum, setStratum] = useState<string>('none');
  const [includeMissing, setIncludeMissing] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<{ detail: string; message: string } | null>(
    null,
  );

  useEffect(() => setExpLevels([null, null]), [exposure]);
  useEffect(() => setOutLevels([null, null]), [outcome]);

  const cols = columns.data ?? [];
  const colType = (name: string | null) =>
    cols.find((c) => c.name === name)?.type;

  const expLevelQuery = useQuery({
    queryKey: ['text-levels', datasetId, version, exposure],
    enabled: exposure !== null && colType(exposure) === 'text',
    queryFn: () => textLevels(datasetId, version, exposure ?? ''),
    staleTime: Infinity,
  });
  const outLevelQuery = useQuery({
    queryKey: ['text-levels', datasetId, version, outcome],
    enabled: outcome !== null && colType(outcome) === 'text',
    queryFn: () => textLevels(datasetId, version, outcome ?? ''),
    staleTime: Infinity,
  });

  /** Effective [first, second] pair: the user's picks, else the
   *  category-order default (empty under VITE_TEST_HOOKS). */
  const effective = (
    chosen: [string | null, string | null],
    levels: string[] | null | undefined,
  ): [string | null, string | null] => [
    chosen[0] ?? (TEST_HOOKS ? null : (levels?.[0] ?? null)),
    chosen[1] ?? (TEST_HOOKS ? null : (levels?.[1] ?? null)),
  ];
  const effExp = effective(expLevels, expLevelQuery.data);
  const effOut = effective(outLevels, outLevelQuery.data);
  const spec = (
    name: string | null,
    pair: [string | null, string | null],
  ): TablesOptions['exposure'] | null =>
    name === null
      ? null
      : {
          name,
          levels:
            pair[0] !== null && pair[1] !== null
              ? [
                  { type: 'text', value: pair[0] } as Value,
                  { type: 'text', value: pair[1] } as Value,
                ]
              : null,
        };

  const submit = async () => {
    const exp = spec(exposure, effExp);
    const out = spec(outcome, effOut);
    if (exp === null || out === null) return;
    setBusy(true);
    setError(null);
    try {
      await run({
        exposure: exp,
        outcome: out,
        stratify_by: stratum === 'none' ? [] : [stratum],
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

  const stratumItems = [
    { id: 'none', label: t.analysis.noStratum },
    ...cols.map((c) => ({ id: c.name, label: c.name })),
  ];

  return (
    <ModalOverlay isOpen={open} onOpenChange={onOpenChange} isDismissable>
      <Modal>
        <Dialog aria-busy={busy}>
          <Heading slot="title">{t.analysis.tables}</Heading>
          <div className={styles.form}>
            <ColumnSelect
              label={t.analysis.exposure}
              columns={cols}
              selected={exposure}
              onSelect={setExposure}
              busy={busy}
            />
            {exposure !== null && colType(exposure) === 'text' && (
              <>
                {expLevelQuery.isPending && <p>{t.analysis.levelsLoading}</p>}
                {Array.isArray(expLevelQuery.data) && (
                  <>
                    <LevelPicker
                      label={t.analysis.firstLevel(exposure)}
                      levels={expLevelQuery.data}
                      selected={effExp[0]}
                      onSelect={(l) => setExpLevels([l, expLevels[1]])}
                      busy={busy}
                    />
                    <LevelPicker
                      label={t.analysis.secondLevel(exposure)}
                      levels={expLevelQuery.data}
                      selected={effExp[1]}
                      onSelect={(l) => setExpLevels([expLevels[0], l])}
                      busy={busy}
                    />
                  </>
                )}
              </>
            )}
            <ColumnSelect
              label={t.analysis.outcome}
              columns={cols}
              selected={outcome}
              onSelect={setOutcome}
              busy={busy}
            />
            {outcome !== null && colType(outcome) === 'text' && (
              <>
                {outLevelQuery.isPending && <p>{t.analysis.levelsLoading}</p>}
                {Array.isArray(outLevelQuery.data) && (
                  <>
                    <LevelPicker
                      label={t.analysis.firstLevel(outcome)}
                      levels={outLevelQuery.data}
                      selected={effOut[0]}
                      onSelect={(l) => setOutLevels([l, outLevels[1]])}
                      busy={busy}
                    />
                    <LevelPicker
                      label={t.analysis.secondLevel(outcome)}
                      levels={outLevelQuery.data}
                      selected={effOut[1]}
                      onSelect={(l) => setOutLevels([outLevels[0], l])}
                      busy={busy}
                    />
                  </>
                )}
              </>
            )}
            <Select
              selectedKey={stratum}
              onSelectionChange={(key) => setStratum(String(key))}
              aria-label={t.analysis.stratifyBy}
              isDisabled={busy}
            >
              <Label>{t.analysis.stratifyBy}</Label>
              <Button>
                <SelectValue />
              </Button>
              <Popover>
                <ListBox items={stratumItems}>
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
                isDisabled={busy || exposure === null || outcome === null}
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
