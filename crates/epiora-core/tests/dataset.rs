use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::EngineError;

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

#[test]
fn rejects_unequal_lengths() {
    let err = Dataset::new(
        "d",
        1,
        vec![
            col("A", ColumnData::Integer(vec![Some(1), Some(2)])),
            col("B", ColumnData::Integer(vec![Some(1)])),
        ],
    )
    .unwrap_err();
    assert_eq!(err.code(), "invalid_column");
}

#[test]
fn rejects_duplicate_name() {
    let err = Dataset::new(
        "d",
        1,
        vec![
            col("A", ColumnData::Integer(vec![Some(1)])),
            col("A", ColumnData::Boolean(vec![Some(true)])),
        ],
    )
    .unwrap_err();
    assert_eq!(err.code(), "invalid_column");
}

#[test]
fn rejects_empty_name() {
    let err = Dataset::new("d", 1, vec![col("", ColumnData::Integer(vec![Some(1)]))]).unwrap_err();
    assert_eq!(err.code(), "invalid_column");
}

#[test]
fn rejects_nan_and_infinity() {
    for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let err =
            Dataset::new("d", 1, vec![col("R", ColumnData::Real(vec![Some(bad)]))]).unwrap_err();
        assert_eq!(err.code(), "invalid_column");
    }
}

#[test]
fn zero_columns_is_empty_dataset() {
    let err = Dataset::new("d", 1, vec![]).unwrap_err();
    assert_eq!(err, EngineError::EmptyDataset);
}

#[test]
fn zero_rows_is_valid() {
    let ds = Dataset::new(
        "d",
        1,
        vec![
            col("A", ColumnData::Integer(vec![])),
            col("B", ColumnData::Text(vec![])),
        ],
    )
    .expect("zero rows is valid");
    assert_eq!(ds.row_count(), 0);
    assert_eq!(ds.columns().len(), 2);
}

#[test]
fn unknown_column_errors() {
    let ds =
        Dataset::new("d", 1, vec![col("A", ColumnData::Integer(vec![Some(1)]))]).expect("dataset");
    let err = ds.column("NOPE").unwrap_err();
    assert_eq!(
        err,
        EngineError::UnknownVariable {
            name: "NOPE".to_string()
        }
    );
}

/// The golden dataset of plan v0.2 Task 4: `A` Integer `[1, None, -3]`,
/// `B` Real `[0.5, -0.0, None]`, `C` Text `["é", None, ""]`,
/// `D` Boolean `[true, false, None]`, `E` Date `[0, -1, None]`.
fn golden() -> Dataset {
    Dataset::new(
        "golden",
        7,
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
        ],
    )
    .expect("golden dataset")
}

#[test]
fn golden_canonical_bytes() {
    // Written out by hand from spec §3.3: format byte 0x01, column count
    // (u32 LE), then per column name (u32 LE length + UTF-8), type tag
    // (1=Integer, 2=Real, 3=Text, 4=Boolean, 5=Date), row count (u64 LE),
    // then a presence byte 0/1 per value followed by the value (i64 LE;
    // f64 bits LE with -0.0 normalised to 0.0; u32 LE length + UTF-8;
    // u8 0/1; i32 LE).
    #[rustfmt::skip]
    let expected: Vec<u8> = vec![
        0x01, // format
        0x05, 0x00, 0x00, 0x00, // 5 columns
        // A: integer [1, None, -3]
        0x01, 0x00, 0x00, 0x00, 0x41, // name "A"
        0x01, // tag integer
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 3 rows
        0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // 1
        0x00, // missing
        0x01, 0xfd, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, // -3
        // B: real [0.5, -0.0, None]
        0x01, 0x00, 0x00, 0x00, 0x42, // name "B"
        0x02, // tag real
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x3f, // 0.5
        0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // -0.0 → 0.0
        0x00,
        // C: text ["é", None, ""]
        0x01, 0x00, 0x00, 0x00, 0x43, // name "C"
        0x03, // tag text
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x02, 0x00, 0x00, 0x00, 0xc3, 0xa9, // "é"
        0x00,
        0x01, 0x00, 0x00, 0x00, 0x00, // ""
        // D: boolean [true, false, None]
        0x01, 0x00, 0x00, 0x00, 0x44, // name "D"
        0x04, // tag boolean
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x01, // true
        0x01, 0x00, // false
        0x00,
        // E: date [0, -1, None]
        0x01, 0x00, 0x00, 0x00, 0x45, // name "E"
        0x05, // tag date
        0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00, 0x00, // 0
        0x01, 0xff, 0xff, 0xff, 0xff, // -1
        0x00,
    ];
    assert_eq!(golden().canonical_bytes(), expected);
}

#[test]
fn golden_hash() {
    // Constant computed with:
    //   xxd -r -p <<< "010500000001000000410103000000000000000101000000000000000001fdffffffffffffff010000004202030000000000000001000000000000e03f0100000000000000000001000000430303000000000000000102000000c3a9000100000000010000004404030000000000000001010100000100000045050300000000000000010000000001ffffffff00" | sha256sum
    assert_eq!(
        golden().content_hash_hex(),
        "09813a1101935a5f28269ee4a1fc691929e43b05a098cf0d90a9705498c4ae6c"
    );
}

#[test]
fn negative_zero_hashes_as_zero() {
    let neg =
        Dataset::new("d", 1, vec![col("R", ColumnData::Real(vec![Some(-0.0)]))]).expect("dataset");
    let pos =
        Dataset::new("d", 1, vec![col("R", ColumnData::Real(vec![Some(0.0)]))]).expect("dataset");
    assert_eq!(neg.canonical_bytes(), pos.canonical_bytes());
    assert_eq!(neg.content_hash_hex(), pos.content_hash_hex());
}

#[test]
fn id_and_version_not_hashed() {
    let a = Dataset::new("one", 1, vec![col("A", ColumnData::Integer(vec![Some(1)]))])
        .expect("dataset");
    let b = Dataset::new(
        "two",
        99,
        vec![col("A", ColumnData::Integer(vec![Some(1)]))],
    )
    .expect("dataset");
    assert_eq!(a.content_hash_hex(), b.content_hash_hex());
}
