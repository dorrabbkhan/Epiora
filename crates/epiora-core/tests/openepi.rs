#![cfg(feature = "corpus")]

use epiora_core::corpus::openepi::{compare_openepi, within_display};
use std::path::{Path, PathBuf};

fn openepi_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/openepi")
}

#[test]
fn openepi_matches_at_display() {
    let report = compare_openepi(&openepi_dir()).expect("compare openepi");
    assert!(report.checked > 0, "no mapped OpenEpi values compared");
    assert!(
        report.mismatches.is_empty(),
        "{} OpenEpi mismatch(es):\n{}",
        report.mismatches.len(),
        report
            .mismatches
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn rule_is_half_unit_in_last_decimal() {
    // decimals 0: tol 0.5; decimals 2: 0.005; decimals 4: 0.00005.
    assert!(within_display(1.4, 1.0, 0));
    assert!(!within_display(1.6, 1.0, 0));
    assert!(within_display(1.2349, 1.23, 2));
    assert!(!within_display(1.2351, 1.23, 2));
    assert!(within_display(0.11876, 0.1188, 4));
    assert!(!within_display(0.11874, 0.1188, 4));
    // Negative decimals (a rate restated to a coarser per-unit scale)
    // loosen the bound: d = -1 gives 5.
    assert!(within_display(14.0, 10.0, -1));
    assert!(!within_display(16.0, 10.0, -1));
}
