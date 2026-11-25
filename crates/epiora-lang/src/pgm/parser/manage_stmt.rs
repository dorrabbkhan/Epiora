//! Statement parsers for the single-line cleaning commands
//! (`Command` producers): SELECT, CANCEL, SORT, DEFINE, UNDEFINE,
//! ASSIGN / LET / simple assign, DELETE, UNDELETE, KEYVARS and SET.
//! The block commands (IF, RECODE, MERGE, RELATE, SUMMARIZE) live in
//! `blocks.rs`.

use super::{blocks, Parser, StmtOut, Tok};
use crate::expr::ast::{Expr, Lit};
use crate::manage::{Command, Scope};
use crate::pgm::ast::Statement;
use crate::pgm::lexer::Lexed;
use crate::view::Order;
use crate::LangError;
use epiora_core::dataset::ValueType;

mod set;

/// Dispatch a single-line manage command by head word.
pub(crate) fn manage(
    p: &Parser,
    i: usize,
    toks: &[Lexed],
    head: &str,
) -> Result<StmtOut, LangError> {
    match head {
        "SELECT" => select(p, i, toks),
        "CANCEL" => cancel(p, i, toks),
        "SORT" => sort(p, i, toks),
        "DEFINE" => define(p, i, toks),
        "UNDEFINE" => undefine(p, i, toks),
        "ASSIGN" => assign(p, i, toks, 1),
        "LET" => assign(p, i, toks, 1),
        "DELETE" => delete(p, i, toks),
        "UNDELETE" => undelete(p, i, toks),
        "KEYVARS" => keyvars(p, i, toks),
        "SET" => set::set(p, i, toks),
        "IF" | "RECODE" | "MERGE" | "RELATE" | "SUMMARIZE" => blocks::block(p, i, toks, head),
        _ => Ok(p.unsup(i, i + 1)),
    }
}

/// `<qid> = <expr>` — the keywordless simple assign.
pub(crate) fn simple_assign(
    p: &Parser,
    i: usize,
    toks: &[Lexed],
    eq: usize,
) -> Result<StmtOut, LangError> {
    let (name, _) = match p.qid(toks, 0) {
        Some(x) => x,
        None => return Ok(p.unsup(i, i + 1)),
    };
    let e = p.expr(toks, eq + 1, toks.len())?;
    Ok(StmtOut {
        statements: vec![p.spanned(toks, Statement::Manage(Command::Assign { name, expr: e }))],
        next: i + 1,
    })
}

fn select(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    // A bare `SELECT` re-selects everything (spec §5.3).
    let cmd = if toks.len() == 1 {
        Command::CancelSelect
    } else {
        Command::Select(p.expr(toks, 1, toks.len())?)
    };
    Ok(StmtOut {
        statements: vec![p.spanned(toks, Statement::Manage(cmd))],
        next: i + 1,
    })
}

fn cancel(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let cmd = match p.word(toks, 1).as_deref() {
        Some("SELECT") => Command::CancelSelect,
        Some("SORT") => Command::CancelSort,
        _ => return Ok(p.unsup(i, i + 1)),
    };
    if toks.len() != 2 {
        return Ok(p.unsup(i, i + 1));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(toks, Statement::Manage(cmd))],
        next: i + 1,
    })
}

fn sort(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    if n == 1 {
        // A bare `SORT` cancels the sort — grammar's `CANCEL SORT`
        // synonym in 7.2's interpreter.
        return Ok(StmtOut {
            statements: vec![p.spanned(toks, Statement::Manage(Command::CancelSort))],
            next: i + 1,
        });
    }
    let mut keys = Vec::new();
    let mut j = 1;
    while j < n {
        let Some((name, end)) = p.qid(toks, j) else {
            return Ok(p.unsup(i, i + 1));
        };
        j = end;
        let mut ord = Order::Asc;
        match p.word(toks, j).as_deref() {
            Some("DESCENDING" | "DESC") => {
                ord = Order::Desc;
                j += 1;
            }
            Some("ASCENDING" | "ASC") => j += 1,
            _ => {}
        }
        keys.push((name, ord));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(toks, Statement::Manage(Command::Sort(keys)))],
        next: i + 1,
    })
}

