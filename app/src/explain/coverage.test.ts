// Formula-template coverage (v1.0 plan Task 16 Step 1, spec §6):
// the engine's `FORMULAS` list is parsed out of
// `crates/epiora-core/src/explain/formulas.rs` (the list has no wasm
// export — the source is the oracle) and every id must have an app
// template, with no template lacking an engine id.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { TEMPLATES } from './index';

// Vitest runs with cwd = app/ — resolve against it (import.meta.url
// is not file:// inside the worker transform).
const formulasRs = readFileSync(
  resolve(process.cwd(), '../crates/epiora-core/src/explain/formulas.rs'),
  'utf8',
);

function engineFormulas(): string[] {
  const m = /pub const FORMULAS[^=]*=\s*&\[([\s\S]*?)\];/.exec(formulasRs);
  if (m === null) throw new Error('FORMULAS const not found');
  return [...(m[1]?.matchAll(/"([^"]+)"/g) ?? [])].map((x) => x[1] ?? '');
}

describe('formula template coverage', () => {
  it('parses a nonempty FORMULAS list', () => {
    expect(engineFormulas().length).toBeGreaterThan(0);
  });

  it('every_engine_formula_has_a_template', () => {
    const missing = engineFormulas().filter(
      (id) => TEMPLATES[id] === undefined,
    );
    expect(missing).toEqual([]);
  });

  it('no_orphan_templates', () => {
    const ids = new Set(engineFormulas());
    const orphans = Object.keys(TEMPLATES).filter((id) => !ids.has(id));
    expect(orphans).toEqual([]);
  });
});
