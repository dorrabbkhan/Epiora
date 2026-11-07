// The component-set and token-only rules really fire (Task 7 Step 5):
// both linters run on the intentional violations under lint-fixtures/
// (which the normal lint gates ignore) using the shared rule sources.
import { ESLint } from 'eslint';
import stylelint from 'stylelint';
import tseslint from 'eslint-ts6';
import { describe, expect, it } from 'vitest';
import restrictionsJson from './eslint.restrictions.json';
import stylelintRules from './stylelint.rules.json';

const app = process.cwd();

describe('lint rules', () => {
  it('lint_rules_catch_violations', async () => {
    const eslint = new ESLint({
      cwd: app,
      overrideConfigFile: true,
      overrideConfig: {
        files: ['**/*.tsx'],
        languageOptions: { parser: tseslint.parser },
        rules: {
          'no-restricted-syntax': ['error', ...restrictionsJson.jsx],
        },
      },
    });
    const jsxResults = await eslint.lintFiles([
      'lint-fixtures/component-violations.tsx',
    ]);
    const jsxMessages = jsxResults.flatMap((r) => r.messages);
    expect(jsxMessages.filter((m) => m.fatal)).toEqual([]);
    const fired = jsxMessages.filter(
      (m) => m.ruleId === 'no-restricted-syntax',
    );
    // button, input, select, textarea, a[onClick] — one each.
    expect(fired).toHaveLength(5);
    expect(
      fired.every((m) => m.message === 'use the Epiora component set'),
    ).toBe(true);

    const cssResult = await stylelint.lint({
      cwd: app,
      files: 'lint-fixtures/style-violations.module.css',
      config: { rules: stylelintRules },
    });
    const warnings = cssResult.results.flatMap((r) => r.warnings);
    const cssRules = new Set(warnings.map((w) => w.rule));
    expect(cssRules.has('color-no-hex')).toBe(true);
    expect(cssRules.has('function-disallowed-list')).toBe(true);
    expect(cssRules.has('declaration-property-unit-disallowed-list')).toBe(
      true,
    );
  });
});
