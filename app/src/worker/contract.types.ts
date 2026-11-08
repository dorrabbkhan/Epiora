// Type-level contract between `DataApi` and the generated bindings
// (v0.3 plan Task 9): every payload that crosses the wasm boundary uses
// the ts-rs types verbatim — the assignments must hold in both
// directions or `tsc --noEmit` fails.

import type { DataApi, CommitResult, GridPage, ImportProgress, RunRecord } from './api';
import type { FreqOptions } from '@engine-types/FreqOptions';
import type { FreqResult } from '@engine-types/FreqResult';
import type { TablesOptions } from '@engine-types/TablesOptions';
import type { TablesResult } from '@engine-types/TablesResult';
import type { Receipt } from '@engine-types/Receipt';
import type { Inspection } from '@import-types/Inspection';
import type { ImportSettings } from '@import-types/ImportSettings';
import type { KeyCandidate } from '@import-types/KeyCandidate';
import type { DiffSummary } from '@import-types/DiffSummary';
import type { ValueType } from '@import-types/ValueType';

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2
  ? (<T>() => T extends B ? 1 : 2) extends <T>() => T extends A ? 1 : 2
    ? true
    : never
  : never;
type Assert<T extends true> = T;

type InspectionReturn = Awaited<ReturnType<DataApi['inspectFile']>>['inspection'];
type CandidatesReturn = Awaited<ReturnType<DataApi['inspectFile']>>['keyCandidates'];
type DiffReturn = Awaited<ReturnType<DataApi['diffAgainst']>>;
type SettingsParam = Parameters<DataApi['previewWithSettings']>[1];
type FreqOptionsParam = Parameters<DataApi['runFreq']>[3];
type TablesOptionsParam = Parameters<DataApi['runTables']>[3];

// Binding types flow through the API unchanged, in both directions.
export type Check_a = Assert<Equal<InspectionReturn, Inspection>>;
export type Check_b = Assert<Equal<CandidatesReturn, KeyCandidate[]>>;
export type Check_c = Assert<Equal<DiffReturn, DiffSummary>>;
export type Check_d = Assert<Equal<SettingsParam, ImportSettings>>;
export type Check_e = Assert<Equal<FreqOptionsParam, FreqOptions>>;
export type Check_f = Assert<Equal<TablesOptionsParam, TablesOptions>>;

// The API's own wire shapes match the plan (§5.3 surfaces, Task 9
// Interfaces block).
export type Check_g = Assert<Equal<ImportProgress['stage'], 'parse' | 'diff' | 'write' | 'snapshot'>>;
export type Check_h = Assert<Equal<CommitResult, { datasetId: string; version: number | null }>>;
export type Check_i = Assert<Equal<RunRecord['procedure'], 'freq' | 'tables'>>;
export type Check_j = Assert<Equal<RunRecord['options'], FreqOptions | TablesOptions>>;
export type Check_k = Assert<Equal<RunRecord['result'], FreqResult | TablesResult>>;
export type Check_l = Assert<Equal<RunRecord['receipt'], Receipt>>;
export type Check_m = Assert<
  Equal<GridPage['columns'][number], { name: string; type: ValueType; missing: number }>
>;
export type Check_n = Assert<Equal<GridPage['rows'][number][number], string | null>>;

export const TYPES_CHECKED = true;
