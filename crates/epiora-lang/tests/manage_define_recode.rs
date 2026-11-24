//! The `manage::*` DEFINE/ASSIGN/RECODE commands (v1.1 plan Task 11):
//! RECODE boundaries and a PERMANENT define entering a session's
//! recipe. Shared builders live in `tests/common/ds.rs`.

mod common;
use common::ds::*;
use epiora_core::dataset::{Value, ValueType};
use epiora_lang::{replay, Command, RecodeBound, RecodeRule, Scope};

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
