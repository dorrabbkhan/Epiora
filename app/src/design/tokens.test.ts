// Token file integrity (spec §9.1): every colour role declares all four
// themes, and the committed tokens.css is byte-identical to a fresh run
// of build-tokens.mjs.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';
import tokensJson from './tokens.json';
import { THEME_NAMES } from './contrast';

const here = join(process.cwd(), 'src', 'design');

describe('tokens.json', () => {
  it('every_color_role_has_four_themes', () => {
    for (const [role, themes] of Object.entries(tokensJson.color)) {
      for (const theme of THEME_NAMES) {
        const value = (themes as Record<string, string>)[theme];
        expect(value, `${role}.${theme}`).toMatch(/^#[0-9a-fA-F]{6}$/);
      }
    }
  });

  it('css_is_regenerated', () => {
    const out = join(tmpdir(), `epiora-tokens-${process.pid}-${Date.now()}.css`);
    execFileSync(process.execPath, [join(here, 'build-tokens.mjs'), out]);
    const regenerated = readFileSync(out, 'utf8');
    const committed = readFileSync(join(here, 'tokens.css'), 'utf8');
    expect(regenerated).toBe(committed);
  });
});
