//! The `manage::*` view-state commands (v1.1 plan Task 11): SELECT
//! AND-combining, SORT, DELETE/UNDELETE over both raw tables (physical)
//! and `__recstatus` datasets (marked), and SUMMARIZE's grouped output.
//! Shared builders live in `tests/common/ds.rs`.

mod common;
use common::ds::*;
use epiora_core::dataset::Value;
use epiora_lang::{AggFunc, Aggregate, Command, DatasetRef, Order, Process, SetOption};
use std::collections::BTreeMap;

// ---------- SELECT / SORT ----------

#[test]
fn select_combines_with_and() {
    // Task 3 pin: SELECT ILL=1 keeps 46 rows; SELECT AGE>40 then keeps
    // 22 of those — the second select intersects, it does not restart.
    let src = ds(
        "t",
        vec![
            ("ILL", ints(&[1, 0, 1, 1, 0, 1])),
            ("AGE", ints(&[59, 33, 8, 52, 20, 35])),
        ],
    );
    let view = replay_one(
        &src,
        vec![
            Command::Select(expr("ILL = 1")),
            Command::Select(expr("AGE > 40")),
        ],
    );
    let m = view.materialize();
    assert_eq!(m.row_count(), 2);
    assert_eq!(
        cells(&m, "AGE"),
        vec![Some(Value::Integer(59)), Some(Value::Integer(52))]
    );
}

#[test]
fn select_and_sort_cancel_restore() {
    let src = ds(
        "t",
        vec![("ILL", ints(&[1, 0, 1])), ("AGE", ints(&[59, 33, 8]))],
    );
    let view = replay_one(
        &src,
        vec![Command::Select(expr("ILL = 1")), Command::CancelSelect],
    );
    assert_eq!(view.materialize().row_count(), 3);

    let sorted = replay_one(
        &src,
        vec![Command::Sort(vec![
            ("AGE".into(), Order::Desc),
            ("ILL".into(), Order::Asc),
        ])],
    );
    assert_eq!(
        cells(&sorted.materialize(), "AGE"),
        vec![
            Some(Value::Integer(59)),
            Some(Value::Integer(33)),
            Some(Value::Integer(8)),
        ]
    );
    let restored = replay_one(
        &src,
        vec![
            Command::Sort(vec![("AGE".into(), Order::Asc)]),
            Command::CancelSort,
        ],
    );
    assert_eq!(
        cells(&restored.materialize(), "AGE"),
        vec![
            Some(Value::Integer(59)),
            Some(Value::Integer(33)),
            Some(Value::Integer(8)),
        ]
    );
}

#[test]
fn sort_places_missing_first() {
    // Task 3 pin (SORT TIMESUPPER): missing cells lead the ascending
    // order.
    let src = ds(
        "t",
        vec![("T", opt_ints(&[None, Some(5), Some(2), None, Some(9)]))],
    );
    let view = replay_one(&src, vec![Command::Sort(vec![("T".into(), Order::Asc)])]);
    assert_eq!(
        cells(&view.materialize(), "T"),
        vec![
            None,
            None,
            Some(Value::Integer(2)),
            Some(Value::Integer(5)),
            Some(Value::Integer(9)),
        ]
    );
}

// ---------- DELETE / UNDELETE ----------

#[test]
fn delete_records_then_write_creates_version() {
    // Raw tables carry no RecStatus: a session DELETE removes the rows
    // for good (EI7 "Deleted Records Included: No" from then on).
    let src = ds(
        "t",
        vec![
            ("x", ints(&[1, 2, 3, 4, 5])),
            ("y", texts(&["a", "b", "c", "d", "e"])),
        ],
    );
    let before = src.content_hash_hex();
    let view = replay_one(&src, vec![Command::DeleteRecords(expr("x < 3"))]);
    let written = view.materialize();
    assert_eq!(written.row_count(), 3);
    assert_eq!(
        cells(&written, "x"),
        vec![
            Some(Value::Integer(3)),
            Some(Value::Integer(4)),
            Some(Value::Integer(5)),
        ]
    );
    // WRITE is a dataset-version statement (Task 12): the materialized
    // view is what becomes version 2 — its content hash differs from
    // the source's.
    assert_ne!(written.content_hash_hex(), before);
}

