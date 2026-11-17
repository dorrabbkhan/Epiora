//! `export` — script and command generation from a run's canonical
//! options (spec §7.3 G4, §8.1). v1.0 Task 11 lands the Epi Info
//! Classic Analysis command line shown on every result card; Task
//! 17 adds the R script generator.

pub mod command;

pub use command::epi_info_command;
