// URL-state unit tests (v1.0 plan Task 14 Step 1): the registry is
// loaded from the real wasm build so the round-trip covers every
// declared calculator, kind and default — no hand-copied defs.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { beforeAll, describe, expect, it } from 'vitest';
import initEngine, { calcRegistry } from '@engine/epiora_wasm';
import type { CalculatorDef } from '@engine-types/CalculatorDef';
import type { InputDef } from '@engine-types/InputDef';
import {
  collectInputs,
  decodeInputs,
  defaultDraft,
  draftsFromParams,
  encodeInputs,
} from './url-state';

let registry: CalculatorDef[] = [];

beforeAll(async () => {
  // `import.meta.url` is rewritten to a (dead) dev-server URL in this
  // project, so the wasm path resolves from the app root cwd instead.
  const wasm = readFileSync(
    join(
      process.cwd(),
      '../crates/epiora-wasm/pkg/epiora_wasm_bg.wasm',
    ),
  );
  await initEngine(wasm);
  registry = calcRegistry() as CalculatorDef[];
});

function def(id: string): CalculatorDef {
  const d = registry.find((r) => r.id === id);
  if (d === undefined) throw new Error(`registry lacks ${id}`);
  return d;
}

function isTable(input: InputDef): boolean {
  const kind = input.kind;
  return typeof kind === 'object' && 'table' in kind;
}

describe('url-state', () => {
  it('round_trips_every_registry_calculator_defaults', () => {
    expect(registry.length).toBe(24);
    for (const d of registry) {
      // Every declared default encoded then decoded must come back as
      // the same canonical value.
      const values: Record<string, unknown> = {};
      for (const input of d.inputs) {
        if (isTable(input)) continue;
        const draft = defaultDraft(input);
        if (draft !== undefined) values[input.name] = draft;
      }
      const params = encodeInputs(d, values);
      const decoded = decodeInputs(d, params);
      expect(decoded.errors, `${d.id} default round-trip`).toEqual({});
      for (const [name, v] of Object.entries(values)) {
        const input = d.inputs.find((i) => i.name === name)!;
        const kind = input.kind;
        const expected =
          typeof kind === 'object' && 'choice' in kind ? v : Number(v);
        expect(decoded.values[name], `${d.id}.${name}`).toEqual(expected);
      }
    }
  });

  it('table_encoding', () => {
    const d = def('two_by_two');
    const params = encodeInputs(d, {
      strata: [
        [
          [12, 5],
          [3, 40],
        ],
      ],
    });
    expect(params.get('strata')).toBe('12,5;3,40');
    const decoded = decodeInputs(d, params);
    expect(decoded.errors).toEqual({});
    expect(decoded.values.strata).toEqual([
      [
        [12, 5],
        [3, 40],
      ],
    ]);
    // A second stratum separates with `|`.
    const two = encodeInputs(d, {
      strata: [
        [
          [1, 2],
          [3, 4],
        ],
        [
          [5, 6],
          [7, 8],
        ],
      ],
    });
    expect(two.get('strata')).toBe('1,2;3,4|5,6;7,8');
    expect(decodeInputs(d, two).values.strata).toEqual([
      [
        [1, 2],
        [3, 4],
      ],
      [
        [5, 6],
        [7, 8],
      ],
    ]);
  });

  it('unknown_keys_reported', () => {
    const d = def('two_by_two');
    const decoded = decodeInputs(
      d,
      new URLSearchParams('strata=12,5;3,40&zz=1&qq=x'),
    );
    expect(decoded.errors._unknown).toBe('zz,qq');
    expect(decoded.values.strata).toEqual([
      [
        [12, 5],
        [3, 40],
      ],
    ]);
  });

  it('bad_values_reported_not_thrown', () => {
    const d = def('mean_ci');
    expect(() =>
      decodeInputs(d, new URLSearchParams('mean=abc&n=-3&zz=1')),
    ).not.toThrow();
    const decoded = decodeInputs(
      d,
      new URLSearchParams('mean=abc&n=-3&zz=1'),
    );
    expect(decoded.errors.mean).toBeDefined();
    expect(decoded.errors.n).toBeDefined();
    expect(decoded.errors._unknown).toBe('zz');
    expect(decoded.values.mean).toBeUndefined();
    // A malformed table keeps its raw draft and reports a cell error.
    const t2 = def('two_by_two');
    const bad = decodeInputs(
      t2,
      new URLSearchParams('strata=abc,5;3,40'),
    );
    expect(bad.errors.strata).toBeDefined();
    const drafts = draftsFromParams(
      t2,
      new URLSearchParams('strata=abc,5;3,40'),
    );
    expect(drafts.strata).toEqual([[['abc', '5'], ['3', '40']]]);
    // collectInputs reports, never throws.
    const collected = collectInputs(t2, drafts);
    expect(collected.errors.strata).toBeDefined();
  });

  it('missing_table_row_reports_bad_shape', () => {
    const d = def('two_by_two');
    const decoded = decodeInputs(
      d,
      new URLSearchParams('strata=10,11;;4,20'),
    );
    expect(decoded.errors.strata).toBe('bad_table_shape');
  });

  it('one_of_group_two_given_reports', () => {
    const d = def('t_test_summary');
    const members = d.groups[0]!.members;
    const drafts = Object.fromEntries(members.slice(0, 2).map((m) => [m, '1']));
    const collected = collectInputs(d, drafts);
    expect(collected.errors[members.join('/')]).toBe('one_of');
  });
});
