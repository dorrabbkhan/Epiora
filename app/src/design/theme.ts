// Theme switching (spec §9.1): 'system' follows prefers-color-scheme
// and prefers-contrast; the other four set data-theme on <html>.
export type ThemeName = 'light' | 'dark' | 'hc-light' | 'hc-dark';
export type Theme = 'system' | ThemeName;

export const THEME_STORAGE_KEY = 'epiora.theme';

export function applyTheme(theme: Theme): void {
  if (theme === 'system') {
    document.documentElement.removeAttribute('data-theme');
  } else {
    document.documentElement.setAttribute('data-theme', theme);
  }
}
