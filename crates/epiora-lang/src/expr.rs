//! The expression language: lexer, recursive-descent parser and AST
//! (the grammar's `Expression → … → Value` precedence chain), the
//! Epi Info missing-value domain, the type checker and the
//! column evaluator.

pub(crate) mod ast;
pub(crate) mod eval;
pub(crate) mod lexer;
pub(crate) mod lit;
pub(crate) mod missing;
pub(crate) mod ops;
pub(crate) mod parser;
pub(crate) mod types;
