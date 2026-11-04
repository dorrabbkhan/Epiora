//! Corpus support: loads and compares the R-generated fixtures of spec
//! §6. Enabled by the `corpus` feature; used by tests and corpus-runner
//! only, never a product importer.

pub mod compare;
pub mod dataset_json;
pub mod ei7;
pub mod run_case;

pub use dataset_json::{load_dataset, load_dataset_dir};
pub use run_case::{run_case, CaseOutput};
