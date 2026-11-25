//! `print_parse_roundtrip` — `print_command` output re-parses to the
//! identical `Command` over every variant (spec §6.1's canonical
//! spelling; the recipe's `canonical_text` uses the same printer).

use epiora_core::dataset::{Value, ValueType};
use epiora_lang::{
    parse_expr, parse_program, print_command, AggFunc, Aggregate, Command, DatasetRef, Expr,
    JoinMode, MergeMode, Order, Process, RecodeBound, RecodeRule, Scope, SetOption, Statement,
    StatisticsLevel,
};
use proptest::prelude::*;

proptest! {
    /// print → parse → identical command, for every `Command`
    /// variant (DefineExpr included — Task 11's addition).
    #[test]
    fn print_parse_roundtrip(c in any_command()) {
        let printed = print_command(&c);
        let p = parse_program(&printed)
            .unwrap_or_else(|e| panic!("{printed:?}: {e}"));
        prop_assert_eq!(p.statements.len(), 1);
        match &p.statements[0].node {
            Statement::Manage(parsed) => prop_assert_eq!(parsed, &c),
            other => panic!("{printed:?} → {other:?}"),
        }
    }
}

/// Identifier-safe names (uppercase letters and digits; never a
/// keyword).
fn name() -> impl Strategy<Value = String> {
    prop::sample::select(vec!["ZA", "ZB", "ZC", "ZD", "QA", "QQ", "V9"]).prop_map(str::to_string)
}

fn names(max: usize) -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(name(), 1..=max)
}

/// A `DatasetRef` as printed: `t` or `f:t` — `Merge` gets
/// `Matching` join only (the MERGE line carries no join word).
fn dref() -> impl Strategy<Value = DatasetRef> {
    (
        prop::sample::select(vec!["T2", "F:T2"]).prop_map(str::to_string),
        prop::sample::select(vec![JoinMode::Matching, JoinMode::All]),
    )
        .prop_map(|(name, join)| DatasetRef { name, join })
}

/// Expressions from a pinned source set — each already proves
/// `parse(print(e)) = e` in `expr_props`.
fn expr() -> impl Strategy<Value = Expr> {
    prop::sample::select(vec![
        "AGE",
        "X",
        "AGE + 1",
        "AGE * 2 - X",
        "AGE > 10 AND ILL = 1",
        "AGE = (.)",
        "NOT ILL",
        "(+)",
        "5",
        "3.5",
        "\"txt\"",
        "UPPERCASE(NAME)",
        "AGE MOD 3",
        "12:30:00",
        "10/25/2005",
        "AGE & \"x\"",
        "STRLEN(NAME) <= 2",
        "X OR AGE",
    ])
    .prop_map(|s| parse_expr(s).expect("expr"))
}

/// RECODE literal values — every form `value_text` prints. Dates
/// stay in year ≥ 100 (`days ≥ -683003`): a civil year < 100 prints
/// with ≤ 2 significant digits and re-parses through the 2-digit
/// year pivot (`99` → 1999), so no spelling can round-trip it —
/// EI7's `DateTime.TryParse` pivot is two-way lossy there.
fn value() -> impl Strategy<Value = Value> {
    prop_oneof![
        (-100i64..=100).prop_map(Value::Integer),
        (0i64..100).prop_map(|n| Value::Real(n as f64 + 0.5)),
        prop::collection::vec(prop::char::range('a', 'z'), 0..=8)
            .prop_map(|cs| Value::Text(cs.into_iter().collect())),
        prop::bool::ANY.prop_map(Value::Boolean),
        (-683_003i32..=2_900_000).prop_map(Value::Date),
        (0i32..=86_399).prop_map(Value::Time),
        (-1_000_000_000i64..=2_000_000_000).prop_map(Value::DateTime),
    ]
}

fn bound() -> impl Strategy<Value = RecodeBound> {
    prop_oneof![
        Just(RecodeBound::LoValue),
        Just(RecodeBound::HiValue),
        value().prop_map(RecodeBound::Value),
    ]
}

fn recode_rule() -> impl Strategy<Value = RecodeRule> {
    prop_oneof![
        (value(), value()).prop_map(|(from, to)| RecodeRule::Single { from, to }),
        (bound(), bound(), value()).prop_map(|(lo, hi, to)| RecodeRule::Range { lo, hi, to }),
    ]
}

fn scope() -> impl Strategy<Value = Scope> {
    prop::sample::select(vec![Scope::Standard, Scope::Global, Scope::Permanent])
}

fn order() -> impl Strategy<Value = Order> {
    prop::sample::select(vec![Order::Asc, Order::Desc])
}

