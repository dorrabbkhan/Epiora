//! Recipes (v1.1 plan Task 11): canonical text stability, replay
//! determinism over random command sequences, and typed errors.
//! Receipt v4 tests live in `tests/receipt.rs`; shared builders in
//! `tests/common/ds.rs`.

mod common;
use common::ds::*;
use epiora_core::dataset::{ColumnData, Dataset, Value, ValueType};
use epiora_lang::{
    parse_expr, replay, Command, DatasetRef, MergeMode, Order, Process, Recipe, RecodeBound,
    RecodeRule, Scope, SetOption,
};
use proptest::prelude::*;

// ---------- canonical text ----------

#[test]
fn canonical_text_is_stable() {
    let r = recipe(vec![
        Command::Select(expr("AGE > 40 AND ILL = 1")),
        Command::Sort(vec![
            ("AGE".into(), Order::Asc),
            ("SEX".into(), Order::Desc),
        ]),
        Command::Define {
            name: "BIG".into(),
            ty: ValueType::Real,
            scope: Scope::Standard,
        },
        Command::Assign {
            name: "BIG".into(),
            expr: expr("AGE * 10"),
        },
        Command::Set(SetOption::Process(Process::Both)),
        Command::CancelSelect,
        Command::CancelSort,
        Command::Merge {
            dataset: DatasetRef::named("Visits"),
            keys: vec![("PatientID".into(), "VisitPatientID".into())],
            mode: MergeMode::Update,
        },
        Command::Recode {
            from: "AGE".into(),
            to: "G".into(),
            rules: vec![
                RecodeRule::Range {
                    lo: RecodeBound::LoValue,
                    hi: RecodeBound::Value(Value::Integer(17)),
                    to: Value::Integer(1),
                },
                RecodeRule::Single {
                    from: Value::Integer(64),
                    to: Value::Integer(2),
                },
            ],
            otherwise: Some(Value::Integer(9)),
        },
        Command::DeleteRecords(expr("ILL = 0")),
        Command::Undelete(None),
        Command::DeleteVariables(vec!["TMP".into(), "TMP2".into()]),
        Command::KeyVars(vec!["PatientID".into()]),
        Command::Relate {
            dataset: DatasetRef {
                name: "Visits".into(),
                join: epiora_lang::JoinMode::All,
            },
            keys: vec![("PatientID".into(), "VisitPatientID".into())],
        },
    ]);
    let text = r.canonical_text();
    let expected = "AS-OF 2024-10-04 00:00:00\n\
        SELECT ((AGE > 40) AND (ILL = 1))\n\
        SORT AGE SEX DESCENDING\n\
        DEFINE BIG NUMERIC\n\
        ASSIGN BIG = (AGE * 10)\n\
        SET PROCESS=BOTH\n\
        CANCEL SELECT\n\
        CANCEL SORT\n\
        MERGE Visits PatientID::VisitPatientID UPDATE\n\
        RECODE AGE TO G\n\
        LOVALUE - 17 = 1\n\
        64 = 2\n\
        ELSE = 9\n\
        END\n\
        DELETE (ILL = 0)\n\
        UNDELETE *\n\
        DELETE VARIABLES TMP TMP2\n\
        KEYVARS PatientID\n\
        RELATE Visits PatientID::VisitPatientID ALL";
    assert_eq!(text, expected);
    // The hash covers the canonical text exactly and is deterministic.
    let h1 = r.sha256_hex();
    let h2 = recipe(r.steps.clone()).canonical_text();
    assert_eq!(h2, text);
    assert_eq!(h1.len(), 64);
    assert!(h1
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase()));
    assert_eq!(h1, r.sha256_hex());
}

// ---------- replay determinism ----------

fn any_expr() -> impl Strategy<Value = epiora_lang::Expr> {
    prop::sample::select(vec![
        "AGE > 40",
        "ILL = 1",
        "AGE = (.)",
        "SEX = \"M\"",
        "AGE > 10 OR ILL = 0",
        "AGE + 1",
        "AGE * 2",
        "ILL",
    ])
    .prop_map(|s| parse_expr(s).expect("literal parses"))
}

