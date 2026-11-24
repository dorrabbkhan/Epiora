//! Recipes and receipt v4 (v1.1 plan Task 11): canonical text
//! stability, replay determinism over random command sequences, and
//! the receipt contract — v4 exactly when a `working_view` stamp is
//! present, v1–v3 byte-unchanged.

use epiora_core::dataset::{Column, ColumnData, Dataset, Value, ValueType};
use epiora_core::procedures::{freq, FreqOptions, RunContext};
use epiora_core::receipt::{
    canonical_json, CalculatorStamp, EngineStamp, ProjectStamp, WorkingViewStamp,
};
use epiora_lang::{
    parse_expr, replay, working_view_stamp, AsOf, Command, DatasetRef, LinkedDatasets, MergeMode,
    Order, Process, Recipe, RecodeBound, RecodeRule, Scope, SetOption,
};
use proptest::prelude::*;
use serde_json::json;
use std::path::Path;

fn col(name: &str, data: ColumnData) -> Column {
    Column {
        name: name.to_string(),
        data,
    }
}

fn ds(id: &str, cols: Vec<(&str, ColumnData)>) -> Dataset {
    Dataset::new(id, 1, cols.into_iter().map(|(n, d)| col(n, d)).collect()).expect("dataset")
}

fn expr(text: &str) -> epiora_lang::Expr {
    parse_expr(text).expect("expression parses")
}

fn as_of() -> AsOf {
    AsOf {
        date: 20_000,
        seconds: 0,
    }
}

fn recipe(steps: Vec<Command>) -> Recipe {
    Recipe {
        as_of: as_of(),
        steps,
    }
}

struct NoLinks;
impl LinkedDatasets for NoLinks {
    fn get(&self, _name: &str) -> Option<&Dataset> {
        None
    }
    fn link(&self, _parent: &str, _child: &str) -> Option<&epiora_core::dataset::DatasetLink> {
        None
    }
}

fn ctx(working_view: Option<WorkingViewStamp>) -> RunContext {
    RunContext {
        timestamp: "2026-10-30T12:00:00Z".into(),
        project: None,
        working_view,
    }
}

fn freq_opts(variable: &str) -> FreqOptions {
    FreqOptions {
        variables: vec![variable.into()],
        stratify_by: None,
        include_missing: false,
        ci_method: Default::default(),
    }
}

/// The pinned engine stamp every receipt golden uses.
fn pin_engine(mut receipt: epiora_core::receipt::Receipt) -> epiora_core::receipt::Receipt {
    receipt.engine = EngineStamp {
        name: "epiora-core".into(),
        version: "1.0.0".into(),
        git_sha: "0000000".into(),
    };
    receipt.receipt_sha256 = receipt.sha256_hex();
    receipt
}

fn canon(receipt: &epiora_core::receipt::Receipt) -> String {
    canonical_json(&serde_json::to_value(receipt).expect("receipt serializes"))
}

fn golden(name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../epiora-core/tests/golden")
            .join(name),
    )
    .unwrap_or_else(|_| panic!("golden {name}"))
    .trim_end()
    .to_string()
}

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
            vec!["ILL".to_string(), "AGE".to_string()],
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

// ---------- receipt v4 ----------

fn golden_ds() -> Dataset {
    ds(
        "golden",
        vec![
            (
                "E",
                ColumnData::Boolean(vec![
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(true),
                    Some(false),
                    Some(false),
                    Some(false),
                    Some(false),
                ]),
            ),
            (
                "I",
                ColumnData::Boolean(vec![
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(false),
                    Some(true),
                    Some(true),
                    Some(false),
                    Some(false),
                ]),
            ),
            (
                "S",
                ColumnData::Text(vec![
                    Some("x".into()),
                    Some("x".into()),
                    Some("y".into()),
                    Some("y".into()),
                    Some("x".into()),
                    Some("x".into()),
                    Some("y".into()),
                    Some("y".into()),
                ]),
            ),
        ],
    )
}

