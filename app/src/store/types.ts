// Store contract shared by every backend (v0.3 spec §4). All SQL flows
// through these two methods; repositories only ever see a `Tx`.

export type SqlValue = null | number | bigint | string | Uint8Array;
export type Row = Record<string, SqlValue>;

export interface Tx {
  exec(sql: string, params?: SqlValue[]): Promise<void>;
  query(sql: string, params?: SqlValue[]): Promise<Row[]>;
}

export interface Store extends Tx {
  /**
   * Commits when `fn` resolves and rolls back when it throws (the error
   * is re-thrown). Integers above 2^53 are returned as `bigint`.
   */
  transaction<T>(fn: (tx: Tx) => Promise<T>): Promise<T>;
  close(): Promise<void>;
  /**
   * Test hook (conformance suite): the next mutating statement fails
   * with `quota_exceeded`. Present on every backend; only used by tests.
   */
  __simulateQuotaOnNextWrite?(): void;
}

export class StoreError extends Error {
  readonly code:
    | 'migration_failed'
    | 'tx_aborted'
    | 'locked_elsewhere'
    | 'quota_exceeded'
    | 'keychain_unavailable'
    | 'key_invalid'
    | 'sql';

  constructor(
    code: StoreError['code'],
    message: string,
    options?: { cause?: unknown },
  ) {
    super(message, options);
    this.name = 'StoreError';
    this.code = code;
  }
}
