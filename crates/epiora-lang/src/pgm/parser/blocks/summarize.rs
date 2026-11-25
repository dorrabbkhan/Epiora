//! `SUMMARIZE <name>::<agg>(<arg>)… TO <table> [STRATAVAR=vars]`.

use super::{target_prefix, Parser, StmtOut, Tok};
use crate::manage::{AggFunc, Aggregate, Command};
use crate::pgm::ast::Statement;
use crate::pgm::lexer::Lexed;
use crate::view::DatasetRef;
use crate::LangError;

/// `SUMMARIZE <name>::<agg>(<arg>)… TO <qid> [STRATAVAR=vars]`.
pub(super) fn summarize(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    let mut j = 1;
    let mut aggregates = Vec::new();
    loop {
        // `name :: FUNC ( arg )`
        let Some((name, end)) = p.qid(toks, j) else {
            return Ok(p.unsup(i, i + 1));
        };
        if name.contains('.') {
            return Ok(p.unsup(i, i + 1));
        }
        if !matches!(toks.get(end).map(|t| &t.tok), Some(Tok::DColon)) {
            return Ok(p.unsup(i, i + 1));
        }
        let func = match p.word(toks, end + 1).as_deref() {
            Some("COUNT") => AggFunc::Count,
            Some("SUM") => AggFunc::Sum,
            Some("AVG") => AggFunc::Avg,
            Some("MIN") => AggFunc::Min,
            Some("MAX") => AggFunc::Max,
            Some("FIRST") => AggFunc::First,
            Some("LAST") => AggFunc::Last,
            Some("STDEV") => AggFunc::Stdev,
            Some("VAR" | "VARIANCE") => AggFunc::Variance,
            _ => return Ok(p.unsup(i, i + 1)),
        };
        let mut k = end + 2;
        if !matches!(toks.get(k).map(|t| &t.tok), Some(Tok::LParen)) {
            return Ok(p.unsup(i, i + 1));
        }
        k += 1;
        let arg = if matches!(toks.get(k).map(|t| &t.tok), Some(Tok::RParen)) {
            None
        } else {
            match p.qid(toks, k) {
                Some((a, end)) => {
                    k = end;
                    Some(a)
                }
                None => return Ok(p.unsup(i, i + 1)),
            }
        };
        if !matches!(toks.get(k).map(|t| &t.tok), Some(Tok::RParen)) {
            return Ok(p.unsup(i, i + 1));
        }
        k += 1;
        aggregates.push(Aggregate { name, func, arg });
        j = k;
        // Aggregates separate by a comma or just whitespace.
        if matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Comma)) {
            j += 1;
        }
        match p.qid(toks, j) {
            Some((_, end)) if matches!(toks.get(end).map(|t| &t.tok), Some(Tok::DColon)) => {
                continue;
            }
            _ => break,
        }
    }
    if !p.kw(toks, j, "TO") {
        return Ok(p.unsup(i, i + 1));
    }
    let Some((into, end)) = target_prefix(p, toks, j + 1) else {
        return Ok(p.unsup(i, i + 1));
    };
    j = end;
    let mut by = Vec::new();
    while j < n {
        // Options: `STRATAVAR = <idlist>` (WEIGHTVAR and others are
        // outside the implemented subset).
        if p.kw(toks, j, "STRATAVAR") && matches!(toks.get(j + 1).map(|t| &t.tok), Some(Tok::Eq)) {
            let (names, end) = p.idlist(toks, j + 2, &[]);
            by.extend(names);
            j = end;
        } else {
            return Ok(p.unsup(i, i + 1));
        }
    }
    Ok(StmtOut {
        statements: vec![p.spanned(
            toks,
            Statement::Manage(Command::Summarize {
                aggregates,
                by,
                into: DatasetRef::named(&into),
            }),
        )],
        next: i + 1,
    })
}
