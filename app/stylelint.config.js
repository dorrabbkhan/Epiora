// Stylelint (v0.3 spec §9.2): CSS modules reference tokens only — no
// raw colours, and no raw px on spacing, radius or font-size. The rules
// live in stylelint.rules.json so lint-rules.test.ts checks the same
// rules. Generated token output and the intentional-violation fixtures
// are exempt.
import { readFileSync } from 'node:fs';

const rules = JSON.parse(
  readFileSync(new URL('./stylelint.rules.json', import.meta.url), 'utf8'),
);

/** @type {import('stylelint').Config} */
export default {
  ignoreFiles: [
    'src/design/**',
    'lint-fixtures/**',
    'dist/**',
    'node_modules/**',
  ],
  rules,
};
