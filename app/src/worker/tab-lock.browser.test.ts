// Tab-lock take-over browser test (the 2026-11-22 fix): when "Use
// here" steals the lock, the old data worker must free the OPFS
// sahpool — closing the database alone is not enough, because the
// installed VFS keeps a sync access handle on every pool file — or
// the new tab's worker can never open the store and its projects
// list stays "Loading…" forever.
//
// This drives the real release path end to end: 'take-over' on the
// lock channel → markReleased → closeStore → pauseVfs → 'lost',
// while the holder's worker is never terminated.

import { afterEach, describe, expect, it } from 'vitest';
import {
  openHarness,
  spawn,
  terminateSpawned,
} from './integration-helpers';
import { takeOver } from './tab-lock';

afterEach(terminateSpawned);

describe('tab lock take-over', () => {
  it(
    'lock_takeover_releases_pool_for_next_worker',
    { timeout: 60_000 },
    async () => {
      const db = `t-${crypto.randomUUID()}`;
      const api = spawn(db);
      const project = await api.createProject('Oswego');

      const lost = new Promise<void>((resolve) => {
        // The channel name is tab-lock.ts's 'epiora-tab-lock'.
        const chan = new BroadcastChannel('epiora-tab-lock');
        chan.addEventListener('message', (e) => {
          const m = e.data as { worker?: string; type?: string };
          if (m.type === 'lost' && m.worker === db) {
            chan.close();
            resolve();
          }
        });
      });
      // What the LockScreen's "Use here" does: posts 'take-over' to
      // the holder (and steals the app lock, which no test worker
      // holds — their locks carry the per-db suffix).
      await takeOver();
      // 'lost' is posted only after closeStore finishes.
      await lost;

      // The holder is still alive — terminateSpawned only runs in
      // afterEach — so its release, not its death, freed the pool.
      const store = await openHarness(db);
      expect(
        await store.query('SELECT name FROM projects WHERE id = ?', [
          project.id,
        ]),
      ).toEqual([{ name: 'Oswego' }]);
      await store.close();
    },
  );
});
