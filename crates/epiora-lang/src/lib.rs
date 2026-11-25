//! `epiora-lang`: the Epi Info language. v1.1 Task 10 ships the
//! expression language (spec §5.2): [`parse_expr`], [`type_check`] and
//! [`eval_column`] over the [`Expr`] AST, the `(.)` missing domain and
//! every function the register marks `supported` at `1.1`.
//!
//! Semantics port Epi Info 7.2.7.0's
//! `Epi.Core.AnalysisInterpreter/Rules/` (pinned source @ 4cd353c, kept
//! under `reference/`): missing cells evaluate as empty text, `(.)` is
//! the `null` literal, and every failure is a [`LangError`] with a
//! stable `code()` — never a panic.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

mod expr;
pub mod functions;
mod links;
mod manage;
mod pgm;
mod recipe;
mod view;

#[cfg(feature = "fuzzing")]
pub mod fuzz_harness;

pub use expr::ast::{BinOp, Expr, Lit, UnOp};
pub use expr::eval::{eval_column, AsOf, EvalEnv};
pub use expr::parser::parse_expr;
pub use expr::types::{type_check, Schema};
pub use links::StoredLinks;
pub use manage::{
    AggFunc, Aggregate, Command, MergeMode, RecodeBound, RecodeRule, Scope, SetOption,
};
pub use pgm::{
    decode_pgm_text, parse_program, plan, plan_with, print_command, register, register_json,
    Action, DisplayWhat, Entry, FieldType, Module, Program, ProgramRef, ProgramSource, Register,
    Spanned, Statement, Status, WriteMode, WriteTarget, WriteVars,
};
pub use recipe::{replay, working_view_stamp, Recipe};
pub use view::{
    DatasetRef, JoinMode, LinkedDatasets, Order, Process, StatisticsLevel, ViewSettings,
    WorkingView,
};

/// A language failure with a stable machine-readable `code()`
/// (spec §10: `pgm_syntax`, `expr_type`, `unknown_variable`; the two
/// function codes name the register surface the register cannot).
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum LangError {
    /// Lexical or parse failure (`pgm_syntax`). `line`/`column` are
    /// 1-based positions of the offending token.
    #[error("syntax error at {line}:{column}: {message}")]
    Syntax {
        /// 1-based line.
        line: u32,
        /// 1-based column.
        column: u32,
        /// What the parser expected.
        message: String,
    },
    /// An operand or argument has the wrong type (`expr_type`).
    /// `column` names the offending variable or sub-expression.
    #[error("type error in {column}: expected {expected}, found {found}")]
    ExprType {
        /// The variable or sub-expression at fault.
        column: String,
        /// The expected type class.
        expected: String,
        /// The type that was found.
        found: String,
    },
    /// A name that is neither a dataset column nor a session global
    /// (`unknown_variable`).
    #[error("unknown variable {name}")]
    UnknownVariable {
        /// The unresolved name.
        name: String,
    },
    /// A callee outside the recognized function set
    /// (`unknown_function`; EI7 reports the same at parse time).
    #[error("function name {name} is not a recognized function")]
    UnknownFunction {
        /// The unrecognized callee.
        name: String,
    },
    /// A recognized function EI7 itself rejects at run time —
    /// `PFROMZ` and `ZSCORE` throw "no longer supported by Epi Info"
    /// (`unsupported_function`).
    #[error("function {name} is no longer supported by Epi Info")]
    UnsupportedFunction {
        /// The rejected function.
        name: String,
    },
    /// A MERGE key pair whose destination and source types cannot
    /// join (`merge_key_type`, spec §10).
    #[error("merge key type mismatch on {column}: {message}")]
    MergeKeyType {
        /// The destination key column.
        column: String,
        /// Detail.
        message: String,
    },
    /// A keyless RELATE the session has no stored link for
    /// (`relate_no_link`).
    #[error("no stored link from {parent} to {child}")]
    RelateNoLink {
        /// The parent (current) dataset.
        parent: String,
        /// The child dataset.
        child: String,
    },
    /// A recipe step that cannot replay against the source dataset
    /// (`recipe_replay_mismatch`) — reserved for the store-side
    /// replay checks.
    #[error("recipe step cannot replay: {message}")]
    RecipeReplayMismatch {
        /// Which step and why.
        message: String,
    },
    /// A `RUNPGM` chain that names a program already running
    /// (`pgm_runpgm_cycle`, spec §10).
    #[error("RUNPGM cycle: {name} is already running")]
    RunPgmCycle {
        /// The program that closed the cycle.
        name: String,
    },
}

impl LangError {
    /// The stable machine-readable code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Syntax { .. } => "pgm_syntax",
            Self::ExprType { .. } => "expr_type",
            Self::UnknownVariable { .. } => "unknown_variable",
            Self::UnknownFunction { .. } => "unknown_function",
            Self::UnsupportedFunction { .. } => "unsupported_function",
            Self::MergeKeyType { .. } => "merge_key_type",
            Self::RelateNoLink { .. } => "relate_no_link",
            Self::RecipeReplayMismatch { .. } => "recipe_replay_mismatch",
            Self::RunPgmCycle { .. } => "pgm_runpgm_cycle",
        }
    }
}
