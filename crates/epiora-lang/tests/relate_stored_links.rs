//! Stored `dataset_links` in `replay` (v1.1 plan Task 13b): a keyless
//! `RELATE` resolves the project's stored link through
//! [`StoredLinks`] — the `LinkedDatasets` over the session's tables
//! plus the stored rows — to the same view the explicit-key spelling
//! produces; no stored link is `relate_no_link`.

mod common;
use common::ds::*;
use epiora_core::dataset::{Dataset, DatasetLink, LinkKind, Value};
use epiora_lang::{replay, Command, DatasetRef, StoredLinks};
use std::collections::BTreeMap;

fn parent() -> Dataset {
    ds(
        "PARENT",
        vec![
            ("k", texts(&["3", "1", "2"])),
            ("n", texts(&["p3", "p1", "p2"])),
        ],
    )
}

fn child() -> Dataset {
    ds(
        "CHILD",
        vec![("ck", texts(&["1", "1", "1"])), ("h", ints(&[7, 8, 9]))],
    )
}

fn tables() -> BTreeMap<String, Dataset> {
    BTreeMap::from([("child".to_string(), child())])
}

fn stored_link() -> DatasetLink {
    DatasetLink {
        parent: "PARENT".into(),
        child: "CHILD".into(),
        parent_key: "k".into(),
        child_key: "ck".into(),
        kind: LinkKind::Relate,
    }
}

#[test]
fn keyless_relate_uses_stored_link() {
    let linked = StoredLinks::new(tables(), vec![stored_link()]);
    let keyless = replay(
        &parent(),
        &linked,
        &recipe(vec![Command::Relate {
            dataset: DatasetRef::named("CHILD"),
            keys: vec![],
        }]),
    )
    .expect("keyless relate resolves the stored link");
    let keyed = replay(
        &parent(),
        &linked,
        &recipe(vec![Command::Relate {
            dataset: DatasetRef::named("CHILD"),
            keys: vec![("k".into(), "ck".into())],
        }]),
    )
    .expect("explicit-key relate");
    assert_eq!(
        keyless.materialize().content_hash_hex(),
        keyed.materialize().content_hash_hex(),
    );
    // Parent '1' has three children; the other parents are unmatched
    // under MATCHING.
    let m = keyless.materialize();
    assert_eq!(m.row_count(), 3);
    assert_eq!(
        cells(&m, "h"),
        vec![
            Some(Value::Integer(7)),
            Some(Value::Integer(8)),
            Some(Value::Integer(9))
        ]
    );

    // The wasm boundary decodes the replay source under the
    // placeholder id "source": the stored link still resolves, because
    // the source is the one project dataset absent from `linked`.
    let source_side = ds(
        "source",
        vec![
            ("k", texts(&["3", "1", "2"])),
            ("n", texts(&["p3", "p1", "p2"])),
        ],
    );
    let view = replay(
        &source_side,
        &linked,
        &recipe(vec![Command::Relate {
            dataset: DatasetRef::named("CHILD"),
            keys: vec![],
        }]),
    )
    .expect("stored link resolves through the replay placeholder");
    assert_eq!(
        view.materialize().content_hash_hex(),
        keyed.materialize().content_hash_hex(),
    );
}

#[test]
fn keyless_relate_without_link_is_relate_no_link() {
    let linked = StoredLinks::new(tables(), vec![]);
    let err = replay(
        &parent(),
        &linked,
        &recipe(vec![Command::Relate {
            dataset: DatasetRef::named("CHILD"),
            keys: vec![],
        }]),
    )
    .expect_err("no keys and no stored link");
    assert_eq!(err.code(), "relate_no_link");
}
