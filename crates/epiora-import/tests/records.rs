//! Record JSON tests (v0.3 spec §4.1/§4.3, plan Task 5): the canonical
//! per-record JSON (`{"<col>":{"type":…,"value":…}|<null>}`, keys
//! sorted by UTF-8 bytes) and `dataset_from_records` rebuilding a
//! dataset whose content hash matches.

use epiora_core::dataset::{Column, ColumnData, Dataset, ValueType};
use epiora_import::{dataset_from_records, record_json};

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

/// The golden dataset of plan v0.2 Task 4 (same as
/// `epiora-core/tests/snapshot.rs`): `A` Integer `[1, None, -3]`,
/// `B` Real `[0.5, -0.0, None]`, `C` Text `["é", None, ""]`,
/// `D` Boolean `[true, false, None]`, `E` Date `[0, -1, None]`.
fn golden_columns() -> Vec<Column> {
    vec![
        col("A", ColumnData::Integer(vec![Some(1), None, Some(-3)])),
        col("B", ColumnData::Real(vec![Some(0.5), Some(-0.0), None])),
        col(
            "C",
            ColumnData::Text(vec![Some("é".to_string()), None, Some(String::new())]),
        ),
        col(
            "D",
            ColumnData::Boolean(vec![Some(true), Some(false), None]),
        ),
        col("E", ColumnData::Date(vec![Some(0), Some(-1), None])),
    ]
}

#[test]
fn record_json_is_canonical() {
    let columns = golden_columns();
    assert_eq!(
        record_json(&columns, 0),
        "{\"A\":{\"type\":\"integer\",\"value\":1},\
         \"B\":{\"type\":\"real\",\"value\":0.5},\
         \"C\":{\"type\":\"text\",\"value\":\"é\"},\
         \"D\":{\"type\":\"boolean\",\"value\":true},\
         \"E\":{\"type\":\"date\",\"value\":0}}"
    );
    // Missing cells are explicit null; -0.0 normalises to 0.0.
    assert_eq!(
        record_json(&columns, 1),
        "{\"A\":null,\
         \"B\":{\"type\":\"real\",\"value\":0.0},\
         \"C\":null,\
         \"D\":{\"type\":\"boolean\",\"value\":false},\
         \"E\":{\"type\":\"date\",\"value\":-1}}"
    );
}

#[test]
fn record_json_out_of_range_is_empty_object() {
    let columns = golden_columns();
    assert_eq!(record_json(&columns, 3), "{}");
    assert_eq!(record_json(&columns, usize::MAX), "{}");
}

#[test]
fn dataset_from_records_roundtrips_hash() {
    let columns = golden_columns();
    let original = Dataset::new("golden", 7, columns.clone()).expect("golden dataset");
    let schema: Vec<(String, ValueType)> = columns
        .iter()
        .map(|c| (c.name.clone(), c.data.value_type()))
        .collect();
    let rows: Vec<String> = (0..3).map(|r| record_json(&columns, r)).collect();
    let row_refs: Vec<&str> = rows.iter().map(String::as_str).collect();
    let rebuilt = dataset_from_records("golden", 7, &schema, &row_refs).expect("rebuild");
    assert_eq!(rebuilt.columns(), original.columns());
    assert_eq!(rebuilt.content_hash_hex(), original.content_hash_hex());
}
