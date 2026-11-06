//! Record-key tests (v0.3 spec §4.4, plan Task 5): key columns render
//! record ids, duplicates and missing ids are refused with the 1-based
//! data lines, `row:<n>` positional ids, and the preview's
//! `key_candidates` report.

use epiora_core::dataset::{Column, ColumnData, ValueType};
use epiora_import::{key_candidates, record_ids, ImportError, RecordKey};

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

fn column_key(name: &str) -> RecordKey {
    RecordKey::Column {
        name: name.to_string(),
    }
}

#[test]
fn integer_text_date_keys() {
    let columns = vec![
        col(
            "ID",
            ColumnData::Integer(vec![Some(10), Some(20), Some(-7)]),
        ),
        col(
            "NM",
            ColumnData::Text(vec![
                Some("alpha".to_string()),
                Some("beta".to_string()),
                Some("gamma".to_string()),
            ]),
        ),
        col("DT", ColumnData::Date(vec![Some(0), Some(-1), Some(20089)])),
    ];
    assert_eq!(
        record_ids(&columns, &column_key("ID")).expect("integer key"),
        vec!["10", "20", "-7"]
    );
    assert_eq!(
        record_ids(&columns, &column_key("NM")).expect("text key"),
        vec!["alpha", "beta", "gamma"]
    );
    assert_eq!(
        record_ids(&columns, &column_key("DT")).expect("date key"),
        vec!["1970-01-01", "1969-12-31", "2025-01-01"]
    );
}

#[test]
fn real_key_is_invalid() {
    for (name, data, found) in [
        (
            "R",
            ColumnData::Real(vec![Some(1.5), Some(2.5)]),
            ValueType::Real,
        ),
        (
            "B",
            ColumnData::Boolean(vec![Some(true), Some(false)]),
            ValueType::Boolean,
        ),
    ] {
        let columns = vec![col(name, data)];
        match record_ids(&columns, &column_key(name)) {
            Err(ImportError::InvalidKeyType { column, found: f }) => {
                assert_eq!(column, name);
                assert_eq!(f, found);
            }
            other => panic!("expected invalid_key_type, got {other:?}"),
        }
    }
}

#[test]
fn duplicate_key_names_both_lines() {
    let columns = vec![col(
        "ID",
        ColumnData::Integer(vec![Some(5), Some(2), Some(5)]),
    )];
    match record_ids(&columns, &column_key("ID")) {
        Err(ImportError::DuplicateKey {
            key,
            first_line,
            line,
        }) => {
            assert_eq!(key, "5");
            assert_eq!(first_line, 1);
            assert_eq!(line, 3);
        }
        other => panic!("expected duplicate_key, got {other:?}"),
    }
}

#[test]
fn missing_key_names_line() {
    let columns = vec![col("ID", ColumnData::Integer(vec![Some(1), None, Some(3)]))];
    match record_ids(&columns, &column_key("ID")) {
        Err(ImportError::MissingKey { line }) => assert_eq!(line, 2),
        other => panic!("expected missing_key, got {other:?}"),
    }
}

#[test]
fn row_position_ids() {
    let columns = vec![col(
        "X",
        ColumnData::Text(vec![Some("a".to_string()), None, Some("c".to_string())]),
    )];
    assert_eq!(
        record_ids(&columns, &RecordKey::RowPosition).expect("positional"),
        vec!["row:1", "row:2", "row:3"]
    );
}

#[test]
fn candidates_flag_duplicates_and_missing() {
    let columns = vec![
        col("U", ColumnData::Integer(vec![Some(1), Some(2), Some(3)])),
        col(
            "D",
            ColumnData::Text(vec![
                Some("a".to_string()),
                Some("a".to_string()),
                Some("b".to_string()),
            ]),
        ),
        col("M", ColumnData::Date(vec![Some(0), None, Some(5)])),
        col(
            "B",
            ColumnData::Boolean(vec![Some(true), Some(false), Some(true)]),
        ),
    ];
    let candidates = key_candidates(&columns);
    assert_eq!(candidates.len(), 4);
    let by_name = |name: &str| {
        candidates
            .iter()
            .find(|c| c.column == name)
            .unwrap_or_else(|| panic!("no candidate for {name}"))
    };
    let u = by_name("U");
    assert!(u.usable && u.duplicates == 0 && u.missing == 0, "{u:?}");
    let d = by_name("D");
    assert!(!d.usable && d.duplicates == 1 && d.missing == 0, "{d:?}");
    let m = by_name("M");
    assert!(!m.usable && m.duplicates == 0 && m.missing == 1, "{m:?}");
    let b = by_name("B");
    assert!(!b.usable && b.duplicates == 1 && b.missing == 0, "{b:?}");
}
