//! Dataset model v2 tests (v1.1 plan Task 6, spec §4.3): the `Time`
//! and `DateTime` column types, per-column metadata, label sets, the
//! hash's `LBLS` section and snapshot format 2.

use epiora_core::dataset::{
    Column, ColumnData, ColumnMeta, Dataset, LabelSet, LinkKind, Value, ValueType,
};
use epiora_core::snapshot::{decode_snapshot, encode_snapshot};
use proptest::prelude::*;
use std::collections::BTreeMap;

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

fn meta(
    prompt: Option<&str>,
    legacy_type: Option<&str>,
    legacy_name: Option<&str>,
    label_set: Option<&str>,
) -> ColumnMeta {
    ColumnMeta {
        prompt: prompt.map(str::to_string),
        legacy_type: legacy_type.map(str::to_string),
        legacy_name: legacy_name.map(str::to_string),
        label_set: label_set.map(str::to_string),
    }
}

/// Every `fixtures/data/*.dataset.json` keeps the hash the v1 canonical
/// encoding gave it on `main` before this task; the golden receipts
/// (which embed those hashes as dataset stamps) are pinned by the
/// existing receipt goldens in `tests/golden/`.
#[cfg(feature = "corpus")]
#[test]
fn v1_datasets_hash_unchanged() {
    const EXPECTED: &[(&str, &str)] = &[
        (
            "coding.dataset.json",
            "d8975288f55900643e6ebb4c9ad29c70831b4a205fde6e30669f7d320158d720",
        ),
        (
            "means_all_missing.dataset.json",
            "08f4c72aacc24dcd3fa677872e6964fe541c9177e02842c9bf5d29bf6a1eccef",
        ),
        (
            "means_basic.dataset.json",
            "3585b5f5ce234eabedf633dec70b48c20b45d652c96e42bfbb953c67f1ec570d",
        ),
        (
            "means_constant.dataset.json",
            "246e32d0cf00289c7229c33e6049bedc076bcec90e79be4432a78dcc3ff5f5c9",
        ),
        (
            "means_dates.dataset.json",
            "3f5326841f2b571138b6738cc898a2e2c0e57dc447b5db0fec2481621a258bf1",
        ),
        (
            "means_one_obs_group.dataset.json",
            "a32f346408a06e4329bc0994ba77ee5c5a35086961494583780a527f558a4050",
        ),
        (
            "means_ties.dataset.json",
            "840ff88c12ed3d9cc8cc9351c502c27878cbcea96f58348368da1c14e7a6e8c4",
        ),
        (
            "missing.dataset.json",
            "3ddae9418290dc025e923f05695c3b0c247f84d73e68f2395a60877cb9d5adb8",
        ),
        (
            "oswego.dataset.json",
            "c55bee96fa7b1c101fd5bcedd0fb2f6ede78185052cbae6558910fe020617561",
        ),
        (
            "strata.dataset.json",
            "0f4781665446b0e1017a2b87afcd007e32097b1ec48041b3721c0f7b85866f3e",
        ),
        (
            "text_levels.dataset.json",
            "cdf7c940f565861a7f6372bacf8074918f4d612d792c326d26798ab3c41aa360",
        ),
    ];
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/data");
    for (file, sha) in EXPECTED {
        let json = std::fs::read_to_string(dir.join(file)).expect("read fixture");
        let ds = epiora_core::corpus::load_dataset(&json).expect("load fixture");
        assert_eq!(ds.content_hash_hex(), *sha, "{file}");
        assert_eq!(ds.meta(), &[] as &[ColumnMeta]);
        assert!(ds.label_sets().is_empty());
    }
}

