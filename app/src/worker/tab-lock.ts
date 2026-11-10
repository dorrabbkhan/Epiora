// Tab lock (v0.3 spec §7.1, plan Task 13): the OPFS sahpool allows a
// single store connection, so the data worker holds a Web Lock for
// its lifetime. A second tab's worker gets 'held-elsewhere' and the
// UI shows the LockScreen; "Use here" steals the lock — the old
// worker's lock releases, it closes its store, and its tab locks.
//
// The lock name is the plan's 'epiora-store' for the app database;
// other (test) databases get a per-db suffix so concurrent test
// workers on one page do not contend.
//
// State transitions are also announced on a BroadcastChannel so the
// owning tab's main thread can swap to the LockScreen: 'held-elsewhere'
// at startup, 'lost' after a take-over. A window that mounts after the
// announcement posts 'ui-ready' and the worker re-posts its state.

const APP_LOCK = 'epiora-store';
const CHANNEL = 'epiora-tab-lock';

export type StoreLockResult = 'acquired' | 'held-elsewhere';
type LockState = StoreLockResult | 'pending' | 'released';

let state: LockState = 'pending';
let acquireP: Promise<StoreLockResult> | null = null;
let channel: BroadcastChannel | null = null;
const releasedWaiters: (() => void)[] = [];

function selfName(): string {
  const name = (globalThis as { name?: unknown }).name;
  return typeof name === 'string' && name !== '' ? name : 'epiora';
}

/** The lock for this worker's database: 'epiora-store' for the app
 *  db, 'epiora-store:<db>' otherwise (test workers each name their
 *  own database via the worker `name` option). */
function lockName(): string {
  const db = selfName().split(':')[0]!;
  return db === 'epiora' ? APP_LOCK : `${APP_LOCK}:${db}`;
}

function chan(): BroadcastChannel | null {
  if (typeof BroadcastChannel === 'undefined') return null;
  channel ??= new BroadcastChannel(CHANNEL);
  return channel;
}

function post(type: 'held-elsewhere' | 'lost'): void {
  chan()?.postMessage({ worker: selfName(), type });
}

function markReleased(): void {
  if (state !== 'acquired') return;
  state = 'released';
  while (releasedWaiters.length > 0) releasedWaiters.pop()!();
}

function onChannelMessage(e: MessageEvent): void {
  const type = (e.data as { type?: unknown }).type;
  if (type === 'take-over') {
    markReleased();
  } else if (type === 'ui-ready' && state !== 'acquired' && state !== 'pending') {
    post(state === 'released' ? 'lost' : 'held-elsewhere');
  }
}

/**
 * Requests the store lock (`ifAvailable`), holding it for the
 * worker's lifetime once granted. Resolves 'held-elsewhere' when
 * another tab already holds it; 'acquired' also when Web Locks are
 * unavailable (the sahpool's own errors then apply).
 */
export function acquireStoreLock(): Promise<StoreLockResult> {
  acquireP ??= (async () => {
    chan()?.addEventListener('message', onChannelMessage);
    if (typeof navigator === 'undefined' || !('locks' in navigator)) {
      state = 'acquired';
      return 'acquired' as const;
    }
    return await new Promise<StoreLockResult>((resolve) => {
      navigator.locks
        .request(lockName(), { ifAvailable: true }, (lock) => {
          if (lock === null) {
            state = 'held-elsewhere';
            resolve('held-elsewhere');
            post('held-elsewhere');
            return Promise.resolve();
          }
          state = 'acquired';
          resolve('acquired');
          // Never resolves: the lock stays held until it is stolen
          // or the worker is terminated.
          return new Promise<never>(() => {});
        })
        .then(() => {
          markReleased();
        })
        .catch(() => {
          markReleased();
        });
    });
  })();
  return acquireP;
}

/** Resolves once a held lock is released (stolen by another tab). */
export function storeLockReleased(): Promise<void> {
  if (state === 'released') return Promise.resolve();
  return new Promise((r) => releasedWaiters.push(r));
}

/** The current lock state; `openStore` refuses to run unlocked. */
export function storeLockState(): LockState {
  return state;
}

/** Announces on the channel that this worker's lock was lost (after
 *  its store is closed); the owning tab shows the LockScreen. */
export function reportLockLost(): void {
  post('lost');
}

/**
 * 'Use here' (LockScreen): tells the current holder to let go, then
 * steals the lock and releases it so the caller's fresh worker can
 * acquire it on the next load.
 */
export async function takeOver(): Promise<void> {
  chan()?.postMessage({ type: 'take-over' });
  if (typeof navigator === 'undefined' || !('locks' in navigator)) return;
  await navigator.locks.request(APP_LOCK, { steal: true }, () =>
    Promise.resolve(),
  );
}
