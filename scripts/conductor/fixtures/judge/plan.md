# Fixture plan for judge tests — not a real plan.

## Task Index

| # | Task | Depends on | Gate | Conductor |
|---|------|------------|------|-----------|
| 1 | plain task | — | — | Opus |
| 2 | gated task | 1 | OWNER REVIEW (process) | Opus |
| 3 | human task | 2 | gate (human step) | Opus |

### Task 1: Plain
**Files:**
- Create: `src/foo.ts`, `foo.test.ts`, `scripts/conductor/x.sh`, `src/glob-?.ts`
- Modify: `docs/decisions.md`, `crates/epiora-core/src/stats.rs`, `docs/spikes/wasm-mdb.md`

### Task 2: Gated
**Files:**
- Create: `src/bar.ts`

### Task 3: Human step
**Files:**
- Create: `src/baz.ts`