fn set_option() -> impl Strategy<Value = SetOption> {
    prop_oneof![
        prop::sample::select(vec![Process::Undeleted, Process::Both, Process::Deleted])
            .prop_map(SetOption::Process),
        prop::bool::ANY.prop_map(SetOption::Missing),
        prop::bool::ANY.prop_map(SetOption::Deleted),
        prop::sample::select(vec![
            StatisticsLevel::None,
            StatisticsLevel::Minimal,
            StatisticsLevel::Intermediate,
            StatisticsLevel::Complete,
        ])
        .prop_map(SetOption::Statistics),
        prop::bool::ANY.prop_map(SetOption::Percents),
        prop::bool::ANY.prop_map(SetOption::Ignore),
        prop::bool::ANY.prop_map(SetOption::Select),
    ]
}

fn ty() -> impl Strategy<Value = ValueType> {
    // NUMERIC maps to Real (EI7's Number is float) — Integer has no
    // DEFINE word.
    prop::sample::select(vec![
        ValueType::Real,
        ValueType::Text,
        ValueType::Boolean,
        ValueType::Date,
        ValueType::Time,
        ValueType::DateTime,
    ])
}

fn aggregate() -> impl Strategy<Value = Aggregate> {
    (name(), agg_func(), arg()).prop_map(|(name, func, arg)| Aggregate {
        name,
        func,
        // `Count` takes no argument; every other verb does.
        arg: if func == AggFunc::Count {
            None
        } else {
            Some(arg)
        },
    })
}

fn agg_func() -> impl Strategy<Value = AggFunc> {
    prop::sample::select(vec![
        AggFunc::Count,
        AggFunc::Sum,
        AggFunc::Avg,
        AggFunc::Min,
        AggFunc::Max,
        AggFunc::First,
        AggFunc::Last,
        AggFunc::Stdev,
        AggFunc::Variance,
    ])
}

fn arg() -> impl Strategy<Value = String> {
    name()
}

/// Commands allowed inside an IF block (no nesting needed to
/// exercise round-trip).
fn leaf_command() -> impl Strategy<Value = Command> {
    prop_oneof![
        (name(), expr()).prop_map(|(name, expr)| Command::Assign { name, expr }),
        expr().prop_map(Command::Select),
        (name(), ty(), scope()).prop_map(|(name, ty, scope)| Command::Define { name, ty, scope }),
        name().prop_map(Command::Undefine),
        set_option().prop_map(Command::Set),
        Just(Command::CancelSelect),
        Just(Command::CancelSort),
    ]
}

fn any_command() -> impl Strategy<Value = Command> {
    prop_oneof![
        expr().prop_map(Command::Select),
        Just(Command::CancelSelect),
        prop::collection::vec((name(), order()), 1..=3).prop_map(Command::Sort),
        Just(Command::CancelSort),
        (name(), ty(), scope()).prop_map(|(name, ty, scope)| Command::Define { name, ty, scope }),
        (name(), expr(), scope()).prop_map(|(name, expr, scope)| Command::DefineExpr {
            name,
            expr,
            scope,
        }),
        name().prop_map(Command::Undefine),
        (name(), expr()).prop_map(|(name, expr)| Command::Assign { name, expr }),
        (
            expr(),
            prop::collection::vec(leaf_command(), 0..=2),
            prop::collection::vec(leaf_command(), 0..=2),
        )
            .prop_map(|(cond, then, otherwise)| Command::If {
                cond,
                then,
                otherwise,
            }),
        (
            name(),
            name(),
            prop::collection::vec(recode_rule(), 0..=3),
            prop::option::of(value()),
        )
            .prop_map(|(from, to, rules, otherwise)| Command::Recode {
                from,
                to,
                rules,
                otherwise,
            }),
        (
            merge_dref(),
            prop::collection::vec((name(), name()), 1..=3),
            prop::sample::select(vec![
                MergeMode::MatchOrInsert,
                MergeMode::Update,
                MergeMode::Append,
            ]),
        )
            .prop_map(|(dataset, keys, mode)| Command::Merge {
                dataset,
                keys,
                mode,
            }),
        (dref(), prop::collection::vec((name(), name()), 0..=3))
            .prop_map(|(dataset, keys)| Command::Relate { dataset, keys }),
        expr().prop_map(Command::DeleteRecords),
        prop::option::of(expr()).prop_map(Command::Undelete),
        names(3).prop_map(Command::DeleteVariables),
        names(3).prop_map(Command::KeyVars),
        set_option().prop_map(Command::Set),
        (
            prop::collection::vec(aggregate(), 1..=3),
            prop::collection::vec(name(), 0..=2),
            prop::sample::select(vec!["T2", "F:T2"]).prop_map(str::to_string),
        )
            .prop_map(|(aggregates, by, into)| Command::Summarize {
                aggregates,
                by,
                into: DatasetRef::named(&into),
            }),
    ]
}

fn merge_dref() -> impl Strategy<Value = DatasetRef> {
    prop::sample::select(vec!["T2", "F:T2"]).prop_map(DatasetRef::named)
}
