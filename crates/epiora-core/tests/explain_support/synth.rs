//! Synthetic dataset and options builders for the explain tests
//! (v1.0 Task 10): table fixtures carry no dataset, so their
//! declared counts are expanded into a record-level `Dataset` the
//! procedures can run, and the `freq`/`means`/`tables` options the
//! explain calls share are built once here. Since v1.0 Task 11 the
//! construction lives in `epiora_core::corpus::synth` — the same
//! code `corpus-runner` and the wasm parity run use — and these are
//! panicking wrappers over it (tests may panic).

use epiora_core::corpus::synth as core_synth;
use epiora_core::dataset::Dataset;
use serde_json::Value;
use std::collections::BTreeMap;

/// The `(dataset, options)` an explain run needs for a `tables`
/// fixture — the corpus dataset when declared, else a synthetic one
/// expanded from the fixture's counts.
pub fn tables_case(fixture: &Value, datasets: &BTreeMap<String, Dataset>) -> (Dataset, Value) {
    core_synth::tables_case(fixture, datasets).expect("tables_case")
}

/// A synthetic `ROW`/`COL` dataset for an `rxc` table fixture.
pub fn rxc_case(fixture: &Value) -> (Dataset, Value) {
    core_synth::rxc_case(fixture).expect("rxc_case")
}

/// The `freq` options of a fixture — the same options both
/// `explain_for` and `run_for` build.
pub fn freq_opts(fixture: &Value) -> Value {
    core_synth::freq_opts(fixture).expect("freq_opts")
}

/// The `means` options of a fixture.
pub fn means_opts(fixture: &Value) -> Value {
    core_synth::means_opts(fixture).expect("means_opts")
}

/// The corpus dataset a `freq`/`means` fixture names.
pub fn fixture_dataset<'a>(
    fixture: &Value,
    datasets: &'a BTreeMap<String, Dataset>,
) -> &'a Dataset {
    core_synth::fixture_dataset(fixture, datasets).expect("fixture_dataset")
}
