# Sync protocol prototype — does the §7.3 outline hold its invariants under offline concurrent edits and skewed clocks?

**Status:** go

## Question

Spec §7.3 sketches a sync protocol: changes carry (record id, field, value,
device, logical timestamp, base version); the server is the ordering
authority; a stale base version with a different value raises a conflict that
keeps both writes. v0.1 Task 9 asked whether that outline, simulated,
satisfies its stated invariants — no lost write, every conflict surfaced, all
devices converge — including under wall-clock skew and identical timestamps
(Review Focus 3).

## Method

A Rust library (`spikes/sync-proto/`) implements `Change`, `Device`, `Server`
and `PushResult` with the exact signatures the plan's Interfaces block
specifies. Devices keep a Lamport clock and a per-field "server version
seen"; the server assigns a monotonically increasing version per field and
keeps an ordered change log; `pull` replays log entries past the device's
high-water mark. `Server::apply` orders each batch by (lamport, device) and
raises a `Conflict` — keeping both the applied and the rejected change —
when an incoming `base_version` is older than the field's current version and
the value differs. Five simulation tests in `tests/simulation.rs` cover
sequential convergence, same-field and different-field concurrency, a
1-hour wall-clock skew with identical Lamport timestamps, and 1,000 seeds ×
200 random edit/push/pull/drop-connection steps across 3 devices with a
seeded `ChaCha8Rng` asserting all three invariants.

## Environment

- OS: Linux 7.0.0-1015-oem (local sandbox); toolchain: rustc/cargo 1.99.0
  (b940084d7 2026-09-28), edition 2021.
- Dependencies: `rand` 0.9.5, `rand_chacha` 0.9.0 (dev-dependencies only,
  lockfile-pinned in `spikes/sync-proto/Cargo.lock`).
- RNG: `ChaCha8Rng::seed_from_u64(seed)` with seeds 0–999 (recorded per test
  failure message).

## Measurements

- `cargo test --offline --manifest-path spikes/sync-proto/Cargo.toml`: **5
  passed, 0 failed, finished in 0.51 s** — the random-interleavings run covers
  1,000 seeds × 200 steps over 3 devices (records 0–4, fields f0–f2). Raw
  output: <https://github.com/dorrabbkhan/Epiora/pull/11#issuecomment-6508998989>
- `cargo clippy --offline --manifest-path spikes/sync-proto/Cargo.toml
  --all-targets -- -D warnings`: clean (0 warnings). Raw output:
  <https://github.com/dorrabbkhan/Epiora/pull/11#issuecomment-6508998989>
- `cargo fmt --manifest-path spikes/sync-proto/Cargo.toml --check`: clean
  (no output, exit 0). Raw output:
  <https://github.com/dorrabbkhan/Epiora/pull/11#issuecomment-6508998989>
- `CARGO_NET_OFFLINE=1 ./scripts/check.sh`: all gates pass ("ALL CHECKS
  PASSED"), including the cargo fmt/clippy/test block for
  `spikes/sync-proto` (run output: "test result: ok. 5 passed", 0.54 s).
  Raw output:
  <https://github.com/dorrabbkhan/Epiora/pull/11#issuecomment-6508998989>
- Skewed-clock check (Review Focus 3): with `wall_ms` 1 hour apart and
  identical Lamport timestamps, submitting the wall-later device's change
  first still orders by (lamport, device) — A's value wins, B's becomes a
  conflict — verified by `skewed_clocks_order_by_lamport`.
- Known limitation measured by the design, not hidden: a device that pushes,
  does not pull, then edits the same field again can raise a spurious
  conflict (its `base_version` is stale relative to the version its own just-
  accepted write created). False conflicts are safe — the write is surfaced,
  kept and resolvable — never silent data loss; the random test still
  converges after resolution.

## Decision

**go** — the §7.3 outline holds all three invariants under simulation: every
submitted change is reflected in the server log or recorded in a conflict
(exactly once), every conflict raised appears in `conflicts()`, and all
devices equal the server after a full sync plus resolution. Conflicts are
deterministic under wall-clock skew and identical timestamps because ordering
keys on (lamport, device), never on wall time.

## Consequences

- The custom protocol direction is viable; the v2.0 sync spec must add what
  this spike deliberately omits: **tombstones** for deletions, **form-version
  migration** with an audited mapping, and **persistence** (the spike keeps
  state in memory).
- v2.0 should also return per-change apply status (or reconcile versions in
  `push`) to remove the spurious-conflict limitation noted above, and define
  how conflicts reach the devices that did not push them (the spike surfaces
  them server-side only).
- For D8: nothing here rules out Electric — it remains a fallback pending the
  v2.0 spec, per spec §17.

## Licences

- `rand` 0.9.5 — MIT OR Apache-2.0 (dev-dependency, test-only).
- `rand_chacha` 0.9.0 — MIT OR Apache-2.0 (dev-dependency, test-only).
- Rust toolchain (rustc/cargo/clippy/rustfmt) 1.99.0 — MIT OR Apache-2.0.
- No runtime dependencies; the spike is self-contained Rust + std.
