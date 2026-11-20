// `/validation` data (v1.0 plan Task 24, spec §7.4): the
// CI-generated `validation.json` — `validation-report --format json`
// uploaded by engine.yml and fetched into `public/` by app.yml — is
// the page's one source of measured numbers. A build that does not
// bundle it (local dev, this repo's unit tests) falls back to the
// wasm binding's `validationRegistry()` entries and the page says
// "observed errors not measured in this build". Nothing on the page
// may show a number that is not in the JSON — the loader therefore
// returns the parsed document unmodified, not a recomputed copy.

import { calcApi } from '../calc/client';

/** The `statistics[]` shape `validationRegistry()` returns — the
 *  registry entry only: no `references`, no `worst_error`. */
export interface RegistryEntry {
  id: string;
  method: string;
  class: 'closed_form' | 'iterative';
  cases: number;
}

/** One `statistics[]` entry of `validation-report --format json`. */
export interface ValidationEntry extends RegistryEntry {
  references?: { r?: string; ei7?: string; openepi?: string };
  /** Absent when the statistic was not measured; `null` when a leaf
   *  could not be compared numerically (the report's `inf`). */
  worst_error?: number | null;
}

/** The `validation-report --format json` document. */
export interface ValidationReport {
  engine_version: string;
  commit: string;
  run_url: string;
  statistics: ValidationEntry[];
}

/** What the page renders: `measured` distinguishes "the CI run said
 *  so" from "registry entries only" (the notice). */
export interface ValidationData {
  measured: boolean;
  engine_version?: string;
  commit?: string;
  run_url?: string;
  statistics: ValidationEntry[];
}

/** Reads `/validation.json`; `null` when the build did not bundle
 *  one or the body is not a report. The SPA fallback answers the path
 *  with `index.html` (200, text/html), so the content type is the
 *  real "is it bundled" test — `res.ok` alone is not. */
export async function fetchValidationJson(): Promise<ValidationReport | null> {
  try {
    const res = await fetch('/validation.json');
    if (
      !res.ok ||
      !(res.headers.get('content-type') ?? '').includes('json')
    ) {
      return null;
    }
    const doc: unknown = await res.json();
    if (
      typeof doc !== 'object' ||
      doc === null ||
      !Array.isArray((doc as ValidationReport).statistics)
    ) {
      return null;
    }
    return doc as ValidationReport;
  } catch {
    return null;
  }
}

/** The page's data: the measured report when bundled, the wasm
 *  registry entries otherwise. Both dependencies are injected so the
 *  unit test never spawns a worker or a real fetch. */
export async function loadValidationData(
  fetchJson: () => Promise<ValidationReport | null> = fetchValidationJson,
  registry: () => Promise<RegistryEntry[]> = () =>
    calcApi().validationRegistry(),
): Promise<ValidationData> {
  const report = await fetchJson();
  if (report !== null) {
    return { measured: true, ...report };
  }
  return { measured: false, statistics: await registry() };
}
