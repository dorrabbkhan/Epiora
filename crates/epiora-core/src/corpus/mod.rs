//! Corpus support: loads and compares the R-generated fixtures of spec
//! §6. Enabled by the `corpus` feature; used by tests and corpus-runner
//! only, never a product importer.

pub mod compare;
pub mod dataset_json;
pub mod ei7;
pub mod explain_lines;
pub mod openepi;
pub mod parity_norm;
pub mod report;
pub mod run_all;
pub mod run_case;
pub mod run_means;
pub mod run_special;
pub mod synth;

pub use dataset_json::{load_dataset, load_dataset_dir};
pub use explain_lines::{command_line, explain_line, explain_plan};
pub use run_all::{
    corpus_lines, corpus_lines_with, dataset_files, load_datasets, CorpusLine, FAMILIES,
};
pub use run_case::{run_case, CaseOutput};
pub use run_special::run_special;
