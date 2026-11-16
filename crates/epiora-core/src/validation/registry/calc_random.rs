//! `calc` registry coverage for `random_numbers` (v1.0 Task 9). It
//! has no R fixture (spec §5.2 — R cannot reproduce a ChaCha
//! stream); the coverage row points at the golden file
//! `crates/epiora-core/tests/golden/random_numbers.json`, whose
//! `expected.values` runs through `run_case`/`compare` like every
//! other corpus fixture (owner decision C2).

use super::super::ToleranceClass::ClosedForm as CF;
use super::super::{cov, Coverage};

const RANDOM_NUMBERS: &[&str] = &["../crates/epiora-core/tests/golden/random_numbers"];

pub(super) static ENTRIES: &[Coverage] = &[cov("calc.random_numbers.values[]", RANDOM_NUMBERS, CF)];
