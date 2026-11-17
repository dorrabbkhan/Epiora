#![cfg(feature = "corpus")]

use epiora_core::corpus::report::{build_report, build_rows};
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

#[test]
fn report_has_openepi_column() {
    let md = build_report(&fixtures_root(), "test-run", "not measured", "not measured")
        .expect("build report");
    assert!(
        md.contains("## OpenEpi check"),
        "report has no OpenEpi check section"
    );
    assert!(
        md.contains("| Calculator | OpenEpi |"),
        "OpenEpi section has no OpenEpi column"
    );
    // One row per captured calculator file.
    let dir = fixtures_root().join("openepi");
    let skip = [
        "cases.json",
        "labels.json",
        "package.json",
        "package-lock.json",
    ];
    for entry in std::fs::read_dir(&dir).expect("read openepi dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let name = path.file_name().expect("file name").to_string_lossy();
        if skip.contains(&name.as_ref()) {
            continue;
        }
        let stem = path.file_stem().expect("file stem").to_string_lossy();
        assert!(
            md.contains(&format!("| {stem} |")),
            "no OpenEpi row for {stem}"
        );
    }
}
