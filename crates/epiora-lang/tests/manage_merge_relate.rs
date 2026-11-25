//! The `manage::*` MERGE and RELATE commands (v1.1 plan Task 11):
//! MERGE's three modes (hand-computed — EI7 7.2.7.0 crashes on the
//! option word; the owner decision of 2026-11-23 pins only the bare
//! match-or-insert form against EI7) and RELATE one-to-many. Shared
//! builders live in `tests/common/ds.rs`.

mod common;
use common::ds::*;
use epiora_core::dataset::{Dataset, DatasetLink, Value};
use epiora_core::procedures::{freq, FreqOptions};
use epiora_lang::{replay, Command, DatasetRef, JoinMode, MergeMode};
use std::collections::BTreeMap;

// ---------- MERGE ----------

fn merge_dest() -> Dataset {
    ds(
        "N1",
        vec![
            ("k", texts(&["1", "2"])),
            ("v", ints(&[10, 20])),
            ("g", texts(&["a", "b"])),
        ],
    )
}

fn merge_src() -> Dataset {
    ds(
        "V2",
        vec![
            ("k", texts(&["1", "1", "3"])),
            ("v", ints(&[1, 2, 3])),
            ("g", texts(&["x", "y", "z"])),
            ("extra", texts(&["p", "q", "r"])),
        ],
    )
}

fn linked() -> Links {
    Links {
        tables: BTreeMap::from([("V2".to_string(), merge_src())]),
        links: vec![],
    }
}

#[test]
fn merge_duplicate_keys_update_and_append() {
    // Bare match-or-insert: both '1' sources update dest row '1'
    // (last wins), '3' inserts; '2' is untouched and the new shared
    // `extra` column is missing on it.
    let tables = linked();
    let view = replay(
        &merge_dest(),
        &tables,
        &recipe(vec![Command::Merge {
            dataset: DatasetRef::named("V2"),
            keys: vec![("k".into(), "k".into())],
            mode: MergeMode::MatchOrInsert,
        }]),
    )
    .expect("merge");
    assert_eq!(
        cells(&view.dataset, "k"),
        vec![
            Some(Value::Text("1".into())),
            Some(Value::Text("2".into())),
            Some(Value::Text("3".into())),
        ]
    );
    assert_eq!(
        cells(&view.dataset, "v"),
        vec![
            Some(Value::Integer(2)),
            Some(Value::Integer(20)),
            Some(Value::Integer(3)),
        ]
    );
    assert_eq!(
        cells(&view.dataset, "extra"),
        vec![
            Some(Value::Text("q".into())),
            None,
            Some(Value::Text("r".into())),
        ]
    );

    // UPDATE: matched rows update (last wins), no inserts.
    let upd = replay(
        &merge_dest(),
        &tables,
        &recipe(vec![Command::Merge {
            dataset: DatasetRef::named("V2"),
            keys: vec![("k".into(), "k".into())],
            mode: MergeMode::Update,
        }]),
    )
    .expect("update merge");
    assert_eq!(upd.dataset.row_count(), 2);
    assert_eq!(
        cells(&upd.dataset, "v"),
        vec![Some(Value::Integer(2)), Some(Value::Integer(20)),]
    );
    assert_eq!(
        cells(&upd.dataset, "extra"),
        vec![Some(Value::Text("q".into())), None,]
    );

    // APPEND: every source row appends — duplicate keys and all.
    let app = replay(
        &merge_dest(),
        &tables,
        &recipe(vec![Command::Merge {
            dataset: DatasetRef::named("V2"),
            keys: vec![("k".into(), "k".into())],
            mode: MergeMode::Append,
        }]),
    )
    .expect("append merge");
    assert_eq!(app.dataset.row_count(), 5);
    assert_eq!(
        cells(&app.dataset, "k"),
        vec!["1", "2", "1", "1", "3"]
            .into_iter()
            .map(|s| Some(Value::Text(s.into())))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        cells(&app.dataset, "v"),
        vec![10, 20, 1, 2, 3]
            .into_iter()
            .map(|x| Some(Value::Integer(x)))
            .collect::<Vec<_>>()
    );
}

#[test]
fn merge_key_type_mismatch_is_typed() {
    let dest = ds("D", vec![("k", ints(&[1])), ("v", ints(&[5]))]);
    let src = ds("S", vec![("k", texts(&["1"])), ("v", ints(&[9]))]);
    let tables = Links {
        tables: BTreeMap::from([("S".to_string(), src)]),
        links: vec![],
    };
    let err = replay(
        &dest,
        &tables,
        &recipe(vec![Command::Merge {
            dataset: DatasetRef::named("S"),
            keys: vec![("k".into(), "k".into())],
            mode: MergeMode::MatchOrInsert,
        }]),
    )
    .expect_err("int-vs-text keys are merge_key_type");
    assert_eq!(err.code(), "merge_key_type");
}

