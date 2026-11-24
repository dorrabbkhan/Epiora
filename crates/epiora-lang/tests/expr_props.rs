//! Task 10's proptests — the canonical `Display` re-parses to the
//! identical AST over the whole grammar, and `eval_column` on a
//! generated expression is a total function (never panics). The
//! `sample()`/`env()` fixtures duplicate `expr.rs`'s so each test
//! file is self-contained.

use epiora_core::dataset::{Column, ColumnData, Dataset, Value};
use epiora_lang::{eval_column, parse_expr, AsOf, EvalEnv, Expr, Lit};
use proptest::prelude::*;

/// A small dataset: `x` has a missing middle cell, `flag` a boolean,
/// `name` text, `d` a date column, `t` a time column, `My Col` a
/// bracketed-only name.
fn sample() -> Dataset {
    Dataset::new(
        "t",
        1,
        vec![
            Column {
                name: "x".into(),
                data: ColumnData::Integer(vec![Some(1), None, Some(3)]),
            },
            Column {
                name: "flag".into(),
                data: ColumnData::Boolean(vec![Some(true), Some(false), None]),
            },
            Column {
                name: "name".into(),
                data: ColumnData::Text(vec![Some("Doe, John".into()), None, Some("SMITH".into())]),
            },
            Column {
                name: "d".into(),
                // 1940-04-19, 2000-01-01, missing — epoch days.
                data: ColumnData::Date(vec![Some(-10839), Some(10957), None]),
            },
            Column {
                name: "t".into(),
                data: ColumnData::Time(vec![Some(8 * 3600), None, Some(22 * 3600)]),
            },
            Column {
                name: "My Col".into(),
                data: ColumnData::Text(vec![Some("a".into()), Some("b".into()), Some("c".into())]),
            },
        ],
    )
    .expect("dataset builds")
}

fn env() -> EvalEnv {
    EvalEnv {
        as_of: AsOf {
            // 2025-11-19 (days since epoch) at 04:30:07.
            date: 20_385,
            seconds: 4 * 3600 + 30 * 60 + 7,
        },
        globals: [
            ("environ:workdir".to_string(), Value::Text("/tmp".into())),
            ("exists:workdir".to_string(), Value::Boolean(true)),
            (
                "filedate:workdir".to_string(),
                Value::DateTime(1_745_000_000),
            ),
            ("pi".to_string(), Value::Real(3.25)),
        ]
        .into_iter()
        .collect(),
    }
}

proptest! {
    /// The canonical `Display` re-parses to the identical AST —
    /// exercised over the whole grammar (operators, literals, calls,
    /// bracketed names, LIKE, expr lists).
    #[test]
    fn parse_print_roundtrip(e in any_expr()) {
        let printed = e.to_string();
        let reparsed = parse_expr(&printed)
            .unwrap_or_else(|err| panic!("{printed}: {err}"));
        prop_assert_eq!(reparsed, e);
    }

    /// `eval_column` on a generated expression over the fixture
    /// dataset never panics — errors are `LangError`s, missingness
    /// is `None`, everything else is a cell.
    #[test]
    fn eval_never_panics(e in any_expr(), seed in any::<u64>()) {
        let ds = sample();
        let mut env = env();
        env.as_of = AsOf {
            date: 10_000 + (seed % 20_000) as i32,
            seconds: (seed % 86_400) as i32,
        };
        let _ = eval_column(&e, &ds, &env);
    }
}

/// An arbitrary `Expr` over the printable grammar — literals stay in
/// the grammar's own ranges so `Display` round-trips.
fn any_expr() -> impl Strategy<Value = Expr> {
    let leaf = prop_oneof![
        4 => ident_strategy(),
        3 => lit_strategy().prop_map(Expr::Literal),
        2 => prop::sample::select(vec!["SYSTEMDATE", "SYSTEMTIME", "RECORDCOUNT"])
            .prop_map(|n| Expr::Call {
                name: n.to_string(),
                args: Vec::new(),
            }),
    ];
    leaf.prop_recursive(4, 48, 8, |inner| {
        prop_oneof![
            4 => (unop(), inner.clone()).prop_map(|(op, e)| Expr::Unary {
                op,
                operand: Box::new(e),
            }),
            8 => (binop(), inner.clone(), inner.clone()).prop_map(|(op, a, b)| Expr::Binary {
                op,
                lhs: Box::new(a),
                rhs: Box::new(b),
            }),
            2 => (inner.clone(), like_pattern()).prop_map(|(e, p)| Expr::Like {
                operand: Box::new(e),
                pattern: p,
            }),
            3 => (func_name(), prop::collection::vec(inner.clone(), 0..=3)).prop_map(
                |(name, args)| Expr::Call { name, args }
            ),
            1 => prop::collection::vec(inner.clone(), 2..=3).prop_map(Expr::List),
        ]
    })
}

fn unop() -> impl Strategy<Value = epiora_lang::UnOp> {
    prop::sample::select(vec![
        epiora_lang::UnOp::Neg,
        epiora_lang::UnOp::Pos,
        epiora_lang::UnOp::Not,
    ])
}

fn binop() -> impl Strategy<Value = epiora_lang::BinOp> {
    use epiora_lang::BinOp::*;
    prop::sample::select(vec![
        Or, Xor, And, Eq, Ne, Lt, Le, Gt, Ge, Concat, Add, Sub, Mul, Div, Mod, Pow,
    ])
}

fn ident_strategy() -> impl Strategy<Value = Expr> {
    prop::sample::select(vec![
        "x",
        "flag",
        "name",
        "d",
        "t",
        "age",
        "ILL",
        "My Col",
        "weird var",
    ])
    .prop_map(|s| Expr::Ident(s.to_string()))
}

fn lit_strategy() -> impl Strategy<Value = Lit> {
    prop_oneof![
        Just(Lit::Missing),
        prop::bool::ANY.prop_map(Lit::Boolean),
        // Non-negative only: a leading `-` parses as `NegateExp`, so a
        // negative literal is not a printable literal.
        (0i64..1_000_000).prop_map(Lit::Integer),
        // |x| < 1e15 keeps `Display` inside `RealLiteral`'s shape.
        (0.0f64..1e15).prop_map(Lit::Real),
        safe_text().prop_map(Lit::Text),
        (-700_000i32..=2_900_000).prop_map(Lit::Date),
        (0i32..=86_399).prop_map(Lit::Time),
        (-1_000_000_000i64..2_000_000_000).prop_map(Lit::DateTime),
    ]
}

fn safe_text() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::char::range('a', 'z'), 0..=12)
        .prop_map(|cs| cs.into_iter().collect())
}

fn like_pattern() -> impl Strategy<Value = String> {
    prop::collection::vec(prop::char::range('a', 'z'), 1..=6).prop_map(|cs| {
        let mut s: String = cs.into_iter().collect();
        if !s.is_empty() {
            s.push('*');
        }
        s
    })
}

fn func_name() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "ABS",
        "SQRT",
        "ROUND",
        "DAY",
        "DAYS",
        "MONTHS",
        "SUBSTRING",
        "UPPERCASE",
        "TXTTONUM",
        "FINDTEXT",
        "RND",
        "STEP",
        "LN",
        "NUMTODATE",
        "STRLEN",
        "LAGVALUE",
        "GROUPROWINDEX",
        "EPIWEEK",
        "HOUR",
        "NUMTOTIME",
    ])
    .prop_map(str::to_string)
}
