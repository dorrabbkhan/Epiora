//! `SET <clause>… [END-SET]` — one statement per clause.

use super::{Parser, StmtOut, Tok};
use crate::manage::{Command, SetOption};
use crate::pgm::ast::Statement;
use crate::pgm::lexer::Lexed;
use crate::view::{Process, StatisticsLevel};
use crate::LangError;

/// `SET <clause>… [END-SET]` — one statement per clause: the
/// results-affecting options become `Command::Set`, display-only
/// ones become `Output` notes.
pub(super) fn set(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let mut n = toks.len();
    // Trailing `END-SET` (lexed `END - SET`).
    if n >= 3
        && p.kw(toks, n - 3, "END")
        && matches!(toks.get(n - 2).map(|t| &t.tok), Some(Tok::Minus))
        && p.kw(toks, n - 1, "SET")
    {
        n -= 3;
    }
    let mut j = 1;
    let mut out = Vec::new();
    while j < n {
        // Clause name: an option word, or a Boolean literal
        // (`SET (+) = "y"` — always a display note).
        let (name, jname) = match p.word(toks, j) {
            Some(w) => (w, j + 1),
            None => match (
                toks.get(j).map(|t| &t.tok),
                toks.get(j + 1).map(|t| &t.tok),
                toks.get(j + 2).map(|t| &t.tok),
            ) {
                (Some(Tok::LParen), Some(Tok::Plus), Some(Tok::RParen)) => {
                    ("(+)".to_string(), j + 3)
                }
                (Some(Tok::LParen), Some(Tok::Minus), Some(Tok::RParen)) => {
                    ("(-)".to_string(), j + 3)
                }
                (Some(Tok::LParen), Some(Tok::Dot), Some(Tok::RParen)) => {
                    ("(.)".to_string(), j + 3)
                }
                _ => return Ok(p.unsup(i, i + 1)),
            },
        };
        j = jname;
        if !matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Eq)) {
            return Ok(p.unsup(i, i + 1));
        }
        let (v, end) = p.opt_value(toks, j + 1);
        let Some(v) = v else {
            return Ok(p.unsup(i, i + 1));
        };
        j = end;
        match set_option(&name, &v) {
            Some(opt) => out.push(p.spanned(toks, Statement::Manage(Command::Set(opt)))),
            None => {
                let text = p.raw(i).trim().to_string();
                out.push(p.spanned(toks, Statement::Output(text)))
            }
        }
    }
    if out.is_empty() {
        // `SET` alone or `SET END-SET`.
        return Ok(p.unsup(i, i + 1));
    }
    Ok(StmtOut {
        statements: out,
        next: i + 1,
    })
}

/// One `SET k=v` clause → its `SetOption`, or `None` for the
/// display-only options (recorded as notes).
fn set_option(name: &str, v: &serde_json::Value) -> Option<SetOption> {
    let word = v.as_str()?.to_ascii_uppercase();
    let on = |w: &str| match w {
        "ON" | "YES" => Some(true),
        "OFF" | "NO" => Some(false),
        _ => None,
    };
    match name {
        "PROCESS" => Some(SetOption::Process(match word.as_str() {
            "UNDELETED" => Process::Undeleted,
            "DELETED" => Process::Deleted,
            "BOTH" => Process::Both,
            _ => return None,
        })),
        "STATISTICS" => Some(SetOption::Statistics(match word.as_str() {
            "NONE" => StatisticsLevel::None,
            "MINIMAL" => StatisticsLevel::Minimal,
            "INTERMEDIATE" => StatisticsLevel::Intermediate,
            "COMPLETE" => StatisticsLevel::Complete,
            _ => return None,
        })),
        "MISSING" => on(&word).map(SetOption::Missing),
        "DELETED" => on(&word).map(SetOption::Deleted),
        "PERCENTS" => on(&word).map(SetOption::Percents),
        "IGNORE" => on(&word).map(SetOption::Ignore),
        "SELECT" => on(&word).map(SetOption::Select),
        _ => None,
    }
}
