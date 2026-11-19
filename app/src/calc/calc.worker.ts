/// <reference lib="webworker" />
//
// The calculator worker (v1.0 plan Task 14, spec §7.1): loads
// core-wasm only — no store, no import-wasm, no Web Lock — and serves
// the generic calculator screen. It stamps `RunContext.timestamp`
// itself (the engine reads no clock, E6) and draws seeds with
// `crypto.getRandomValues` so the receipt seals exactly the 53-bit
// seed the RNG consumed.

import { expose } from 'comlink';
import initEngine, {
  calcRegistry,
  calcRun,
  engineVersion,
  explainCalculator,
  exportRCalc,
} from '@engine/epiora_wasm';
import engineUrl from '@engine/epiora_wasm_bg.wasm?url';
import type { CalculatorDef } from '@engine-types/CalculatorDef';
import type { Explanation } from '@engine-types/Explanation';
import type { CalcOutput } from './api';
import { zipRExport } from '../worker/export';
import { throwWired } from '../worker/errors';
import type { CalcApi } from './api';

declare const self: DedicatedWorkerGlobalScope;

/** Cached engine + the version string for `RunContext` (the same SHA
 *  the engine stamps into every receipt). */
const ready = initEngine(engineUrl).then(() => engineVersion());

function cryptoSeed(): number {
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  let v = 0n;
  for (const b of bytes) v = (v << 8n) | BigInt(b);
  return Number(v % 2n ** 53n);
}

const api: CalcApi = {
  async listCalculators(): Promise<CalculatorDef[]> {
    await ready;
    return calcRegistry() as CalculatorDef[];
  },

  async run(id, inputs): Promise<CalcOutput> {
    const version = await ready;
    try {
      return calcRun(id, inputs, {
        app_version: 'epiora-app',
        engine_version: version,
        timestamp: new Date().toISOString(),
      }) as CalcOutput;
    } catch (err) {
      throwWired(err);
    }
  },

  async explain(id, inputs, statistic): Promise<Explanation> {
    await ready;
    try {
      return explainCalculator(id, inputs, statistic) as Explanation;
    } catch (err) {
      throwWired(err);
    }
  },

  async exportR(
    id,
    inputs,
    receipt,
  ): Promise<{ name: string; bytes: Uint8Array<ArrayBuffer> }> {
    await ready;
    try {
      const wasm = exportRCalc(id, inputs, receipt.receipt_sha256) as {
        script: string;
        dataCsv: Uint8Array | null;
      };
      return zipRExport(id, receipt.receipt_sha256, wasm, receipt);
    } catch (err) {
      throwWired(err);
    }
  },

  async newSeed(): Promise<number> {
    return cryptoSeed();
  },
};

expose(api);
export {};
void self;