fn any_command() -> impl Strategy<Value = Command> {
    prop_oneof![
        any_expr().prop_map(Command::Select),
        Just(Command::CancelSelect),
        prop::sample::select(vec![
            vec![("AGE".into(), Order::Asc)],
            vec![("AGE".into(), Order::Desc)],
            vec![("SEX".into(), Order::Asc), ("AGE".into(), Order::Desc)],
        ])
        .prop_map(Command::Sort),
        Just(Command::CancelSort),
        (
            prop::sample::select(vec!["D1", "D2"]),
            prop::sample::select(vec![ValueType::Integer, ValueType::Real, ValueType::Text,])
        )
            .prop_map(|(name, ty)| Command::Define {
                name: name.to_string(),
                ty,
                scope: Scope::Standard,
            }),
        (prop::sample::select(vec!["D1", "D2", "X2"]), any_expr()).prop_map(|(name, e)| {
            Command::Assign {
                name: name.to_string(),
                expr: e,
            }
        }),
        prop::sample::select(vec!["D1", "D2", "X2", "NOPE"])
            .prop_map(|s| Command::Undefine(s.to_string())),
        (prop::sample::select(vec![
            RecodeBound::LoValue,
            RecodeBound::Value(Value::Integer(10)),
        ]))
        .prop_map(|lo| Command::Recode {
            from: "AGE".into(),
            to: "G".into(),
            rules: vec![RecodeRule::Range {
                lo,
                hi: RecodeBound::HiValue,
                to: Value::Integer(1),
            }],
            otherwise: Some(Value::Integer(0)),
        }),
        any_expr().prop_map(Command::DeleteRecords),
        Just(Command::Undelete(None)),
        prop::sample::select(vec![
            vec!["D1".to_string()],
            vec!["G".to_string(), "D2".to_string()],
            vec!["NOPE".to_string()],
        ])
        .prop_map(Command::DeleteVariables),
        prop::sample::select(vec![
            SetOption::Process(Process::Undeleted),
            SetOption::Process(Process::Both),
            SetOption::Missing(true),
            SetOption::Missing(false),
        ])
        .prop_map(Command::Set),
        prop::sample::select(vec![
            vec!["AGE".to_string()],
            vec!["ILL".into(), "AGE".into()],
        ])
        .prop_map(Command::KeyVars),
    ]
}

fn corpus_ds() -> Dataset {
    ds(
        "t",
        vec![
            (
                "AGE",
                ColumnData::Integer(
                    [3, 8, 11, 20, 33, 45, 52, 59, 64, 77]
                        .iter()
                        .map(|x| Some(*x))
                        .collect(),
                ),
            ),
            (
                "ILL",
                ColumnData::Integer(
                    [1, 0, 1, 1, 0, 1, 0, 1, 1, 0]
                        .iter()
                        .map(|x| Some(*x))
                        .collect(),
                ),
            ),
            (
                "SEX",
                ColumnData::Text(
                    ["F", "M", "F", "F", "M", "F", "M", "M", "F", "M"]
                        .iter()
                        .map(|s| Some((*s).to_string()))
                        .collect(),
                ),
            ),
        ],
    )
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(200))]

    /// Random command sequences replay deterministically: two replays
    /// of the same recipe produce the same materialized hash — or the
    /// same typed error (never a panic).
    #[test]
    fn replay_reproduces_view_hash(steps in prop::collection::vec(any_command(), 0..8)) {
        let src = corpus_ds();
        let r = Recipe {
            as_of: as_of(),
            steps,
        };
        let a = replay(&src, &NoLinks, &r);
        let b = replay(&src, &NoLinks, &r);
        match (a, b) {
            (Ok(va), Ok(vb)) => {
                prop_assert_eq!(
                    va.materialize().content_hash_hex(),
                    vb.materialize().content_hash_hex(),
                    "same recipe must materialize the same view"
                );
            }
            (Err(ea), Err(eb)) => prop_assert_eq!(ea.code(), eb.code()),
            (a, b) => prop_assert!(
                false,
                "replay was not deterministic: {a:?} vs {b:?}",
                a = a.map(|v| v.materialize().content_hash_hex()),
                b = b.map(|v| v.materialize().content_hash_hex())
            ),
        }
    }
}

// ---------- typed errors ----------

#[test]
fn replay_errors_are_typed_not_panics() {
    // A command against a missing variable errors, it never panics.
    let src = ds("t", vec![("x", ColumnData::Integer(vec![Some(1)]))]);
    let err = replay(
        &src,
        &NoLinks,
        &recipe(vec![Command::Select(expr("NOPE = 1"))]),
    )
    .expect_err("unknown variable");
    assert_eq!(err.code(), "unknown_variable");

    // A merge against an absent linked dataset is `unknown_variable`.
    let err = replay(
        &src,
        &NoLinks,
        &recipe(vec![Command::Merge {
            dataset: DatasetRef::named("MISSING"),
            keys: vec![("x".into(), "x".into())],
            mode: MergeMode::MatchOrInsert,
        }]),
    )
    .expect_err("missing dataset");
    assert_eq!(err.code(), "unknown_variable");
}
