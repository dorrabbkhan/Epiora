#![cfg(feature = "corpus")]

use epiora_core::corpus::{load_dataset, load_dataset_dir};
use epiora_core::dataset::{ColumnData, ValueType};
use std::path::{Path, PathBuf};

fn data_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/data")
}

#[test]
fn loads_every_corpus_dataset() {
    let dir = data_dir();
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("fixtures/data")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".dataset.json"))
        })
        .collect();
    files.sort();
    assert!(!files.is_empty(), "no corpus datasets in {dir:?}");

    for path in &files {
        let json = std::fs::read_to_string(path).expect("read dataset");
        load_dataset(&json).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    }

    let sets = load_dataset_dir(&dir).expect("load_dataset_dir");
    assert_eq!(sets.len(), files.len());
    for path in &files {
        let json = std::fs::read_to_string(path).expect("read dataset");
        let single = load_dataset(&json).expect("load_dataset");
        assert!(
            sets.contains_key(single.id()),
            "{} missing from map",
            single.id()
        );
    }

    let oswego = sets.get("oswego").expect("oswego dataset");
    assert_eq!(oswego.row_count(), 75);
    assert_eq!(oswego.columns().len(), 22);
}

#[test]
fn malformed_json_is_corpus_format() {
    let err = load_dataset("{ not json").unwrap_err();
    assert_eq!(err.code(), "corpus_format");

    let wrong_type = r#"{"id":"d","version":1,"columns":[
        {"name":"A","type":"integer","values":["not an integer"]}]}"#;
    let err = load_dataset(wrong_type).unwrap_err();
    assert_eq!(err.code(), "corpus_format");
}

#[test]
fn date_parses_iso() {
    let json = r#"{"id":"d","version":1,"columns":[
        {"name":"D","type":"date","values":["1970-01-02",null,"1969-12-31","2000-02-29"]}]}"#;
    let ds = load_dataset(json).expect("dataset");
    match &ds.column("D").expect("column").data {
        ColumnData::Date(days) => {
            assert_eq!(days, &vec![Some(1), None, Some(-1), Some(11016)]);
        }
        other => panic!("expected date column, got {:?}", other.value_type()),
    }
    assert_eq!(
        ds.column("D").expect("column").data.value_type(),
        ValueType::Date
    );
}

#[test]
fn bad_date_is_corpus_format() {
    for bad in ["not a date", "2026-13-01", "2026-02-30", "1970-1-2"] {
        let json = format!(
            r#"{{"id":"d","version":1,"columns":[{{"name":"D","type":"date","values":["{bad}"]}}]}}"#
        );
        let err = load_dataset(&json).unwrap_err();
        assert_eq!(err.code(), "corpus_format", "date {bad:?}");
    }
}
