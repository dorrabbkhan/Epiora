// Lazy module loading (v1.1 plan Task 13): worker start must not
// fetch or instantiate epiora-legacy-wasm or epiora-lang-wasm — the
// same rule the importer already follows. `vi.mock` on the pkg entry
// points records the first dynamic `import()` (the mock factory runs
// at first import), so the assertions observe loading, not init.

import { describe, expect, it, vi } from 'vitest';

const imported: string[] = [];
vi.mock('@legacy/epiora_legacy_wasm', () => {
  imported.push('legacy');
  return { default: vi.fn().mockResolvedValue(undefined), legacyNew: vi.fn() };
});
vi.mock('@lang/epiora_lang_wasm', () => {
  imported.push('lang');
  return { default: vi.fn().mockResolvedValue(undefined) };
});
vi.mock('@import/epiora_import_wasm', () => {
  imported.push('import');
  return { default: vi.fn().mockResolvedValue(undefined) };
});

describe('lazy wasm modules', () => {
  it('lang_and_legacy_not_loaded_at_start', async () => {
    // Importing the API surface pulls in engines.ts (and the static
    // core-wasm import) but must not touch the lazy modules.
    await import('./api');
    expect(imported).toEqual([]);
  });

  it('first_call_loads_each_module', async () => {
    const { legacy, lang } = await import('./engines');
    await legacy();
    expect(imported).toEqual(['legacy']);
    await lang();
    expect(imported).toEqual(['legacy', 'lang']);
  });
});