#[test]
fn receipt_v4_only_with_steps() {
    let src = golden_ds();
    let r = recipe(vec![Command::Select(expr("E = (+)"))]);
    let view = replay(&src, &NoLinks, &r).expect("replay");
    let mat = view.materialize();

    // The stamp helper yields None for a step-less recipe — a session
    // receipt only becomes v4 once cleaning commands ran.
    assert!(working_view_stamp(&src, &mat, &recipe(vec![])).is_none());
    let st = working_view_stamp(&src, &mat, &r).expect("steps stamp the view");
    assert_eq!(st.recipe_text, r.canonical_text());
    assert_eq!(st.recipe_sha256, r.sha256_hex());
    assert_eq!(st.view_sha256, mat.content_hash_hex());

    // A run on the materialized view carries receipt version 4.
    let out = freq(&mat, &freq_opts("E"), &ctx(Some(st.clone()))).expect("freq");
    assert_eq!(out.receipt.receipt_version, 4);
    let value = serde_json::to_value(&out.receipt).expect("serialize");
    let wv = &value["working_view"];
    assert_eq!(wv["recipe_sha256"].as_str().unwrap(), st.recipe_sha256);
    assert_eq!(wv["view_sha256"].as_str().unwrap(), st.view_sha256);
    assert_eq!(
        wv["source"]["sha256"].as_str().unwrap(),
        src.content_hash_hex()
    );

    // Without a stamp the same run stays v1 — and serializes no key.
    let out = freq(&mat, &freq_opts("E"), &ctx(None)).expect("freq");
    assert_eq!(out.receipt.receipt_version, 1);
    let value = serde_json::to_value(&out.receipt).expect("serialize");
    assert!(value.get("working_view").is_none());
}

#[test]
fn receipts_v1_v2_v3_golden_unchanged() {
    use epiora_core::procedures::{tables, TablesOptions, VarSpec};

    let ds = golden_ds();
    let opts = TablesOptions {
        exposure: VarSpec {
            name: "E".into(),
            levels: None,
        },
        outcome: VarSpec {
            name: "I".into(),
            levels: None,
        },
        stratify_by: vec!["S".into()],
        include_missing: false,
    };

    // v1 — the pre-existing golden pins the canonical bytes.
    let r = pin_engine(tables(&ds, &opts, &ctx(None)).expect("tables").receipt);
    assert_eq!(r.receipt_version, 1);
    assert_eq!(canon(&r), golden("receipt_v1.json"));

    // v2 — a project stamp and no working view stays v2 and emits no
    // `working_view` key (the v4 field must not leak into old shapes).
    let mut c = ctx(None);
    c.project = Some(ProjectStamp {
        id: "p1".into(),
        name: "P".into(),
    });
    let r = pin_engine(tables(&ds, &opts, &c).expect("tables").receipt);
    assert_eq!(r.receipt_version, 2);
    let v = serde_json::to_value(&r).expect("serialize");
    assert!(v.get("working_view").is_none());
    assert_eq!(v["project"]["id"].as_str().expect("project id"), "p1");

    // v3 — a calculator receipt (no dataset, no working view).
    let inputs = json!({"numerator": 3, "denominator": 5});
    let r = pin_engine(epiora_core::receipt::Receipt::for_calculator(
        CalculatorStamp {
            id: "proportion".into(),
            inputs: inputs.clone(),
        },
        "calc.proportion".into(),
        inputs,
        vec![],
        "2026-10-30T12:00:00Z".into(),
        EngineStamp {
            name: "epiora-core".into(),
            version: "1.0.0".into(),
            git_sha: "0000000".into(),
        },
    ));
    assert_eq!(r.receipt_version, 3);
    let v = serde_json::to_value(&r).expect("serialize");
    assert!(v.get("working_view").is_none());
    assert!(v.get("dataset").is_none());
}

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
