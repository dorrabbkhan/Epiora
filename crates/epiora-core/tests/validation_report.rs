#![cfg(feature = "corpus")]

use epiora_core::corpus::report::build_rows;
use epiora_core::validation::REGISTRY;
use std::path::{Path, PathBuf};

fn fixtures_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

#[test]
fn validation_report_has_every_registry_row() {
    let rows = build_rows(&fixtures_root()).expect("build report rows");
    assert_eq!(
        rows.len(),
        REGISTRY.len(),
        "report rows must be one per REGISTRY entry"
    );
    for (row, coverage) in rows.iter().zip(REGISTRY.iter()) {
        assert_eq!(row.statistic, coverage.statistic);
        assert_eq!(row.class, coverage.class);
        assert!(
            row.worst_error <= coverage.class.tolerance(),
            "{}: worst observed error {} exceeds {:?} tolerance {}",
            coverage.statistic,
            row.worst_error,
            coverage.class,
            coverage.class.tolerance()
        );
    }
}