#[test]
fn labels_change_hash_prompts_do_not() {
    let base = Dataset::new(
        "d",
        1,
        vec![col("I", ColumnData::Integer(vec![Some(1), Some(0)]))],
    )
    .expect("dataset");
    let sets = BTreeMap::from([(
        "ill".to_string(),
        LabelSet {
            id: "ill".to_string(),
            entries: vec![
                (Value::Integer(1), "ill".to_string()),
                (Value::Integer(0), "well".to_string()),
            ],
        },
    )]);

    // Prompts, legacy types and legacy names never enter the hash.
    let prompted = base
        .clone()
        .with_meta(
            vec![meta(
                Some("Were you ill?"),
                Some("Option"),
                Some("ILL"),
                None,
            )],
            BTreeMap::new(),
        )
        .expect("with_meta");
    assert_eq!(prompted.content_hash_hex(), base.content_hash_hex());

    // A label set on a column appends the `LBLS` section.
    let labelled = base
        .clone()
        .with_meta(vec![meta(None, None, None, Some("ill"))], sets.clone())
        .expect("with_meta");
    assert_ne!(labelled.content_hash_hex(), base.content_hash_hex());
    assert!(
        String::from_utf8_lossy(&labelled.canonical_bytes()).contains("LBLS"),
        "labelled dataset must carry the LBLS section"
    );

    // Different label text, different hash.
    let other = base
        .clone()
        .with_meta(
            vec![meta(None, None, None, Some("ill"))],
            BTreeMap::from([(
                "ill".to_string(),
                LabelSet {
                    id: "ill".to_string(),
                    entries: vec![
                        (Value::Integer(1), "sick".to_string()),
                        (Value::Integer(0), "well".to_string()),
                    ],
                },
            )]),
        )
        .expect("with_meta");
    assert_ne!(labelled.content_hash_hex(), other.content_hash_hex());

    // A prompt beside the label set still does not enter the hash.
    let prompted_labelled = base
        .with_meta(vec![meta(Some("Q"), None, None, Some("ill"))], sets)
        .expect("with_meta");
    assert_eq!(
        prompted_labelled.content_hash_hex(),
        labelled.content_hash_hex()
    );

    // `label_for` resolves through the column's meta.
    assert_eq!(
        labelled.label_for(0, &Value::Integer(1)),
        Some("ill"),
        "label_for"
    );
    assert_eq!(labelled.label_for(0, &Value::Integer(9)), None);

    // An unknown label-set id is the typed `label_set_missing` error.
    let err = labelled
        .clone()
        .with_meta(vec![meta(None, None, None, Some("nope"))], BTreeMap::new())
        .unwrap_err();
    assert_eq!(err.code(), "label_set_missing");
}

#[test]
fn time_out_of_range_rejected() {
    for bad in [-1, 86_400, i32::MAX] {
        let err =
            Dataset::new("d", 1, vec![col("T", ColumnData::Time(vec![Some(bad)]))]).unwrap_err();
        assert_eq!(err.code(), "invalid_time", "value {bad}");
    }
    Dataset::new(
        "d",
        1,
        vec![col(
            "T",
            ColumnData::Time(vec![Some(0), Some(86_399), None]),
        )],
    )
    .expect("in-range time");
    // A label-set entry cannot hold an out-of-range time either.
    let ds =
        Dataset::new("d", 1, vec![col("I", ColumnData::Integer(vec![Some(1)]))]).expect("dataset");
    let err = ds
        .with_meta(
            vec![meta(None, None, None, Some("s"))],
            BTreeMap::from([(
                "s".to_string(),
                LabelSet {
                    id: "s".to_string(),
                    entries: vec![(Value::Time(90_000), "late".to_string())],
                },
            )]),
        )
        .unwrap_err();
    assert_eq!(err.code(), "invalid_time");
}

#[test]
fn snapshot_v1_still_decodes() {
    let ds = Dataset::new(
        "d",
        3,
        vec![
            col("I", ColumnData::Integer(vec![Some(1), None])),
            col("T", ColumnData::Text(vec![Some("x".into()), None])),
            col("D", ColumnData::Date(vec![Some(0), Some(-1)])),
        ],
    )
    .expect("dataset");
    let bytes = encode_snapshot(&ds);
    assert_eq!(&bytes[..4], b"EPSN");
    assert_eq!(bytes[4], 1, "a dataset without v2 features writes v1");
    let back = decode_snapshot("d", 3, &bytes).expect("decode v1");
    assert_eq!(back.columns(), ds.columns());
    assert_eq!(back.content_hash_hex(), ds.content_hash_hex());
    assert!(back.meta().is_empty());
    assert!(back.label_sets().is_empty());
}

fn arb_time() -> impl Strategy<Value = Option<i32>> {
    prop::option::of(0i32..=86_399)
}

fn arb_coldata() -> impl Strategy<Value = ColumnData> {
    prop_oneof![
        5 => prop::collection::vec(prop::option::of(any::<i64>()), 0..8)
            .prop_map(ColumnData::Integer),
        5 => prop::collection::vec(prop::option::of(any::<f64>().prop_map(|x| {
            if x.is_finite() { x } else { 0.0 }
        })), 0..8)
            .prop_map(ColumnData::Real),
        5 => prop::collection::vec(prop::option::of("[a-zé]{0,6}"), 0..8)
            .prop_map(ColumnData::Text),
        5 => prop::collection::vec(prop::option::of(any::<bool>()), 0..8)
            .prop_map(ColumnData::Boolean),
        5 => prop::collection::vec(prop::option::of(any::<i32>()), 0..8)
            .prop_map(ColumnData::Date),
        5 => prop::collection::vec(arb_time(), 0..8).prop_map(ColumnData::Time),
        5 => prop::collection::vec(prop::option::of(any::<i64>()), 0..8)
            .prop_map(ColumnData::DateTime),
    ]
}

