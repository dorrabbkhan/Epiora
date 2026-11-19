#![cfg(feature = "corpus")]

//! R export tests (v1.0 Task 17, spec §8). The hostile-text dataset
//! must survive `dataset_csv`; the generated script carries its
//! provenance header; every corpus case produces a script and an
//! expected map; `random_numbers` is the one export the engine
//! refuses.

use epiora_core::corpus::{corpus_lines_with, load_datasets};
use epiora_core::export::corpus_datasets::hostile_text_dataset;
use epiora_core::export::corpus_export::case_export;
use epiora_core::export::{dataset_csv, export_r_calculator};
use serde_json::json;
use std::path::Path;

#[test]
fn csv_roundtrip_hostile_text() {
    let ds = hostile_text_dataset().expect("hostile dataset is well-formed");
    let csv = String::from_utf8(dataset_csv(&ds)).expect("dataset_csv is UTF-8");
    // The whole file is asserted so quoting, markers, ISO dates, ryu
    // reals and `\n` endings cannot drift silently (Review Focus 4).
    let expected = concat!(
        "count,real,text,flag,date\n",
        "1,0.1,\"a,b\",TRUE,1950-06-01\n",
        "\u{2205},\u{2205},\"say \"\"hi\"\"\",\u{2205},\u{2205}\n",
        "-7,-1.5e300,\"line\nbreak\",FALSE,2026-11-12\n",
        "0,2.5,caf\u{e9},TRUE,1970-01-01\n",
        "5,4.0,\u{65e5}\u{672c}\u{8a9e},FALSE,1970-01-02\n",
        "6,5.0,NA,TRUE,1969-12-31\n",
        "7,6.0,,FALSE,1970-04-11\n",
        "8,7.0,\u{2205}\u{2205},TRUE,1970-07-20\n",
        "9,8.0,\u{2205},FALSE,1970-10-28\n",
    );
    assert_eq!(csv, expected);
}

#[test]
fn script_header_names_receipt_and_licence() {
    let inputs = json!({
        "numerator": 10,
        "denominator": 45,
        "conf_level": 95,
        "multiplier": 100,
        "population_size": 1000000,
        "compare_with": 50
    });
    let sha = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let out = export_r_calculator("proportion", &inputs, sha).expect("export");
    assert!(out.data_csv.is_none(), "calculators embed their inputs");
    let s = &out.script;
    assert!(s.contains(&format!("# receipt_sha256: {sha}")), "{s}");
    assert!(s.contains("# engine:"), "{s}");
    assert!(s.contains("# R packages:"), "{s}");
    assert!(s.contains("# R code: MIT licence"), "{s}");
    // The verbatim snippets keep their own SPDX line.
    assert!(s.contains("# SPDX-License-Identifier: MIT"), "{s}");
    assert!(s.contains("proportion_write("), "{s}");
    assert!(s.contains("EPIORA"), "{s}");
}

#[test]
fn every_corpus_case_exports() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures");
    let datasets = load_datasets(&root).expect("datasets");
    let lines = corpus_lines_with(&root, &datasets).expect("corpus");
    assert!(!lines.is_empty());
    let mut problems = Vec::new();
    for line in &lines {
        match case_export(&line.fixture, &line.output.actual, &datasets) {
            Ok(export) => {
                if !export.script.contains("EPIORA") {
                    problems.push(format!("{}: script prints no EPIORA lines", line.rel_path));
                }
                if export.expected.is_empty() {
                    problems.push(format!("{}: empty expected map", line.rel_path));
                }
            }
            Err(e) => problems.push(format!("{}: {e}", line.rel_path)),
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn random_numbers_not_exportable() {
    let inputs = json!({
        "lower": 1,
        "upper": 100,
        "count": 5,
        "seed": 42
    });
    let err = export_r_calculator("random_numbers", &inputs, "00")
        .expect_err("random_numbers has no R reference");
    assert_eq!(err.code(), "export_not_available");
}
