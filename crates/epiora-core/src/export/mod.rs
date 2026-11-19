//! `export` — script and command generation from a run's canonical
//! options (spec §7.3 G4, §8.1). v1.0 Task 11 landed the Epi Info
//! Classic Analysis command line shown on every result card
//! (`command`); Task 17 adds the R script generator.
//!
//! A run exports as `analysis.R`: the provenance header, a verbatim
//! copy of the corpus reference R (`r_snippets`), an `epi_walk`
//! prelude that prints `EPIORA <statistic path> <value %.17g>` per
//! leaf, and — for dataset runs — `data.csv` written by
//! `csv::dataset_csv`. `export-corpus` (`corpus_export`, `corpus`
//! feature) turns every corpus fixture into one such script plus the
//! `expected.json` `scripts/r-export/compare.mjs` checks in CI.

pub mod command;
#[cfg(feature = "corpus")]
pub mod corpus_datasets;
#[cfg(feature = "corpus")]
pub mod corpus_export;
pub mod csv;
mod r_calc;
pub(crate) mod r_emit;
mod r_proc;
mod r_script;
pub(crate) mod r_snippets;

pub use command::epi_info_command;
pub use csv::{dataset_csv, missing_markers};
pub use r_script::{export_r_calculator, export_r_procedure, RExport};