fn arb_value() -> impl Strategy<Value = Value> {
    prop_oneof![
        any::<i64>().prop_map(Value::Integer),
        any::<f64>().prop_map(|x| Value::Real(if x.is_finite() { x } else { 0.0 })),
        "[a-zé]{0,6}".prop_map(Value::Text),
        any::<bool>().prop_map(Value::Boolean),
        any::<i32>().prop_map(Value::Date),
        (0i32..=86_399).prop_map(Value::Time),
        any::<i64>().prop_map(Value::DateTime),
    ]
}

fn aligned(datas: Vec<ColumnData>) -> Vec<ColumnData> {
    let rows = datas.iter().map(|d| d.len()).min().unwrap_or(0);
    datas
        .into_iter()
        .map(|d| {
            let mut d = d;
            match &mut d {
                ColumnData::Integer(v) => v.truncate(rows),
                ColumnData::Real(v) => v.truncate(rows),
                ColumnData::Text(v) => v.truncate(rows),
                ColumnData::Boolean(v) => v.truncate(rows),
                ColumnData::Date(v) => v.truncate(rows),
                ColumnData::Time(v) => v.truncate(rows),
                ColumnData::DateTime(v) => v.truncate(rows),
            }
            d
        })
        .collect()
}

proptest! {
    /// Snapshot v2: every dataset, with meta and label sets, round-trips
    /// to a dataset whose columns, meta, label sets and hash reproduce.
    #[test]
    fn snapshot_v2_roundtrip_reproduces_hash(
        columns in prop::collection::vec(arb_coldata(), 1..5)
            .prop_map(|datas| {
                aligned(datas)
                    .into_iter()
                    .enumerate()
                    .map(|(i, data)| Column {
                        name: format!("c{i}"),
                        data,
                    })
                    .collect::<Vec<_>>()
            }),
        version in 1u64..100,
        label_sets in prop::collection::btree_map(
            "[a-z]{1,4}",
            prop::collection::vec((arb_value(), "[a-z ]{0,8}"), 0..5),
            0..4,
        ),
    ) {
        let ncols = columns.len();
        let sets: BTreeMap<String, LabelSet> = label_sets
            .into_iter()
            .map(|(id, entries)| {
                (id.clone(), LabelSet { id, entries })
            })
            .collect();
        let ids: Vec<String> = sets.keys().cloned().collect();
        let meta: Vec<ColumnMeta> = (0..ncols)
            .map(|i| {
                if i == 0 && !ids.is_empty() {
                    meta(None, None, None, Some(&ids[0]))
                } else {
                    meta(None, None, None, None)
                }
            })
            .collect();
        let ds = Dataset::new("p", version, columns)
            .expect("dataset")
            .with_meta(meta.clone(), sets.clone())
            .expect("with_meta");
        let bytes = encode_snapshot(&ds);
        let has_v2 = ds
            .columns()
            .iter()
            .any(|c| matches!(c.data, ColumnData::Time(_) | ColumnData::DateTime(_)))
            || !ds.meta().is_empty()
            || !ds.label_sets().is_empty();
        prop_assert_eq!(bytes[4], if has_v2 { 2 } else { 1 });
        let back = decode_snapshot("p", version, &bytes).expect("decode");
        prop_assert_eq!(back.columns(), ds.columns());
        prop_assert_eq!(back.meta(), ds.meta());
        prop_assert_eq!(back.label_sets(), ds.label_sets());
        prop_assert_eq!(back.content_hash_hex(), ds.content_hash_hex());
        // Re-encoding the decoded dataset reproduces the same bytes.
        prop_assert_eq!(encode_snapshot(&back), bytes);
    }
}

#[test]
fn link_types_exist() {
    // Links live in the project (store), never inside `Dataset`.
    let link = epiora_core::dataset::DatasetLink {
        parent: "v1".into(),
        child: "v2".into(),
        child_key: "FKEY".into(),
        parent_key: "GlobalRecordId".into(),
        kind: LinkKind::Relate,
    };
    assert_eq!(link.kind, LinkKind::Relate);
    assert_ne!(link.kind, LinkKind::Grid);
}

#[test]
fn value_type_tags() {
    assert_eq!(ColumnData::Time(vec![]).value_type(), ValueType::Time);
    assert_eq!(
        ColumnData::DateTime(vec![]).value_type(),
        ValueType::DateTime
    );
}
