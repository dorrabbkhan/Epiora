// RPC errors (v0.3 spec §5.3, §11): every failure reaching the UI is a
// typed `RpcError` — `code` is the broad category, `detail` the inner
// stable code (`ImportError::code()`, `EngineError::code()`,
// `StoreError.code`, or a worker-only code) and `userMessage` the
// English string from `ui/strings.ts`. `toRpcError` is the one place
// Rust and store errors are mapped.
//
// Comlink drops custom properties off thrown Errors, so the worker
// throws the plain `WiredRpcError` object (structured-cloneable) and
// `client.ts` revives it into a real `RpcError` — the same pattern the
// store conformance harness uses for StoreError.

import { StoreError } from '../store/types';
import { t } from '../ui/strings';

export type RpcErrorCode =
  | 'ImportError'
  | 'StoreError'
  | 'EngineError'
  | 'HashMismatch'
  | 'Cancelled';

/** `ImportError::code()` (epiora-import), minus the transparent
 *  `Engine` wrap — those surface under ENGINE_ERROR_CODES. */
export const IMPORT_ERROR_CODES = [
  'empty_file',
  'no_columns',
  'encoding_invalid',
  'ragged_row',
  'too_many_columns',
  'too_many_rows',
  'unknown_column',
  'sheet_not_found',
  'spreadsheet',
  'invalid_settings',
  'duplicate_key',
  'missing_key',
  'invalid_key_type',
] as const;

/** `EngineError::code()` (epiora-core). */
export const ENGINE_ERROR_CODES = [
  'unknown_variable',
  'ambiguous_levels',
  'empty_dataset',
  'invalid_column',
  'type_mismatch',
  'invalid_options',
  'corpus_format',
  'snapshot_decode',
] as const;

/** `StoreError['code']` (app/src/store/types.ts). */
export const STORE_ERROR_CODES = [
  'migration_failed',
  'tx_aborted',
  'locked_elsewhere',
  'quota_exceeded',
  'keychain_unavailable',
  'key_invalid',
  'sql',
] as const;

/** Detail codes the worker itself raises. */
export const WORKER_ERROR_CODES = [
  'schema_change_unaccepted',
  'session_not_found',
  'snapshot_unavailable',
  'not_found',
  'hash_mismatch',
  'cancelled',
  'internal_error',
] as const;

const IMPORT = new Set<string>(IMPORT_ERROR_CODES);
const ENGINE = new Set<string>(ENGINE_ERROR_CODES);
const STORE = new Set<string>(STORE_ERROR_CODES);

const WORKER_CATEGORY: Record<(typeof WORKER_ERROR_CODES)[number], RpcErrorCode> = {
  schema_change_unaccepted: 'ImportError',
  session_not_found: 'ImportError',
  snapshot_unavailable: 'StoreError',
  not_found: 'StoreError',
  hash_mismatch: 'HashMismatch',
  cancelled: 'Cancelled',
  internal_error: 'StoreError',
};

/** The RPC category a detail code belongs to. Engine detail codes win
 *  over import's: an `ImportError::Engine(e)` keeps the engine code. */
export function categoryFor(detail: string): RpcErrorCode {
  if (detail in WORKER_CATEGORY) {
    return WORKER_CATEGORY[detail as (typeof WORKER_ERROR_CODES)[number]];
  }
  if (ENGINE.has(detail)) return 'EngineError';
  if (IMPORT.has(detail)) return 'ImportError';
  if (STORE.has(detail)) return 'StoreError';
  return 'StoreError';
}

function userMessageFor(detail: string): string {
  const messages = t.errors as Record<string, string | undefined>;
  return messages[detail] ?? messages.internal_error ?? 'Something went wrong.';
}

export class RpcError extends Error {
  readonly code: RpcErrorCode;
  /** The inner stable code (`ragged_row`, `quota_exceeded`, …). */
  readonly detail: string;
  readonly userMessage: string;

  constructor(code: RpcErrorCode, detail: string, message?: string) {
    super(message ?? userMessageFor(detail));
    this.name = 'RpcError';
    this.code = code;
    this.detail = detail;
    this.userMessage = userMessageFor(detail);
  }
}

/** Convenience constructor for the worker's own failures. */
export function rpc(detail: (typeof WORKER_ERROR_CODES)[number], message?: string): RpcError {
  return new RpcError(categoryFor(detail), detail, message);
}

export interface WiredRpcError {
  __epioraRpcError: true;
  code: RpcErrorCode;
  detail: string;
  userMessage: string;
  message: string;
}

export function isWiredRpcError(e: unknown): e is WiredRpcError {
  return (
    typeof e === 'object' && e !== null && (e as WiredRpcError).__epioraRpcError === true
  );
}

/** The stable `code` a wasm-bound Rust error carries (`js_error` sets
 *  it on the thrown Error). */
function rustCode(e: unknown): string | undefined {
  if (e instanceof Error) {
    const code = (e as unknown as { code?: unknown }).code;
    if (typeof code === 'string') return code;
  }
  return undefined;
}

/** The one place every backend error becomes an `RpcError`. */
export function toRpcError(e: unknown): RpcError {
  if (e instanceof RpcError) return e;
  if (isWiredRpcError(e)) return new RpcError(e.code, e.detail, e.message);
  if (e instanceof StoreError) return new RpcError('StoreError', e.code, e.message);
  const code = rustCode(e);
  if (code !== undefined) {
    return new RpcError(
      categoryFor(code),
      code,
      e instanceof Error ? e.message : undefined,
    );
  }
  if (e instanceof DOMException && e.name === 'QuotaExceededError') {
    return new RpcError('StoreError', 'quota_exceeded', e.message);
  }
  const message = e instanceof Error ? e.message : String(e);
  return new RpcError('StoreError', 'internal_error', message);
}

/** Converts `e` into the structured-cloneable wire shape and throws
 *  it; for use at the Comlink boundary inside the worker. */
export function throwWired(e: unknown): never {
  const r = toRpcError(e);
  throw {
    __epioraRpcError: true,
    code: r.code,
    detail: r.detail,
    userMessage: r.userMessage,
    message: r.message,
  } satisfies WiredRpcError;
}
