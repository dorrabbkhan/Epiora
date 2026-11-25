//! The cleaning commands (v1.1 spec §5.4, plan Task 11): the
//! Analysis-grammar verbs that mutate a [`WorkingView`] — SELECT /
//! SORT / CANCEL, DEFINE / ASSIGN / UNDEFINE / IF, RECODE, MERGE,
//! RELATE, DELETE / UNDELETE, KEYVARS, SET and SUMMARIZE. Each
//! [`Command`] is a recipe step; `apply` executes one against the
//! view and the session's [`LinkedDatasets`].

mod define_assign;
mod delete;
mod merge;
mod recode;
mod relate;
mod select_sort;
mod set;
mod summarize;

pub(crate) use define_assign::{assign, define, undefine};
pub(crate) use delete::{delete_records, delete_variables, key_vars, undelete};
pub(crate) use merge::merge;
pub(crate) use recode::recode;
pub(crate) use relate::relate;
pub(crate) use select_sort::{cancel_select, cancel_sort, eval_mask, select, sort};
pub(crate) use set::set;
pub(crate) use summarize::summarize;

use crate::expr::ast::Expr;
use crate::expr::eval::EvalEnv;
use crate::view::{DatasetRef, JoinMode, Order, Process, StatisticsLevel, WorkingView};
use crate::LangError;
use epiora_core::dataset::{Value, ValueType};
use serde::{Deserialize, Serialize};

/// A variable's declared scope (`DEFINE` scope word).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    /// View-scoped; dies with the session's view.
    #[default]
    Standard,
    /// Session-scoped global.
    Global,
    /// Persists on the project — enters every session's recipe
    /// (spec §6.1).
    Permanent,
}

/// `MERGE`'s write mode — the grammar's option word (bare =
/// match-or-insert; `UPDATE`/`APPEND` crash EI7 7.2.7.0's
/// `SetMergeOpt`, so only the bare form is corpus-pinned — owner
/// decision 2026-11-23).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeMode {
    /// Update matched destination rows, insert the rest.
    #[default]
    MatchOrInsert,
    /// Update matched rows only.
    Update,
    /// Append every source row, matches or not.
    Append,
}

/// A RECODE bound: `LOVALUE`/`HIVALUE` or a literal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecodeBound {
    /// The lowest literal — matches any value below the range start.
    LoValue,
    /// The highest literal.
    HiValue,
    /// A literal bound (inclusive).
    Value(Value),
}

/// One RECODE rule line.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecodeRule {
    /// `<lit> = <lit>` — a single source value.
    Single {
        /// The source value.
        from: Value,
        /// The target value.
        to: Value,
    },
    /// `<bound> - <bound> = <lit>` — an inclusive range.
    Range {
        /// Inclusive lower bound (`LOVALUE` = unbounded below).
        lo: RecodeBound,
        /// Inclusive upper bound (`HIVALUE` = unbounded above).
        hi: RecodeBound,
        /// The target value.
        to: Value,
    },
}

/// A SUMMARIZE aggregate — `Ident::Agg(arg)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Aggregate {
    /// Output column name.
    pub name: String,
    /// The aggregation.
    pub func: AggFunc,
    /// The argument column (`Count` takes none).
    pub arg: Option<String>,
}

/// The SUMMARIZE aggregation verbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AggFunc {
    /// Row count.
    Count,
    /// Sum (Real).
    Sum,
    /// Mean (Real).
    Avg,
    /// Minimum (argument's type).
    Min,
    /// Maximum (argument's type).
    Max,
    /// First row's value.
    First,
    /// Last row's value.
    Last,
    /// Sample standard deviation (Real).
    Stdev,
    /// Sample variance (Real).
    Variance,
}

/// One `SET` clause.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SetOption {
    /// `PROCESS=UNDELETED|BOTH|DELETED`.
    Process(Process),
    /// `MISSING=ON|OFF`.
    Missing(bool),
    /// `DELETED=ON|OFF`.
    Deleted(bool),
    /// `STATISTICS=NONE|MINIMAL|INTERMEDIATE|COMPLETE`.
    Statistics(StatisticsLevel),
    /// `PERCENTS=ON|OFF`.
    Percents(bool),
    /// `IGNORE=ON|OFF`.
    Ignore(bool),
    /// `SELECT=ON|OFF`.
    Select(bool),
}

