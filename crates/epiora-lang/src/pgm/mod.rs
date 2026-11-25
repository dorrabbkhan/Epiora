//! The `.pgm` program layer (v1.1 spec §6.1, plan Task 12): the
//! parser, planner, command register and canonical printer.
//!
//! - [`parse_program`] parses the full Analysis grammar; commands
//!   Epiora runs become structured [`Statement`]s, recognized-but-
//!   unrun commands become `Statement::Unsupported`, and only
//!   genuinely unrecognized text is a `pgm_syntax` error.
//! - [`plan`]/[`plan_with`] turn a `Program` into [`Action`]s —
//!   the list a run executes, with `may_differ` marking and
//!   `RUNPGM` inlining through a [`ProgramSource`].
//! - [`register`] is the embedded `register.json` command table.
//! - [`print_command`] is the canonical, re-parseable spelling of
//!   a `Command` — the recipe's `canonical_text` is built from it.
//! - [`decode_pgm_text`] decodes program bytes: UTF-8, or
//!   Windows-1252 when the bytes are not UTF-8 (EI3-era files).

pub mod ast;
mod lexer;
mod parser;
pub mod plan;
pub(crate) mod print;
mod register;

pub use ast::{
    DisplayWhat, Program, ProgramRef, Spanned, Statement, WriteMode, WriteTarget, WriteVars,
};
pub use parser::parse_program;
pub use plan::{plan, plan_with, Action, ProgramSource};
pub use print::print_command;
pub use register::{register, register_json, Entry, FieldType, Module, Register, Status};

/// Decode program bytes to text: UTF-8 verbatim; Windows-1252
/// when the bytes are not valid UTF-8 (EI3-era `.pgm` files are
/// ANSI). A BOM is stripped.
pub fn decode_pgm_text(bytes: &[u8]) -> String {
    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| cp1252(b)).collect(),
    };
    text.strip_prefix('\u{feff}').unwrap_or(&text).to_string()
}

/// One Windows-1252 byte → its char. Bytes 0x00–0x7F and
/// 0xA0–0xFF map to Latin-1 (so `b as char` covers them); only
/// 0x80–0x9F need the table.
fn cp1252(b: u8) -> char {
    match b {
        0x80 => '€',
        0x82 => '‚',
        0x83 => 'ƒ',
        0x84 => '„',
        0x85 => '…',
        0x86 => '†',
        0x87 => '‡',
        0x88 => 'ˆ',
        0x89 => '‰',
        0x8A => 'Š',
        0x8B => '‹',
        0x8C => 'Œ',
        0x8E => 'Ž',
        0x91 => '\u{2018}',
        0x92 => '\u{2019}',
        0x93 => '\u{201C}',
        0x94 => '\u{201D}',
        0x95 => '•',
        0x96 => '–',
        0x97 => '—',
        0x98 => '˜',
        0x99 => '™',
        0x9A => 'š',
        0x9B => '›',
        0x9C => 'œ',
        0x9E => 'ž',
        0x9F => 'Ÿ',
        _ => b as char,
    }
}
