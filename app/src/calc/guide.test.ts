// "Which calculator?" guide data (v1.0 plan Task 23, spec §6.1):
// every registry calculator must be reachable as a leaf, the tree
// must have no cycles or dangling `next` ids, and every leaf's
// `prefill` must decode cleanly against the calculator's own input
// definition — the registry comes from the Node wasm build
// (`crates/epiora-wasm/pkg-parity`, built by scripts/check-app.sh),
// the same source `guard.test.ts` reads.
import { createRequire } from 'node:module';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import type { CalculatorDef } from '@engine-types/CalculatorDef';
import { GUIDE, GUIDE_ROOT, isLeaf, type GuideLeaf } from './guide';
import { decodeInputs } from './url-state';

const PARITY_WASM = resolve(
  process.cwd(),
  '../crates/epiora-wasm/pkg-parity/epiora_wasm.js',
);

interface ParityWasm {
  calcRegistry(): CalculatorDef[];
}

function registry(): CalculatorDef[] {
  const require = createRequire(import.meta.url);
  return (require(PARITY_WASM) as ParityWasm).calcRegistry();
}

function leaves(): GuideLeaf[] {
  return Object.values(GUIDE).filter(isLeaf);
}

describe('which-calculator guide', () => {
  it('every_registry_calculator_reachable', () => {
    const defs = registry();
    expect(defs.length).toBeGreaterThan(0);
    const reached = new Set(leaves().map((l) => l.calculator));
    const missing = defs.map((d) => d.id).filter((id) => !reached.has(id));
    expect(missing).toEqual([]);
    // And no leaf names a calculator the registry does not know.
    const ids = new Set(defs.map((d) => d.id));
    const unknown = leaves()
      .map((l) => l.calculator)
      .filter((id) => !ids.has(id));
    expect(unknown).toEqual([]);
  });

  it('no_cycles_no_dangling_ids', () => {
    expect(GUIDE[GUIDE_ROOT]).toBeDefined();
    // Every option's `next` names a declared node.
    for (const node of Object.values(GUIDE)) {
      if (isLeaf(node)) continue;
      for (const option of node.options) {
        expect(
          GUIDE[option.next],
          `${node.id} → ${option.next}`,
        ).toBeDefined();
      }
    }
    // Depth-first from the root: a tree never revisits a node on a
    // path, and every declared node is reachable.
    const visiting = new Set<string>();
    const done = new Set<string>();
    const visit = (id: string): void => {
      expect(visiting.has(id), `cycle through ${id}`).toBe(false);
      if (done.has(id)) return;
      visiting.add(id);
      const node = GUIDE[id];
      expect(node, id).toBeDefined();
      if (node !== undefined && !isLeaf(node)) {
        for (const option of node.options) visit(option.next);
      }
      visiting.delete(id);
      done.add(id);
    };
    visit(GUIDE_ROOT);
    const unreachable = Object.keys(GUIDE).filter((id) => !done.has(id));
    expect(unreachable).toEqual([]);
  });

  it('prefills_decode_cleanly', () => {
    const defs = new Map(registry().map((d) => [d.id, d]));
    const withPrefill = leaves().filter((l) => l.prefill !== undefined);
    expect(withPrefill.length).toBeGreaterThan(0);
    for (const leaf of withPrefill) {
      const def = defs.get(leaf.calculator);
      expect(def, leaf.calculator).toBeDefined();
      const decoded = decodeInputs(
        def as CalculatorDef,
        new URLSearchParams(leaf.prefill),
      );
      expect(decoded.errors, `${leaf.calculator} prefill`).toEqual({});
      for (const key of Object.keys(leaf.prefill ?? {})) {
        expect(
          decoded.values[key],
          `${leaf.calculator}.${key}`,
        ).toBeDefined();
      }
    }
  });
});
