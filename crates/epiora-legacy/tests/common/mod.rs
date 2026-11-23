//! Shared helpers for the Task 7 tests: `LegacySource` impls over the
//! real backends (`epiora-mdb` for Access, `rusqlite` for SQLite —
//! both read the same sample files the golden's `mdb-*`/`sqlite3`
//! dumps read), a minimal EPTB encoder for synthetic sources, and the
//! structural `project.json`/dataset comparisons. Split across
//! `sources`/`encode`/`compare` to stay under the ~400-line cap.

#![allow(dead_code)]

pub mod compare;
pub mod encode;
pub mod sources;

// Each test binary uses a different subset — re-export flat, the
// unused-import warnings are per-binary noise.
#[allow(unused_imports)]
pub use compare::*;
#[allow(unused_imports)]
pub use encode::*;
#[allow(unused_imports)]
pub use sources::*;

use std::path::PathBuf;

/// The repository root (crate dir → crates → repo).
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

/// `fixtures/legacy/golden`.
pub fn golden_root() -> PathBuf {
    root().join("fixtures/legacy/golden")
}