fn define(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    let Some((name, end)) = p.qid(toks, 1) else {
        return Ok(p.unsup(i, i + 1));
    };
    // `DEFINE x = <expr> [scope]` — a qualified name is not allowed
    // after DEFINE, so `name` must be a single identifier.
    if name.contains('.') {
        return Ok(p.unsup(i, i + 1));
    }
    if matches!(toks.get(end).map(|t| &t.tok), Some(Tok::Eq)) {
        // `DEFINE x = <expr>` — the canonical printer may append a
        // scope word (`DEFINE x = e GLOBAL`), so the expression is
        // the longest prefix that parses and a scope word may follow.
        let (e, mut eend) = p.expr_prefix(toks, end + 1, n)?;
        let mut scope = Scope::Standard;
        match p.word(toks, eend).as_deref() {
            Some("STANDARD") => eend += 1,
            Some("GLOBAL") => {
                scope = Scope::Global;
                eend += 1;
            }
            Some("PERMANENT") => {
                scope = Scope::Permanent;
                eend += 1;
            }
            _ => {}
        }
        if eend != n {
            return Ok(p.unsup(i, i + 1));
        }
        return Ok(StmtOut {
            statements: vec![p.spanned(
                toks,
                Statement::Manage(Command::DefineExpr {
                    name,
                    expr: e,
                    scope,
                }),
            )],
            next: i + 1,
        });
    }
    // Typed form: `DEFINE x [scope] TYPE [scope] ["prompt"|(prompt)]`.
    let mut ty: Option<ValueType> = None;
    let mut scope = Scope::Standard;
    let mut j = end;
    while j < n {
        match p.word(toks, j).as_deref() {
            Some("STANDARD") => {
                scope = Scope::Standard;
                j += 1;
            }
            Some("GLOBAL") => {
                scope = Scope::Global;
                j += 1;
            }
            Some("PERMANENT") => {
                scope = Scope::Permanent;
                j += 1;
            }
            Some(w) => {
                let t = match w {
                    "NUMERIC" => Some(ValueType::Real),
                    "TEXTINPUT" => Some(ValueType::Text),
                    "YN" | "YESNO" => Some(ValueType::Boolean),
                    "DATEFORMAT" => Some(ValueType::Date),
                    "TIMEFORMAT" => Some(ValueType::Time),
                    "DATETIMEFORMAT" => Some(ValueType::DateTime),
                    _ => None,
                };
                match t {
                    Some(t) => {
                        ty = Some(t);
                        j += 1;
                    }
                    None => return Ok(p.unsup(i, i + 1)),
                }
            }
            None => {
                // A prompt string — `"prompt"` or `("prompt")`.
                match toks.get(j).map(|t| &t.tok) {
                    Some(Tok::Str(_)) => j += 1,
                    Some(Tok::LParen)
                        if matches!(toks.get(j + 1).map(|t| &t.tok), Some(Tok::Str(_)))
                            && matches!(toks.get(j + 2).map(|t| &t.tok), Some(Tok::RParen)) =>
                    {
                        j += 3;
                    }
                    _ => return Ok(p.unsup(i, i + 1)),
                }
            }
        }
    }
    match ty {
        Some(ty) => Ok(StmtOut {
            statements: vec![
                p.spanned(toks, Statement::Manage(Command::Define { name, ty, scope }))
            ],
            next: i + 1,
        }),
        // `DEFINE x` with no type is the EI3 form — Unsupported.
        None => Ok(p.unsup(i, i + 1)),
    }
}

fn undefine(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    match toks.get(1).map(|t| &t.tok) {
        Some(Tok::Ident(name)) if toks.len() == 2 => Ok(StmtOut {
            statements: vec![p.spanned(toks, Statement::Manage(Command::Undefine(name.clone())))],
            next: i + 1,
        }),
        // `UNDEFINE *` and `UNDEFINE * GLOBAL` can't map to a
        // single-name Command → Unsupported.
        _ => Ok(p.unsup(i, i + 1)),
    }
}

