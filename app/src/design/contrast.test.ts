// Contrast tests for the design tokens (spec §9.3): every declared
// colour pair must meet its WCAG 2.1 ratio in all four themes.
import { describe, expect, it } from 'vitest';
import tokensJson from './tokens.json';
import { contrastRatio, requiredRatio, THEME_NAMES } from './contrast';
import type { ContrastKind } from './contrast';
import type { ThemeName } from './theme';

type TokensFile = {
  color: Record<string, Record<ThemeName, string>>;
  pairs: { fg: string; bg: string; kind: ContrastKind }[];
};

const tokens = tokensJson as unknown as TokensFile;

describe('contrastRatio', () => {
  it('known_ratios', () => {
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 5);
    expect(contrastRatio('#777777', '#ffffff')).toBeCloseTo(4.48, 2);
    expect(contrastRatio('#777777', '#ffffff')).toBeLessThan(4.5);
  });
});

describe('token pairs', () => {
  it('every_declared_pair_meets_its_ratio_in_every_theme', () => {
    const failures: string[] = [];
    for (const pair of tokens.pairs) {
      for (const theme of THEME_NAMES) {
        const fg = tokens.color[pair.fg]?.[theme];
        const bg = tokens.color[pair.bg]?.[theme];
        expect(fg, `color role ${pair.fg} is declared in tokens.color`).toBeDefined();
        expect(bg, `color role ${pair.bg} is declared in tokens.color`).toBeDefined();
        if (fg === undefined || bg === undefined) continue;
        const ratio = contrastRatio(fg, bg);
        const required = requiredRatio(pair.kind, theme);
        if (ratio < required - 1e-9) {
          failures.push(
            `${pair.fg} on ${pair.bg} in ${theme}: ${ratio.toFixed(2)} < ${required}`,
          );
        }
      }
    }
    expect(failures).toEqual([]);
  });
});