#[test]
fn undelete_restores_session_deleted_records() {
    // A project dataset keeps deleted marks in `__recstatus`: DELETE
    // flips 1→0 (processed rows drop), UNDELETE * flips them back.
    let mut ds = ds(
        "t",
        vec![
            ("__recstatus", ints(&[1, 1, 1, 1])),
            ("x", ints(&[1, 2, 3, 4])),
        ],
    );
    // Give the marker column its legacy type so the view treats it as
    // the RecStatus carrier (not an ordinary column).
    let metas: Vec<_> = ds
        .columns()
        .iter()
        .map(|c| {
            let mut m = epiora_core::dataset::ColumnMeta::default();
            if c.name == "__recstatus" {
                m.legacy_type = Some("RecStatus".into());
            }
            m
        })
        .collect();
    ds = ds.with_meta(metas, BTreeMap::new()).expect("meta");

    let deleted = replay_one(&ds, vec![Command::DeleteRecords(expr("x < 3"))]);
    assert_eq!(deleted.materialize().row_count(), 2);
    assert!(!deleted.deleted_records_included());

    let restored = replay_one(
        &ds,
        vec![
            Command::DeleteRecords(expr("x < 3")),
            Command::Undelete(None),
        ],
    );
    assert_eq!(restored.materialize().row_count(), 4);
    assert!(restored.deleted_records_included());
}

#[test]
fn set_process_both_includes_deleted_marks() {
    let mut ds = ds(
        "t",
        vec![("__recstatus", ints(&[0, 1, 1])), ("x", ints(&[1, 2, 3]))],
    );
    let metas: Vec<_> = ds
        .columns()
        .iter()
        .map(|c| {
            let mut m = epiora_core::dataset::ColumnMeta::default();
            if c.name == "__recstatus" {
                m.legacy_type = Some("RecStatus".into());
            }
            m
        })
        .collect();
    ds = ds.with_meta(metas, BTreeMap::new()).expect("meta");

    let plain = replay_one(&ds, vec![]);
    assert_eq!(plain.materialize().row_count(), 2);
    let both = replay_one(&ds, vec![Command::Set(SetOption::Process(Process::Both))]);
    assert_eq!(both.materialize().row_count(), 3);
}

// ---------- SUMMARIZE ----------

#[test]
fn summarize_by_groups_writes_new_dataset() {
    // Grammar: SUMMARIZE out::AGG(x) ... TO table [STRATAVAR= s].
    // Hand-computed: groups 'a' rows 1,2; 'b' rows 3,4.
    let src = ds(
        "t",
        vec![
            ("g", texts(&["a", "a", "b", "b"])),
            ("x", ints(&[1, 2, 3, 4])),
        ],
    );
    let view = replay_one(
        &src,
        vec![Command::Summarize {
            aggregates: vec![
                Aggregate {
                    name: "total".into(),
                    func: AggFunc::Sum,
                    arg: Some("x".into()),
                },
                Aggregate {
                    name: "n".into(),
                    func: AggFunc::Count,
                    arg: None,
                },
                Aggregate {
                    name: "m".into(),
                    func: AggFunc::Avg,
                    arg: Some("x".into()),
                },
                Aggregate {
                    name: "lo".into(),
                    func: AggFunc::Min,
                    arg: Some("x".into()),
                },
                Aggregate {
                    name: "hi".into(),
                    func: AggFunc::Max,
                    arg: Some("x".into()),
                },
                Aggregate {
                    name: "first_x".into(),
                    func: AggFunc::First,
                    arg: Some("x".into()),
                },
                Aggregate {
                    name: "last_x".into(),
                    func: AggFunc::Last,
                    arg: Some("x".into()),
                },
                Aggregate {
                    name: "sd".into(),
                    func: AggFunc::Stdev,
                    arg: Some("x".into()),
                },
            ],
            by: vec!["g".into()],
            into: DatasetRef::named("out"),
        }],
    );
    let m = view.materialize();
    assert_eq!(m.row_count(), 2);
    assert_eq!(
        cells(&m, "g"),
        vec![Some(Value::Text("a".into())), Some(Value::Text("b".into()))]
    );
    assert_eq!(
        cells(&m, "total"),
        vec![Some(Value::Real(3.0)), Some(Value::Real(7.0))]
    );
    assert_eq!(
        cells(&m, "n"),
        vec![Some(Value::Integer(2)), Some(Value::Integer(2))]
    );
    assert_eq!(
        cells(&m, "m"),
        vec![Some(Value::Real(1.5)), Some(Value::Real(3.5))]
    );
    assert_eq!(
        cells(&m, "lo"),
        vec![Some(Value::Integer(1)), Some(Value::Integer(3))]
    );
    assert_eq!(
        cells(&m, "hi"),
        vec![Some(Value::Integer(2)), Some(Value::Integer(4))]
    );
    assert_eq!(
        cells(&m, "first_x"),
        vec![Some(Value::Integer(1)), Some(Value::Integer(3))]
    );
    assert_eq!(
        cells(&m, "last_x"),
        vec![Some(Value::Integer(2)), Some(Value::Integer(4))]
    );
    let sd = cells(&m, "sd");
    for v in sd {
        match v {
            Some(Value::Real(x)) => assert!((x - std::f64::consts::FRAC_1_SQRT_2).abs() < 1e-9),
            other => panic!("sd {other:?}"),
        }
    }
}
