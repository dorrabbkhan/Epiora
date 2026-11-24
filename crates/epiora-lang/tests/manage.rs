//! The `manage::*` cleaning commands (v1.1 plan Task 11): RECODE
//! boundaries, MERGE's three modes (hand-computed — EI7 7.2.7.0 crashes
//! on the option word; the owner decision of 2026-11-23 pins only the
//! bare match-or-insert form against EI7), RELATE one-to-many, SELECT
//! AND-combining, DELETE/UNDELETE over both raw tables (physical) and
//! `__recstatus` datasets (marked), SUMMARIZE's grouped output and a
//! PERMANENT define entering a session's recipe.

use epiora_core::dataset::{Column, ColumnData, Dataset, DatasetLink, Value, ValueType};
use epiora_core::procedures::{freq, FreqOptions, RunContext};
use epiora_lang::{
    parse_expr, replay, AggFunc, Aggregate, AsOf, Command, DatasetRef, JoinMode, LinkedDatasets,
    MergeMode, Order, Process, Recipe, RecodeBound, RecodeRule, Scope, SetOption, WorkingView,
};
use std::collections::BTreeMap;

// ---------- builders ----------

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

fn ints(v: &[i64]) -> ColumnData {
    ColumnData::Integer(v.iter().map(|x| Some(*x)).collect())
}

fn opt_ints(v: &[Option<i64>]) -> ColumnData {
    ColumnData::Integer(v.to_vec())
}

fn reals(v: &[f64]) -> ColumnData {
    ColumnData::Real(v.iter().map(|x| Some(*x)).collect())
}

fn texts(v: &[&str]) -> ColumnData {
    ColumnData::Text(v.iter().map(|s| Some((*s).to_string())).collect())
}

fn ds(id: &str, cols: Vec<(&str, ColumnData)>) -> Dataset {
    Dataset::new(id, 1, cols.into_iter().map(|(n, d)| col(n, d)).collect()).expect("dataset")
}

fn expr(text: &str) -> epiora_lang::Expr {
    parse_expr(text).expect("expression parses")
}

fn recipe(steps: Vec<Command>) -> Recipe {
    Recipe {
        as_of: AsOf {
            date: 20_000,
            seconds: 0,
        },
        steps,
    }
}

#[derive(Default)]
struct Links {
    tables: BTreeMap<String, Dataset>,
    links: Vec<DatasetLink>,
}

impl LinkedDatasets for Links {
    fn get(&self, name: &str) -> Option<&Dataset> {
        self.tables
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }

    fn link(&self, parent: &str, child: &str) -> Option<&DatasetLink> {
        self.links
            .iter()
            .find(|l| l.parent.eq_ignore_ascii_case(parent) && l.child.eq_ignore_ascii_case(child))
    }
}

fn replay_one(src: &Dataset, steps: Vec<Command>) -> WorkingView {
    replay(src, &Links::default(), &recipe(steps)).expect("replay")
}

/// A column's cells as `Option<Value>` — the assertion form of every
/// test below.
fn cells(ds: &Dataset, name: &str) -> Vec<Option<Value>> {
    let c = ds.column(name).unwrap_or_else(|_| panic!("column {name}"));
    match &c.data {
        ColumnData::Integer(v) => v.iter().map(|x| x.map(Value::Integer)).collect(),
        ColumnData::Real(v) => v.iter().map(|x| x.map(Value::Real)).collect(),
        ColumnData::Text(v) => v.iter().map(|x| x.clone().map(Value::Text)).collect(),
        ColumnData::Boolean(v) => v.iter().map(|x| x.map(Value::Boolean)).collect(),
        ColumnData::Date(v) => v.iter().map(|x| x.map(Value::Date)).collect(),
        ColumnData::Time(v) => v.iter().map(|x| x.map(Value::Time)).collect(),
        ColumnData::DateTime(v) => v.iter().map(|x| x.map(Value::DateTime)).collect(),
    }
}

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-11-23T00:00:00Z".into(),
        project: None,
        working_view: None,
    }
}

