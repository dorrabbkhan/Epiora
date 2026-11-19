// "Export to R" (v1.0 Task 17, spec §8.2): one stored run becomes
// `epiora-<procedure>-<receipt hash prefix>.zip` holding
// `analysis.R` (engine-generated verbatim corpus R), `data.csv` for
// dataset runs, and `receipt.json`. The zip is built here, inside the
// worker, with fflate — the main thread only turns the bytes into a
// download. `zipCalculator` is the calc worker's half: calculator
// exports are `analysis.R` + `receipt.json`, never a `data.csv`.

import { zipSync } from 'fflate';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { MeansOptions } from '@engine-types/MeansOptions';
import type { Receipt } from '@engine-types/Receipt';
import type { TablesOptions } from '@engine-types/TablesOptions';
import { engine } from './engines';
import { rpc } from './errors';
import { loadHandle } from './snapshots';
import * as runs from '../store/repo/runs';
import type { Store } from '../store/types';

/** What the download code consumes: suggested name + zip bytes
 *  (`ArrayBuffer`-backed — `new Blob` requires it). */
export interface RExportZip {
  name: string;
  bytes: Uint8Array<ArrayBuffer>;
}

/** The raw `exportR`/`exportRCalc` result crossing the wasm line. */
interface WasmExport {
  script: string;
  dataCsv: Uint8Array | null;
}

const encoder = new TextEncoder();

/** `epiora-<slug>-<first 12 hex>.zip` — the §8.2 filename shape. */
export function zipRExport(
  slug: string,
  receiptSha256: string,
  wasm: WasmExport,
  receipt: Receipt,
): RExportZip {
  const files: Record<string, Uint8Array> = {
    'analysis.R': encoder.encode(wasm.script),
    'receipt.json': encoder.encode(JSON.stringify(receipt, null, 2)),
  };
  if (wasm.dataCsv !== null) {
    files['data.csv'] = wasm.dataCsv;
  }
  return {
    name: `epiora-${slug}-${receiptSha256.slice(0, 12)}.zip`,
    bytes: zipSync(files) as Uint8Array<ArrayBuffer>,
  };
}

/** The export of one stored dataset run (spec §8.2). */
export async function exportRun(store: Store, runId: string): Promise<RExportZip> {
  const run = await runs.get(store, runId);
  if (run === null || run.deletedAt !== null) {
    throw rpc('not_found', `run ${runId} does not exist`);
  }
  const handle = await loadHandle(store, run.datasetId, run.datasetVersion);
  const eng = await engine();
  const options = JSON.parse(run.options) as
    | FreqOptions
    | TablesOptions
    | MeansOptions;
  const wasm = eng.exportR(handle, run.procedure, options, run.receiptHash) as WasmExport;
  return zipRExport(
    run.procedure,
    run.receiptHash,
    wasm,
    JSON.parse(run.receipt) as Receipt,
  );
}
