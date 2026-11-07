// ESLint flat config (v0.3 spec §9.2): typescript-eslint recommended
// plus the component-set rule — JSX outside src/ui/components may not
// use raw interactive elements. The selector list lives in
// eslint.restrictions.json so lint-rules.test.ts checks the same rules.
import { readFileSync } from 'node:fs';
import globals from 'globals';
import tseslint from 'typescript-eslint';

const restrictions = JSON.parse(
  readFileSync(new URL('./eslint.restrictions.json', import.meta.url), 'utf8'),
);

export default tseslint.config(
  {
    ignores: [
      'dist/**',
      'node_modules/**',
      'playwright-report/**',
      'test-results/**',
      'lint-fixtures/**',
    ],
  },
  ...tseslint.configs.recommended,
  {
    files: ['**/*.{ts,tsx}'],
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    files: ['**/*.tsx'],
    ignores: ['src/ui/components/**'],
    rules: {
      'no-restricted-syntax': ['error', ...restrictions.jsx],
    },
  },
);
