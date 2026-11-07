// WCAG 2.1 contrast (spec §9.3): relative luminance ratios for the
// declared token pairs, and the required ratio per pair kind and theme.
import type { ThemeName } from './theme';

export const THEME_NAMES = ['light', 'dark', 'hc-light', 'hc-dark'] as const;

export type ContrastKind = 'text' | 'large-text' | 'ui';

function channel(c: number): number {
  const srgb = c / 255;
  return srgb <= 0.03928
    ? srgb / 12.92
    : Math.pow((srgb + 0.055) / 1.055, 2.4);
}

function luminance(hex: string): number {
  const m = /^#([0-9a-fA-F]{2})([0-9a-fA-F]{2})([0-9a-fA-F]{2})$/.exec(hex);
  if (!m) {
    throw new Error(`invalid hex colour: ${hex}`);
  }
  const r = channel(parseInt(m[1] as string, 16));
  const g = channel(parseInt(m[2] as string, 16));
  const b = channel(parseInt(m[3] as string, 16));
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrastRatio(fg: string, bg: string): number {
  const lf = luminance(fg);
  const lb = luminance(bg);
  const [hi, lo] = lf >= lb ? [lf, lb] : [lb, lf];
  return (hi + 0.05) / (lo + 0.05);
}

export function requiredRatio(kind: ContrastKind, theme: ThemeName): number {
  if (theme === 'hc-light' || theme === 'hc-dark') return 7;
  switch (kind) {
    case 'text':
      return 4.5;
    case 'large-text':
    case 'ui':
      return 3;
  }
}