/// One recipe step — the Interfaces block's `Command` plus
/// [`Command::DefineExpr`] for `DEFINE x = <expr>` (EI7 defines a
/// zero-filled variable of the expression's type, pinned by
/// define_assign_if's `MEANS DOUBLED` — decisions.md).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Command {
    /// `SELECT <expr>` — AND-combines with earlier selects.
    Select(Expr),
    /// `CANCEL SELECT`.
    CancelSelect,
    /// `SORT <key> [ASCENDING|DESCENDING] ...`.
    Sort(Vec<(String, Order)>),
    /// `CANCEL SORT`.
    CancelSort,
    /// `DEFINE <name> <type> [<scope>]` — an empty typed column.
    Define {
        /// Variable name.
        name: String,
        /// Declared type.
        ty: ValueType,
        /// Declared scope.
        scope: Scope,
    },
    /// `DEFINE <name> = <expr>` — a zero-filled column of the
    /// expression's type (see the enum's doc).
    DefineExpr {
        /// Variable name.
        name: String,
        /// The declared expression (evaluated only for its type).
        expr: Expr,
        /// Declared scope.
        scope: Scope,
    },
    /// `UNDEFINE <name>`.
    Undefine(String),
    /// `ASSIGN <name> = <expr>`.
    Assign {
        /// Target column.
        name: String,
        /// The expression evaluated per row.
        expr: Expr,
    },
    /// `IF <expr> THEN <assigns> [ELSE <assigns>] END`.
    If {
        /// Per-row condition.
        cond: Expr,
        /// Commands for true rows (ASSIGNs in the corpus).
        then: Vec<Command>,
        /// Commands for false rows.
        otherwise: Vec<Command>,
    },
    /// `RECODE <from> TO <to> <rules> [ELSE = v] END`.
    Recode {
        /// Source column.
        from: String,
        /// Target column.
        to: String,
        /// Ordered rules; first match wins.
        rules: Vec<RecodeRule>,
        /// `ELSE = <lit>` — never catches a missing source.
        otherwise: Option<Value>,
    },
    /// `MERGE '<file>':<table> <dkey>::<skey> ... [<mode>]`.
    Merge {
        /// The source table.
        dataset: DatasetRef,
        /// `(destination key, source key)` pairs.
        keys: Vec<(String, String)>,
        /// The write mode.
        mode: MergeMode,
    },
    /// `RELATE <table> [<dkey>::<ckey> ...] [ALL]`.
    Relate {
        /// The child table.
        dataset: DatasetRef,
        /// `(destination key, child key)` pairs; empty asks the
        /// session for the stored link.
        keys: Vec<(String, String)>,
    },
    /// `DELETE <expr> [RUNSILENT]` — marks or removes matching rows.
    DeleteRecords(Expr),
    /// `UNDELETE *|<expr> [RUNSILENT]`.
    Undelete(Option<Expr>),
    /// `DELETE VARIABLES <name> ...`.
    DeleteVariables(Vec<String>),
    /// `KEYVARS <name> ...`.
    KeyVars(Vec<String>),
    /// `SET <clause>` (one command per clause).
    Set(SetOption),
    /// `SUMMARIZE <name>::<agg>(<arg>) ... TO <table> [STRATAVAR=<v>]`.
    Summarize {
        /// Output aggregates.
        aggregates: Vec<Aggregate>,
        /// `STRATAVAR=` grouping columns.
        by: Vec<String>,
        /// The grouped table the view becomes.
        into: DatasetRef,
    },
}

/// Applies one command to the view.
pub(crate) fn apply(
    view: &mut WorkingView,
    linked: &dyn crate::view::LinkedDatasets,
    env: &mut EvalEnv,
    cmd: &Command,
) -> Result<(), LangError> {
    match cmd {
        Command::Select(e) => select(view, env, e),
        Command::CancelSelect => cancel_select(view),
        Command::Sort(keys) => sort(view, keys),
        Command::CancelSort => cancel_sort(view),
        Command::Define { name, ty, scope } => define(view, name, *ty, *scope),
        Command::DefineExpr { name, expr, scope } => {
            let ty = crate::expr::types::type_check(
                expr,
                &crate::expr::types::Schema::from_dataset(&view.dataset),
            )
            .unwrap_or(ValueType::Text);
            define_assign::define_expr(view, env, name, expr, ty, *scope)
        }
        Command::Undefine(n) => undefine(view, n),
        Command::Assign { name, expr } => assign(view, env, name, expr),
        Command::If {
            cond,
            then,
            otherwise,
        } => define_assign::if_then(view, env, cond, then, otherwise),
        Command::Recode {
            from,
            to,
            rules,
            otherwise,
        } => recode(view, from, to, rules, otherwise.as_ref()),
        Command::Merge {
            dataset,
            keys,
            mode,
        } => merge(view, linked, dataset, keys, *mode),
        Command::Relate { dataset, keys } => relate(view, linked, dataset, keys),
        Command::DeleteRecords(e) => delete_records(view, env, e),
        Command::Undelete(e) => undelete(view, env, e.as_ref()),
        Command::DeleteVariables(ns) => delete_variables(view, ns),
        Command::KeyVars(ns) => key_vars(view, ns),
        Command::Set(o) => set(view, o),
        Command::Summarize {
            aggregates,
            by,
            into,
        } => summarize(view, env, aggregates, by, into),
    }
}

/// The dataset a `DatasetRef` names, else `unknown_variable`.
pub(crate) fn named<'a>(
    linked: &'a dyn crate::view::LinkedDatasets,
    dref: &DatasetRef,
) -> Result<&'a epiora_core::dataset::Dataset, LangError> {
    linked
        .get(&dref.name)
        .ok_or_else(|| LangError::UnknownVariable {
            name: dref.name.clone(),
        })
}

/// `true` when `join == All`.
pub(crate) fn is_all(dref: &DatasetRef) -> bool {
    dref.join == JoinMode::All
}