/// `ASSIGN <qid> = <expr>` (and `LET x = <expr>`).
fn assign(p: &Parser, i: usize, toks: &[Lexed], start: usize) -> Result<StmtOut, LangError> {
    let Some((name, end)) = p.qid(toks, start) else {
        return Ok(p.unsup(i, i + 1));
    };
    if !matches!(toks.get(end).map(|t| &t.tok), Some(Tok::Eq)) {
        return Ok(p.unsup(i, i + 1));
    }
    // A DLL member call (`class!method`) anywhere in the value is
    // the Assign_DLL form → Unsupported.
    if toks[end + 1..].iter().any(|t| matches!(t.tok, Tok::Bang)) {
        return Ok(p.unsup(i, i + 1));
    }
    let e = p.expr(toks, end + 1, toks.len())?;
    Ok(StmtOut {
        statements: vec![p.spanned(toks, Statement::Manage(Command::Assign { name, expr: e }))],
        next: i + 1,
    })
}

fn delete(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    if n == 1 {
        return Ok(p.unsup(i, i + 1));
    }
    // `DELETE VARIABLES a b …`
    if p.kw(toks, 1, "VARIABLES") {
        let (names, end) = p.idlist(toks, 2, &[]);
        if names.is_empty() || end != n {
            return Ok(p.unsup(i, i + 1));
        }
        return Ok(StmtOut {
            statements: vec![p.spanned(toks, Statement::Manage(Command::DeleteVariables(names)))],
            next: i + 1,
        });
    }
    // `DELETE TABLES …`, `DELETE 'file'`, `DELETE {…}` → not the
    // records form.
    if p.kw(toks, 1, "TABLES")
        || matches!(
            toks.get(1).map(|t| &t.tok),
            Some(Tok::File(_)) | Some(Tok::Brace(_))
        )
    {
        return Ok(p.unsup(i, i + 1));
    }
    // `DELETE * [RUNSILENT]` — all records.
    if matches!(toks.get(1).map(|t| &t.tok), Some(Tok::Star)) {
        return record_options(
            p,
            i,
            toks,
            2,
            Command::DeleteRecords(Expr::Literal(Lit::Boolean(true))),
        );
    }
    // `DELETE <expr> [RUNSILENT]` — the expression is the longest
    // prefix that parses; trailing words must be option words.
    match p.expr_prefix(toks, 1, n) {
        Ok((e, end)) => record_options(p, i, toks, end, Command::DeleteRecords(e)),
        Err(e) => Err(e),
    }
}

fn undelete(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    if n == 1 {
        return Ok(p.unsup(i, i + 1));
    }
    if matches!(toks.get(1).map(|t| &t.tok), Some(Tok::Star)) {
        return record_options(p, i, toks, 2, Command::Undelete(None));
    }
    match p.expr_prefix(toks, 1, n) {
        Ok((e, end)) => record_options(p, i, toks, end, Command::Undelete(Some(e))),
        Err(e) => Err(e),
    }
}

/// Trailing `DELETE`/`UNDELETE` option words: `RUNSILENT` is
/// accepted and dropped; `PERMANENT`/`SAVEDATA` are file-writing
/// forms → Unsupported.
fn record_options(
    p: &Parser,
    i: usize,
    toks: &[Lexed],
    mut j: usize,
    cmd: Command,
) -> Result<StmtOut, LangError> {
    while j < toks.len() {
        match p.word(toks, j).as_deref() {
            Some("RUNSILENT") => j += 1,
            _ => return Ok(p.unsup(i, i + 1)),
        }
    }
    Ok(StmtOut {
        statements: vec![p.spanned(toks, Statement::Manage(cmd))],
        next: i + 1,
    })
}

fn keyvars(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let mut j = 1;
    if matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Eq)) {
        j += 1;
    }
    let (names, end) = p.idlist(toks, j, &[]);
    if names.is_empty() || end != toks.len() {
        return Ok(p.unsup(i, i + 1));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(toks, Statement::Manage(Command::KeyVars(names)))],
        next: i + 1,
    })
}