// ---------- RELATE ----------

#[test]
fn relate_one_to_many_then_freq_counts_children() {
    // Task 3 pin: Nutrition1's three parents ordered 3,1,2; all eleven
    // PatientVisits2 children key parent "1" — MATCHING keeps eleven
    // rows, ALL adds the two unmatched parents with missing children.
    let parent = ds(
        "N1",
        vec![
            ("k", texts(&["3", "1", "2"])),
            ("n", texts(&["p3", "p1", "p2"])),
        ],
    );
    let child = ds(
        "V2",
        vec![
            ("ck", texts(&["1", "1", "1"])),
            ("cn", texts(&["c1", "c2", "c3"])),
        ],
    );
    let tables = Links {
        tables: BTreeMap::from([("V2".to_string(), child)]),
        links: vec![],
    };

    let matching = replay(
        &parent,
        &tables,
        &recipe(vec![Command::Relate {
            dataset: DatasetRef::named("V2"),
            keys: vec![("k".into(), "ck".into())],
        }]),
    )
    .expect("relate");
    let m = matching.materialize();
    assert_eq!(m.row_count(), 3);
    assert_eq!(
        cells(&m, "cn"),
        vec![
            Some(Value::Text("c1".into())),
            Some(Value::Text("c2".into())),
            Some(Value::Text("c3".into())),
        ]
    );
    // FREQ over the related view counts the children under parent 1.
    let out = freq(
        &m,
        &FreqOptions {
            variables: vec!["k".into()],
            stratify_by: None,
            include_missing: false,
            ci_method: Default::default(),
        },
        &ctx(),
    )
    .expect("freq");
    assert_eq!(out.result.tables[0].categories[0].count, 3);

    // Corpus-sized: eleven children of one parent.
    let child11 = ds(
        "V2",
        vec![("ck", texts(&["1"; 11])), ("h", reals(&[1.0; 11]))],
    );
    let tables11 = Links {
        tables: BTreeMap::from([("V2".to_string(), child11)]),
        links: vec![],
    };
    let all = replay(
        &parent,
        &tables11,
        &recipe(vec![Command::Relate {
            dataset: DatasetRef {
                name: "V2".into(),
                join: JoinMode::All,
            },
            keys: vec![("k".into(), "ck".into())],
        }]),
    )
    .expect("relate all");
    let am = all.materialize();
    assert_eq!(am.row_count(), 13);
    let ks = cells(&am, "k");
    assert_eq!(ks[0], Some(Value::Text("3".into())));
    assert_eq!(ks[1], Some(Value::Text("1".into())));
    assert_eq!(ks[11], Some(Value::Text("1".into())));
    assert_eq!(ks[12], Some(Value::Text("2".into())));
    // Unmatched parents carry missing child cells.
    let hs = cells(&am, "h");
    assert!(hs[0].is_none() && hs[12].is_none() && hs[1].is_some());
}

#[test]
fn relate_without_keys_uses_stored_link_or_relate_no_link() {
    let parent = ds("N1", vec![("k", texts(&["1"])), ("g", texts(&["g1"]))]);
    let child = ds("V2", vec![("ck", texts(&["1"])), ("h", ints(&[7]))]);
    let with_link = Links {
        tables: BTreeMap::from([("V2".to_string(), child.clone())]),
        links: vec![DatasetLink {
            parent: "N1".into(),
            child: "V2".into(),
            parent_key: "k".into(),
            child_key: "ck".into(),
            kind: epiora_core::dataset::LinkKind::Relate,
        }],
    };
    let view = replay(
        &parent,
        &with_link,
        &recipe(vec![Command::Relate {
            dataset: DatasetRef::named("V2"),
            keys: vec![],
        }]),
    )
    .expect("stored link relates");
    assert_eq!(view.materialize().row_count(), 1);
    assert_eq!(cells(&view.dataset, "h"), vec![Some(Value::Integer(7))]);

    let no_link = Links {
        tables: BTreeMap::from([("V2".to_string(), child)]),
        links: vec![],
    };
    let err = replay(
        &parent,
        &no_link,
        &recipe(vec![Command::Relate {
            dataset: DatasetRef::named("V2"),
            keys: vec![],
        }]),
    )
    .expect_err("no keys and no stored link");
    assert_eq!(err.code(), "relate_no_link");
}