// ---------- RECODE ----------

#[test]
fn recode_boundaries_as_ei7() {
    // EI7 pins (recode.pgm, Task 3): LOVALUE/HIVALUE and the inner
    // boundaries are inclusive — 17 lands in group 1, 64 in 2, 65 in 3.
    let src = ds(
        "t",
        vec![(
            "AGE",
            opt_ints(&[Some(3), Some(17), Some(64), Some(65), Some(77), None]),
        )],
    );
    let view = replay_one(
        &src,
        vec![
            Command::Define {
                name: "AGEGRP".into(),
                ty: ValueType::Integer,
                scope: Scope::Standard,
            },
            Command::Recode {
                from: "AGE".into(),
                to: "AGEGRP".into(),
                rules: vec![
                    RecodeRule::Range {
                        lo: RecodeBound::LoValue,
                        hi: RecodeBound::Value(Value::Integer(17)),
                        to: Value::Integer(1),
                    },
                    RecodeRule::Range {
                        lo: RecodeBound::Value(Value::Integer(18)),
                        hi: RecodeBound::Value(Value::Integer(64)),
                        to: Value::Integer(2),
                    },
                    RecodeRule::Range {
                        lo: RecodeBound::Value(Value::Integer(65)),
                        hi: RecodeBound::HiValue,
                        to: Value::Integer(3),
                    },
                ],
                otherwise: None,
            },
        ],
    );
    assert_eq!(
        cells(&view.dataset, "AGEGRP"),
        vec![
            Some(Value::Integer(1)),
            Some(Value::Integer(1)),
            Some(Value::Integer(2)),
            Some(Value::Integer(3)),
            Some(Value::Integer(3)),
            None, // a missing source stays missing
        ]
    );
}

#[test]
fn recode_else_does_not_catch_missing() {
    // Task 3 pin: MPGRP's ELSE catches the unmatched 1s but the missing
    // MASHEDPOTA cell stays missing (total 74, not 75).
    let src = ds(
        "t",
        vec![("MP", opt_ints(&[Some(0), Some(1), None, Some(0), Some(1)]))],
    );
    let view = replay_one(
        &src,
        vec![
            Command::Define {
                name: "MPGRP".into(),
                ty: ValueType::Integer,
                scope: Scope::Standard,
            },
            Command::Recode {
                from: "MP".into(),
                to: "MPGRP".into(),
                rules: vec![RecodeRule::Single {
                    from: Value::Integer(0),
                    to: Value::Integer(0),
                }],
                otherwise: Some(Value::Integer(2)),
            },
        ],
    );
    assert_eq!(
        cells(&view.dataset, "MPGRP"),
        vec![
            Some(Value::Integer(0)),
            Some(Value::Integer(2)),
            None,
            Some(Value::Integer(0)),
            Some(Value::Integer(2)),
        ]
    );
}

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

// ---------- PERMANENT define ----------

#[test]
fn permanent_define_enters_every_session() {
    // A session's recipe opens with the project's permanent-var DEFINE
    // steps (spec §6.1); replayed, they resolve like any global.
    let src = ds("t", vec![("x", ints(&[1, 2]))]);
    let session_recipe = recipe(vec![
        Command::Define {
            name: "P".into(),
            ty: ValueType::Real,
            scope: Scope::Permanent,
        },
        Command::Assign {
            name: "P".into(),
            expr: expr("42"),
        },
        Command::Assign {
            name: "q".into(),
            expr: expr("x + P"),
        },
    ]);
    let view = replay(&src, &Links::default(), &session_recipe).expect("replay");
    assert_eq!(
        cells(&view.materialize(), "q"),
        vec![Some(Value::Real(43.0)), Some(Value::Real(44.0))]
    );
}
