// Assumption-note text coverage (v1.0 plan Task 23, spec §6.1): the
// engine's `NOTE_CODES` list is parsed out of
// `crates/epiora-core/src/notes/mod.rs` (the list has no wasm export
// — the source is the oracle, the same convention as
// `coverage.test.ts` parsing `FORMULAS`) and every code must have a
// note text, with no orphan texts.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';
import { NOTE_TEXT } from './notes';

const notesRs = readFileSync(
  resolve(process.cwd(), '../crates/epiora-core/src/notes/mod.rs'),
  'utf8',
);

function noteCodes(): string[] {
  const m = /pub const NOTE_CODES[^=]*=\s*&\[([\s\S]*?)\];/.exec(notesRs);
  if (m === null) throw new Error('NOTE_CODES const not found');
  return [...(m[1]?.matchAll(/"([^"]+)"/g) ?? [])].map((x) => x[1] ?? '');
}

describe('assumption note texts', () => {
  it('parses a nonempty NOTE_CODES list', () => {
    expect(noteCodes().length).toBeGreaterThan(0);
  });

  it('every_note_code_has_text', () => {
    const codes = noteCodes();
    const missing = codes.filter((c) => NOTE_TEXT[c] === undefined);
    expect(missing).toEqual([]);
    const orphans = Object.keys(NOTE_TEXT).filter(
      (id) => !codes.includes(id),
    );
    expect(orphans).toEqual([]);
    for (const [code, text] of Object.entries(NOTE_TEXT)) {
      expect(text.words, `${code}.words`).not.toBe('');
      expect(text.rule, `${code}.rule`).not.toBe('');
      expect(text.source, `${code}.source`).not.toBe('');
    }
  });
});
