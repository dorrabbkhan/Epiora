// Shared axe harness (Task 7): runs axe on the current page and fails
// listing every violation id and target. There is deliberately no
// allow-list parameter (spec §9.3; plan Review Focus 5).
import { AxeBuilder } from '@axe-core/playwright';
import type { Page } from '@playwright/test';

export async function expectNoA11yViolations(
  page: Page,
  context: string,
): Promise<void> {
  const results = await new AxeBuilder({ page }).analyze();
  if (results.violations.length === 0) return;
  const lines = results.violations.map(
    (v) =>
      `${v.id} (${v.impact ?? 'unknown'}): ` +
      v.nodes.map((n) => n.target.join(' ')).join('; '),
  );
  throw new Error(
    `axe violations in ${context}:\n${lines.join('\n')}`,
  );
}
