import type { CalculatorDef } from '@engine-types/CalculatorDef';
import type { Explanation } from '@engine-types/Explanation';
import type { Note } from '@engine-types/Note';
import type { Receipt } from '@engine-types/Receipt';
import type { RegistryEntry } from '../validation/data';

/**
 * What `calcRun` returns over the wasm boundary: the generic result
 * tree the screen flattens into labelled statistic rows, the v3
 * receipt, and the assumption notes the run raised.
 */
export interface CalcOutput {
  result: unknown;
  receipt: Receipt;
  notes: Note[];
}

/**
 * The calculator worker's surface (v1.0 plan Task 14, spec §7.1).
 * Store-less: runs in `calc.worker.ts` with core-wasm only — no
 * store, no import-wasm, no Web Lock — so calculator links work in any
 * number of tabs alongside the single-store app.
 *
 * `newSeed` lives on the worker (not the page) so the 53-bit seed goes
 * straight from `crypto.getRandomValues` into the run inputs and the
 * receipt without rounding on the main thread.
 */
export interface CalcApi {
  listCalculators(): Promise<CalculatorDef[]>;
  /**
   * The `validation::REGISTRY` entries as `statistics[]`-shaped
   * objects (v1.0 plan Task 24, spec §7.4): the `/validation`
   * page's fallback when the build bundled no `validation.json`.
   */
  validationRegistry(): Promise<RegistryEntry[]>;
  run(id: string, inputs: Record<string, unknown>): Promise<CalcOutput>;
  explain(
    id: string,
    inputs: Record<string, unknown>,
    statistic: string,
  ): Promise<Explanation>;
  /**
   * "Export to R" (v1.0 Task 17, spec §8.2): `analysis.R` +
   * `receipt.json` zipped in the worker (calculator exports have no
   * `data.csv`). `random_numbers` rejects with
   * `export_not_available` — no R reference exists for it.
   */
  exportR(
    id: string,
    inputs: Record<string, unknown>,
    receipt: Receipt,
  ): Promise<{ name: string; bytes: Uint8Array<ArrayBuffer> }>;
  newSeed(): Promise<number>;
}
