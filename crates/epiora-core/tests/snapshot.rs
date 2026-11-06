//! Snapshot codec tests (v0.3 spec §5.1): round trips, strict
//! canonical decoding and a property test over arbitrary datasets.

use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::snapshot::{decode_snapshot, encode_snapshot, SNAPSHOT_FORMAT, SNAPSHOT_MAGIC};
use epiora_core::EngineError;
use proptest::prelude::*;

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
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

fn assert_same_dataset(expected: &Dataset, decoded: &Dataset) {
    assert_eq!(decoded.id(), expected.id());
    assert_eq!(decoded.version(), expected.version());
    assert_eq!(decoded.columns(), expected.columns());
    assert_eq!(decoded.content_hash_hex(), expected.content_hash_hex());
}

fn snapshot_decode(err: EngineError) {
    assert_eq!(err.code(), "snapshot_decode", "{err}");
}

#[test]
fn roundtrip_golden_dataset() {
    let ds = golden();
    let bytes = encode_snapshot(&ds);
    assert_eq!(&bytes[..4], &SNAPSHOT_MAGIC);
    assert_eq!(bytes[4], SNAPSHOT_FORMAT);
    let decoded = decode_snapshot("golden", 7, &bytes).expect("decode");
    assert_same_dataset(&ds, &decoded);
}

#[test]
fn roundtrip_zero_rows() {
    let ds = Dataset::new(
        "empty",
        3,
        vec![
            col("I", ColumnData::Integer(vec![])),
            col("T", ColumnData::Text(vec![])),
            col("D", ColumnData::Date(vec![])),
        ],
    )
    .expect("zero rows");
    let decoded = decode_snapshot("empty", 3, &encode_snapshot(&ds)).expect("decode");
    assert_same_dataset(&ds, &decoded);
}

#[test]
fn negative_zero_encodes_as_zero() {
    let neg =
        Dataset::new("d", 1, vec![col("R", ColumnData::Real(vec![Some(-0.0)]))]).expect("dataset");
    let pos =
        Dataset::new("d", 1, vec![col("R", ColumnData::Real(vec![Some(0.0)]))]).expect("dataset");
    assert_eq!(encode_snapshot(&neg), encode_snapshot(&pos));
}

#[test]
fn rejects_bad_magic() {
    let mut bytes = encode_snapshot(&golden());
    bytes[0] = b'X';
    snapshot_decode(decode_snapshot("g", 7, &bytes).unwrap_err());
}

#[test]
fn rejects_unknown_format() {
    let mut bytes = encode_snapshot(&golden());
    bytes[4] = 9;
    snapshot_decode(decode_snapshot("g", 7, &bytes).unwrap_err());
}

#[test]
fn rejects_truncated() {
    let bytes = encode_snapshot(&golden());
    for prefix in 0..bytes.len() {
        let err = decode_snapshot("g", 7, &bytes[..prefix])
            .expect_err(&format!("prefix of {} bytes must fail", prefix));
        assert_eq!(err.code(), "snapshot_decode", "prefix {prefix}: {err}");
    }
}

#[test]
fn rejects_trailing_bytes() {
    let mut bytes = encode_snapshot(&golden());
    bytes.push(0);
    snapshot_decode(decode_snapshot("g", 7, &bytes).unwrap_err());
}

#[test]
fn rejects_unknown_type_tag() {
    let mut bytes = encode_snapshot(&golden());
    // Header is magic(4) + format(1) + column count(4) + row count(8) =
    // 17 bytes; column A's name is u32 length + 1 UTF-8 byte, so its
    // type tag sits at offset 17 + 4 + 1.
    bytes[22] = 9;
    snapshot_decode(decode_snapshot("g", 7, &bytes).unwrap_err());
    bytes[22] = 0;
    snapshot_decode(decode_snapshot("g", 7, &bytes).unwrap_err());
}

