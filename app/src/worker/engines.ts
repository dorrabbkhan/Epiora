// Wasm engines (v0.3 spec §5.2): core (epiora-wasm) initialises at
// worker start; the importer (epiora-import-wasm) is fetched lazily on
// the first import call so cold loads don't pay for it. The legacy and
// language bindings (epiora-legacy-wasm, epiora-lang-wasm — v1.1 Task
// 13) lazy-load the same way on the first legacy/session call. All are
// cached singletons — a failed init clears the cache so a retry can
// succeed (mirroring opfs.ts's pool handling).

import initEngine from '@engine/epiora_wasm';
import * as engineWasm from '@engine/epiora_wasm';
import engineWasmUrl from '@engine/epiora_wasm_bg.wasm?url';
import importWasmUrl from '@import/epiora_import_wasm_bg.wasm?url';
import legacyWasmUrl from '@legacy/epiora_legacy_wasm_bg.wasm?url';
import langWasmUrl from '@lang/epiora_lang_wasm_bg.wasm?url';

export type Engine = typeof engineWasm;
export type Importer = typeof import('@import/epiora_import_wasm');
export type Legacy = typeof import('@legacy/epiora_legacy_wasm');
export type Lang = typeof import('@lang/epiora_lang_wasm');

let enginePromise: Promise<Engine> | null = null;
let importerPromise: Promise<Importer> | null = null;
let legacyPromise: Promise<Legacy> | null = null;
let langPromise: Promise<Lang> | null = null;

/** The initialised epiora-wasm module (spec §5.1 exports). */
export function engine(): Promise<Engine> {
  enginePromise ??= initEngine(engineWasmUrl)
    .then(() => engineWasm)
    .catch((e: unknown) => {
      enginePromise = null;
      throw e;
    });
  return enginePromise;
}

/** The initialised epiora-import-wasm module, loaded on first use. */
export function importer(): Promise<Importer> {
  importerPromise ??= import('@import/epiora_import_wasm')
    .then(async (m) => {
      await m.default(importWasmUrl);
      return m;
    })
    .catch((e: unknown) => {
      importerPromise = null;
      throw e;
    });
  return importerPromise;
}

/** The initialised epiora-legacy-wasm module, loaded on first use. */
export function legacy(): Promise<Legacy> {
  legacyPromise ??= import('@legacy/epiora_legacy_wasm')
    .then(async (m) => {
      await m.default(legacyWasmUrl);
      return m;
    })
    .catch((e: unknown) => {
      legacyPromise = null;
      throw e;
    });
  return legacyPromise;
}

/** The initialised epiora-lang-wasm module, loaded on first use. */
export function lang(): Promise<Lang> {
  langPromise ??= import('@lang/epiora_lang_wasm')
    .then(async (m) => {
      await m.default(langWasmUrl);
      return m;
    })
    .catch((e: unknown) => {
      langPromise = null;
      throw e;
    });
  return langPromise;
}
