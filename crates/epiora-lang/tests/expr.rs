//! Task 10's named tests — the expression language's contract with
//! the recipe system: every `supported@1.1` register function parses
//! and evaluates, missing propagates the way Epi Info 7.2.7.0
//! propagates it (as pinned by `fixtures/ei7/v11/*`), bracketed names
//! resolve, `SYSTEMDATE`/`SYSTEMTIME` read only the as-of, type errors
//! name the column, and the two proptests cover the printer and the
//! evaluator's total function.

use epiora_core::dataset::{Column, ColumnData, Dataset, Value};
use epiora_lang::{
    eval_column, parse_expr, type_check, AsOf, EvalEnv, Expr, LangError, Lit, Schema,
};
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

/// Every register function that is `supported` at `1.1` parses as a
/// call and evaluates over `sample()` — `PFROMZ`/`ZSCORE` evaluate to
/// EI7's own "no longer supported" error, everything else to a column.
#[test]
fn every_register_function_parses_and_evaluates() {
    let register: serde_json::Value =
        serde_json::from_str(include_str!("../register.json")).expect("register parses");
    let names: Vec<&str> = register["functions"]
        .as_array()
        .expect("functions array")
        .iter()
        .filter(|f| f["status"] == "supported" && f["version"] == "1.1")
        .map(|f| f["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names.len(), 46, "the register's 1.1 surface");

    // One valid call per function; argument shapes the corpus and the
    // grammar's `<FunctionParameterList>` accept.
    let call = |name: &str| -> String {
        match name {
            "RECORDCOUNT" | "SYSTEMDATE" | "SYSTEMTIME" => name.to_string(),
            "GROUPROWINDEX" | "LAGVALUE" => format!("{name}(x)"),
            "ENVIRON" | "EXISTS" | "FILEDATE" => format!("{name}(\"workdir\")"),
            "FORMAT" => format!("{name}(54, \"0.00\")"),
            "DAYS" | "MONTHS" | "YEARS" | "HOURS" | "MINUTES" | "SECONDS" => {
                format!("{name}(d, d)")
            }
            "DAY" | "MONTH" | "YEAR" | "EPIWEEK" | "TXTTODATE" => format!("{name}(d)"),
            "HOUR" | "MINUTE" | "SECOND" => format!("{name}(t)"),
            "NUMTODATE" => "NUMTODATE(1940, 4, 19)".to_string(),
            "NUMTOTIME" => "NUMTOTIME(18, 30, 0)".to_string(),
            "SUBSTRING" => "SUBSTRING(name, 1, 3)".to_string(),
            "FINDTEXT" => "FINDTEXT(\"John\", name)".to_string(),
            "STRLEN" | "UPPERCASE" | "TXTTONUM" => format!("{name}(name)"),
            "ROUND" => "ROUND(x, 0)".to_string(),
            "TRUNC" => "TRUNC(-7.5)".to_string(),
            "RND" => "RND(1, 10)".to_string(),
            "STEP" => "STEP(x, 2)".to_string(),
            "POISSONLCL" | "POISSONUCL" => format!("{name}(5, 95)"),
            "PFROMZ" => "PFROMZ(1.96)".to_string(),
            "ZSCORE" => "ZSCORE(x, 2, 1)".to_string(),
            _ => format!("{name}(x)"),
        }
    };

    let ds = sample();
    for name in names {
        let source = call(name);
        let expr = parse_expr(&source).unwrap_or_else(|e| panic!("{source}: {e}"));
        let result = eval_column(&expr, &ds, &env());
        match name {
            "PFROMZ" | "ZSCORE" => {
                let err = result.expect_err(&format!("{source} must be unsupported"));
                assert_eq!(err.code(), "unsupported_function", "{source}");
            }
            _ => {
                let column = result.unwrap_or_else(|e| panic!("{source}: {e}"));
                assert_eq!(column.len(), ds.row_count(), "{source}");
            }
        }
    }
}

/// The v1.1 corpus pins missing-value behaviour in `select_sort`
/// (a 75-row table with one missing cell: `<> 1` selects 38, `= 1`
/// selects 37, `= (.)` selects 1) and in `functions_text_math` /
/// `functions_date` (missing propagates through arithmetic and the
/// `TryParse`/`is`-typed functions).
#[test]
fn missing_propagates_as_epi_info() {
    // MASHEDPOTA: 37 zeros, 37 ones, one missing — Oswego12 row 75.
    let mut cells = vec![Some(0); 37];
    cells.extend([Some(1); 37]);
    cells.push(None);
    let ds = Dataset::new(
        "t",
        1,
        vec![Column {
            name: "mashedpota".into(),
            data: ColumnData::Integer(cells),
        }],
    )
    .unwrap();
    let selected = |source: &str| -> usize {
        let e = parse_expr(source).unwrap();
        match eval_column(&e, &ds, &env()).unwrap() {
            ColumnData::Boolean(v) => v.iter().filter(|c| **c == Some(true)).count(),
            other => panic!("{source} yielded {other:?}"),
        }
    };
    assert_eq!(selected("mashedpota = 1"), 37, "select_sort.json:11");
    assert_eq!(selected("mashedpota <> 1"), 38, "select_sort.json:14");
    assert_eq!(selected("mashedpota = (.)"), 1, "select_sort.json:17");
    assert_eq!(selected("mashedpota <> (.)"), 74);
    assert_eq!(selected("(.) = (.)"), 75);
    assert_eq!(selected("(.) <> (.)"), 0);

    // Arithmetic with a missing operand is missing (the BIGAGE pin).
    let ds = sample();
    let present = |source: &str, ds: &Dataset| -> Vec<Option<String>> {
        let e = parse_expr(source).unwrap();
        match eval_column(&e, ds, &env()).unwrap() {
            ColumnData::Text(v) => v,
            ColumnData::Integer(v) => v.iter().map(|x| x.map(|i| i.to_string())).collect(),
            ColumnData::Real(v) => v.iter().map(|x| x.map(|r| r.to_string())).collect(),
            ColumnData::Boolean(v) => v.iter().map(|x| x.map(|b| b.to_string())).collect(),
            ColumnData::Date(v) => v.iter().map(|x| x.map(|d| d.to_string())).collect(),
            ColumnData::Time(v) => v.iter().map(|x| x.map(|t| t.to_string())).collect(),
            ColumnData::DateTime(v) => v.iter().map(|x| x.map(|s| s.to_string())).collect(),
        }
    };
    assert_eq!(
        present("x + 1", &ds),
        vec![Some("2".into()), None, Some("4".into())],
        "missing + 1 is missing"
    );
    // `0 - missing` then `ABS` stays missing (functions_text_math ABSZ).
    assert_eq!(present("abs(0 - x)", &ds)[1], None);
    // The scalar functions' `TryParse` path misses on `""` text too.
    assert_eq!(present("round(x, 0)", &ds)[1], None);
    assert_eq!(present("uppercase(name)", &ds)[1], None);
    // `LAGVALUE` on the first row is missing.
    assert_eq!(present("lagvalue(x)", &ds)[0], None);
    // `DAYS` on a missing date cell.
    assert_eq!(present("days(d, d)", &ds)[2], None);
}

/// `[bracketed]` names parse (with spaces and keywords inside), and a
/// qualified `a.b` name parses as one identifier.
#[test]
fn bracketed_names_parse() {
    for (source, want) in [
        ("[My Col]", "My Col"),
        ("[a b]", "a b"),
        ("[AND]", "AND"),
        ("[a].[b]", "a.b"),
        ("[a].b", "a.b"),
        ("x.[b]", "x.b"),
    ] {
        match parse_expr(source).unwrap_or_else(|e| panic!("{source}: {e}")) {
            Expr::Ident(name) => assert_eq!(name, want, "{source}"),
            other => panic!("{source} parsed as {other:?}"),
        }
    }
    // A bracketed name resolves its column case-insensitively.
    let ds = sample();
    let e = parse_expr("[MY COL]").unwrap();
    match eval_column(&e, &ds, &env()).unwrap() {
        ColumnData::Text(v) => assert_eq!(v[0].as_deref(), Some("a")),
        other => panic!("[MY COL] yielded {other:?}"),
    }
    // An unclosed bracket is a syntax error.
    assert_eq!(parse_expr("[oops").unwrap_err().code(), "pgm_syntax");
}

/// `SYSTEMDATE`/`SYSTEMTIME` read `EvalEnv::as_of` — never a clock:
/// two different envs give two different columns, deterministically.
#[test]
fn systemdate_reads_as_of_only() {
    let ds = sample();
    let e = parse_expr("systemdate").unwrap();
    let mut e2 = env();
    e2.as_of.date += 30;
    match (
        eval_column(&e, &ds, &env()).unwrap(),
        eval_column(&e, &ds, &e2).unwrap(),
    ) {
        (ColumnData::Date(a), ColumnData::Date(b)) => {
            assert_eq!(a[0], Some(20_385));
            assert_eq!(b[0], Some(20_415));
        }
        other => panic!("systemdate yielded {other:?}"),
    }
    let e = parse_expr("SYSTEMTIME").unwrap();
    match eval_column(&e, &ds, &env()).unwrap() {
        ColumnData::Time(v) => assert_eq!(v[0], Some(4 * 3600 + 30 * 60 + 7)),
        other => panic!("systemtime yielded {other:?}"),
    }
}

/// A `type_check` failure names the offending column (the recipe
/// system underlines it); arity and unknown names are typed too.
#[test]
fn type_errors_name_the_column() {
    let schema = Schema::from_dataset(&sample());
    // Text beside `+` — `column` is the variable name.
    let e = parse_expr("name + 1").unwrap();
    match type_check(&e, &schema) {
        Err(LangError::ExprType { column, .. }) => assert_eq!(column, "name"),
        other => panic!("name + 1: {other:?}"),
    }
    // `Time + n` — a `TimeSpan` never adds in EI7.
    let e = parse_expr("t + 1").unwrap();
    match type_check(&e, &schema) {
        Err(LangError::ExprType { column, .. }) => assert_eq!(column, "t"),
        other => panic!("t + 1: {other:?}"),
    }
    // An unknown name is `unknown_variable` naming it.
    let e = parse_expr("nosuch + 1").unwrap();
    match type_check(&e, &schema) {
        Err(LangError::UnknownVariable { name }) => assert_eq!(name, "nosuch"),
        other => panic!("nosuch + 1: {other:?}"),
    }
    // A bad arity names the function.
    let e = parse_expr("days(d)").unwrap();
    match type_check(&e, &schema) {
        Err(LangError::ExprType { column, .. }) => assert_eq!(column, "DAYS"),
        other => panic!("days(d): {other:?}"),
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