#[test]
fn rejects_invalid_utf8() {
    // Column A's name byte sits at offset 21 (see layout above).
    let mut bytes = encode_snapshot(&golden());
    bytes[21] = 0xff;
    snapshot_decode(decode_snapshot("g", 7, &bytes).unwrap_err());

    // A text cell that is not UTF-8: single Text column "T" one row
    // ["a"], blob byte at 17 + 5 + 1 + 1 + 8 = 32.
    let ds = Dataset::new(
        "d",
        1,
        vec![col("T", ColumnData::Text(vec![Some("a".to_string())]))],
    )
    .expect("dataset");
    let mut bytes = encode_snapshot(&ds);
    let last = bytes.len() - 1;
    bytes[last] = 0xff;
    snapshot_decode(decode_snapshot("d", 1, &bytes).unwrap_err());
}

#[test]
fn rejects_offsets_not_monotonic() {
    // Text column "T", two rows ["ab", "c"]: header 17, name 5, tag 1,
    // bitmap 1, then offsets [0, 2, 3] at 24, 28, 32.
    let ds = Dataset::new(
        "d",
        1,
        vec![col(
            "T",
            ColumnData::Text(vec![Some("ab".to_string()), Some("c".to_string())]),
        )],
    )
    .expect("dataset");
    let mut bytes = encode_snapshot(&ds);
    bytes[28..32].copy_from_slice(&5u32.to_le_bytes());
    snapshot_decode(decode_snapshot("d", 1, &bytes).unwrap_err());
}

#[test]
fn rejects_nan_real() {
    // One Real row, present bit set, bits = NaN: `Dataset::new`
    // rejects it as `InvalidColumn`, not `SnapshotDecode`.
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&SNAPSHOT_MAGIC);
    bytes.push(SNAPSHOT_FORMAT);
    bytes.extend_from_slice(&1u32.to_le_bytes()); // one column
    bytes.extend_from_slice(&1u64.to_le_bytes()); // one row
    bytes.extend_from_slice(&1u32.to_le_bytes()); // name "R"
    bytes.push(b'R');
    bytes.push(2); // tag real
    bytes.push(0b1); // bitmap: row 0 present
    bytes.extend_from_slice(&f64::NAN.to_le_bytes());
    let err = decode_snapshot("d", 1, &bytes).unwrap_err();
    assert_eq!(err.code(), "invalid_column", "{err}");
}

fn arb_data(rows: usize) -> impl Strategy<Value = ColumnData> {
    (0u8..5).prop_flat_map(move |tag| match tag {
        0 => prop::collection::vec(prop::option::of(any::<i64>()), rows)
            .prop_map(ColumnData::Integer)
            .boxed(),
        1 => prop::collection::vec(
            prop::option::of(any::<f64>().prop_map(|x| if x.is_finite() { x } else { 0.0 })),
            rows,
        )
        .prop_map(ColumnData::Real)
        .boxed(),
        2 => prop::collection::vec(prop::option::of(any::<String>()), rows)
            .prop_map(ColumnData::Text)
            .boxed(),
        3 => prop::collection::vec(prop::option::of(any::<bool>()), rows)
            .prop_map(ColumnData::Boolean)
            .boxed(),
        _ => prop::collection::vec(prop::option::of(any::<i32>()), rows)
            .prop_map(ColumnData::Date)
            .boxed(),
    })
}

fn arb_dataset() -> impl Strategy<Value = Dataset> {
    (0usize..=5, 0usize..=200)
        .prop_flat_map(|(ncols, rows)| prop::collection::vec(arb_data(rows), ncols))
        .prop_filter_map("Dataset::new accepts", |datas| {
            // Zero columns is `EmptyDataset` by spec §3.3 — no dataset
            // to round-trip.
            let columns = datas
                .into_iter()
                .enumerate()
                .map(|(i, data)| col(&format!("c{i}"), data))
                .collect();
            Dataset::new("prop", 9, columns).ok()
        })
}

proptest! {
    #[test]
    fn proptest_roundtrip_preserves_hash(ds in arb_dataset()) {
        let bytes = encode_snapshot(&ds);
        let decoded = decode_snapshot(ds.id(), ds.version(), &bytes).expect("decode");
        prop_assert_eq!(decoded.content_hash_hex(), ds.content_hash_hex());
        prop_assert_eq!(decoded.columns(), ds.columns());
        prop_assert_eq!(encode_snapshot(&decoded), bytes);
    }
}
