//! Multi-line manage commands: IF…THEN…ELSE…END, RECODE…END,
//! MERGE, RELATE and SUMMARIZE. Inner statements recurse through
//! [`Parser::statement`]; an inner statement that is not a
//! `Command`-producing one makes the whole block `Unsupported`
//! (grammar-valid, just outside the implemented subset).

use super::{Parser, StmtOut, Tok};

mod recode;
mod summarize;
use crate::manage::{Command, MergeMode};
use crate::pgm::ast::Statement;
use crate::pgm::lexer::Lexed;
use crate::view::{DatasetRef, JoinMode};
use crate::LangError;

/// Dispatch the block-y commands.
pub(crate) fn block(
    p: &Parser,
    i: usize,
    toks: &[Lexed],
    head: &str,
) -> Result<StmtOut, LangError> {
    match head {
        "IF" => if_stmt(p, i, toks),
        "RECODE" => recode::recode(p, i, toks),
        "MERGE" => merge(p, i, toks),
        "RELATE" => relate(p, i, toks),
        "SUMMARIZE" => summarize::summarize(p, i, toks),
        _ => Ok(p.unsup(i, i + 1)),
    }
}

/// `IF <expr> THEN <stmts> [ELSE <stmts>] END` — inner statements
/// must all be `Manage` commands (an IF can nest).
fn if_stmt(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    // THEN is the last THEN-shaped token on the IF line; a line of
    // just `THEN` on the next line is also accepted.
    let mut then_tok = None;
    for (k, t) in toks.iter().enumerate().skip(1) {
        if matches!(&t.tok, Tok::Ident(w) if w.eq_ignore_ascii_case("THEN")) {
            then_tok = Some(k);
        }
    }
    let (cond, mut j) = match then_tok {
        Some(k) => (p.expr(toks, 1, k)?, i + 1),
        None => {
            // `IF <expr>` then a `THEN` line of its own.
            let e = p.expr(toks, 1, toks.len())?;
            let next = p.toks(i + 1);
            match next {
                Ok(nt) if p.kw(nt, 0, "THEN") && nt.len() == 1 => (e, i + 2),
                _ => {
                    return Err(p.err_at(toks, toks.len() - 1, "expected THEN after IF condition"))
                }
            }
        }
    };
    let mut then: Vec<Command> = Vec::new();
    let mut otherwise: Vec<Command> = Vec::new();
    let mut into_else = false;
    let mut ended = false;
    while j < p.line_count() {
        let t = p.toks(j)?;
        if t.is_empty() {
            j += 1;
            continue;
        }
        let head = p.head(j).unwrap_or_default();
        if head == "END" {
            ended = true;
            j += 1;
            break;
        }
        if head == "ELSE" && !into_else {
            // `ELSE` alone on the line.
            if t.len() != 1 {
                return Err(p.err_at(t, 1, "unexpected tokens after ELSE"));
            }
            into_else = true;
            j += 1;
            continue;
        }
        let out = p.statement(j)?;
        for st in out.statements {
            match st.node {
                Statement::Manage(c) => {
                    if into_else {
                        otherwise.push(c);
                    } else {
                        then.push(c);
                    }
                }
                // Any non-Command statement inside IF → the whole IF
                // is outside the implemented subset.
                _ => return Ok(p.unsup(i, out.next)),
            }
        }
        j = out.next;
    }
    if !ended {
        // An unterminated block reports its opening statement.
        return Err(p.err_at(toks, 0, "missing END for IF block"));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(
            toks,
            Statement::Manage(Command::If {
                cond,
                then,
                otherwise,
            }),
        )],
        next: j,
    })
}

/// `MERGE [['file'|'{conn}'] ':'] <qid> <k>::<k>… [APPEND|UPDATE]`.
fn merge(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let Some((target, mut j)) = target_prefix(p, toks, 1) else {
        return Ok(p.unsup(i, i + 1));
    };
    let Some((keys, end)) = key_defs(p, toks, j) else {
        return Ok(p.unsup(i, i + 1));
    };
    if keys.is_empty() {
        return Ok(p.unsup(i, i + 1));
    }
    j = end;
    let mode = match p.word(toks, j).as_deref() {
        None => MergeMode::MatchOrInsert,
        Some("UPDATE") => {
            j += 1;
            MergeMode::Update
        }
        Some("APPEND") => {
            j += 1;
            MergeMode::Append
        }
        _ => return Ok(p.unsup(i, i + 1)),
    };
    if j != toks.len() {
        return Ok(p.unsup(i, i + 1));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(
            toks,
            Statement::Manage(Command::Merge {
                dataset: DatasetRef::named(&target),
                keys,
                mode,
            }),
        )],
        next: i + 1,
    })
}

/// `RELATE [['file'|'{conn}'] ':'] <qid> [<k>::<k>…] [ALL|MATCHING]`.
fn relate(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let Some((target, mut j)) = target_prefix(p, toks, 1) else {
        return Ok(p.unsup(i, i + 1));
    };
    let Some((keys, end)) = key_defs(p, toks, j) else {
        return Ok(p.unsup(i, i + 1));
    };
    j = end;
    let mut join = JoinMode::Matching;
    match p.word(toks, j).as_deref() {
        Some("ALL") => {
            join = JoinMode::All;
            j += 1;
        }
        Some("MATCHING") => j += 1,
        None => {}
        _ => return Ok(p.unsup(i, i + 1)),
    }
    if j != toks.len() {
        return Ok(p.unsup(i, i + 1));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(
            toks,
            Statement::Manage(Command::Relate {
                dataset: DatasetRef { name: target, join },
                keys,
            }),
        )],
        next: i + 1,
    })
}

/// The `[file:]qid` target shared by MERGE and RELATE → (spelled
/// `file:table` or `table`, next token index).
pub(super) fn target_prefix(p: &Parser, toks: &[Lexed], j: usize) -> Option<(String, usize)> {
    // A leading format string (`"dBASE IV" 'file' …`) is the
    // unsupported file-spec form.
    if matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Str(_))) {
        return None;
    }
    if let Some(Tok::File(f)) | Some(Tok::Brace(f)) = toks.get(j).map(|t| &t.tok) {
        if matches!(toks.get(j + 1).map(|t| &t.tok), Some(Tok::Colon)) {
            let (t, end) = p.qid(toks, j + 2)?;
            return Some((format!("{f}:{t}"), end));
        }
        return None;
    }
    // `Ident:Ident` — the canonical printer's unquoted `file:table`.
    if matches!(toks.get(j + 1).map(|t| &t.tok), Some(Tok::Colon)) {
        let (f, _) = p.qid(toks, j)?;
        let (t, end) = p.qid(toks, j + 2)?;
        return Some((format!("{f}:{t}"), end));
    }
    p.qid(toks, j)
}

/// `k::k [AND k::k]…` — at least one pair → (pairs, next index), or
/// `None` if the first pair isn't `qid :: qid`. Zero pairs is legal
/// for RELATE (empty vec).
pub(super) fn key_defs(
    p: &Parser,
    toks: &[Lexed],
    j: usize,
) -> Option<(Vec<(String, String)>, usize)> {
    let mut keys = Vec::new();
    let mut k = j;
    while let Some((lhs, end)) = p.qid(toks, k) {
        if !matches!(toks.get(end).map(|t| &t.tok), Some(Tok::DColon)) {
            break;
        }
        let (rhs, rend) = p.qid(toks, end + 1)?;
        keys.push((lhs, rhs));
        k = rend;
        // Optional `AND` between pairs.
        if p.kw(toks, k, "AND") {
            k += 1;
        }
    }
    Some((keys, k))
}
