//! `RECODE <id> TO <id>` … `END`: rule lines are `v [- v] = v` or
//! `ELSE = v`; a rule outside `RecodeRule`'s literal domain makes
//! the whole block `Unsupported`.

use super::{Parser, StmtOut, Tok};
use crate::manage::{Command, RecodeBound, RecodeRule};
use crate::pgm::ast::Statement;
use crate::pgm::lexer::Lexed;
use crate::LangError;
use epiora_core::dataset::Value;

/// `RECODE <id> TO <id>` then rule lines until `END`. Rules are
/// `value [- value] = value` or `ELSE = value`.
pub(super) fn recode(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let Some((from, end)) = p.qid(toks, 1) else {
        return Err(p.err_at(toks, 0, "expected a variable after RECODE"));
    };
    if !p.kw(toks, end, "TO") {
        return Err(p.err_at(toks, end, "expected TO in RECODE"));
    }
    let Some((to, head_end)) = p.qid(toks, end + 1) else {
        return Err(p.err_at(toks, end, "expected a target variable after TO"));
    };
    if head_end != toks.len() {
        return Err(p.err_at(toks, head_end, "unexpected tokens after RECODE target"));
    }
    let mut rules = Vec::new();
    let mut otherwise = None;
    let mut j = i + 1;
    let mut ended = false;
    while j < p.line_count() {
        let t = p.toks(j)?;
        if t.is_empty() {
            j += 1;
            continue;
        }
        if p.kw(t, 0, "END") && t.len() == 1 {
            ended = true;
            j += 1;
            break;
        }
        match recode_rule(p, t) {
            Some(RuleOut::Rule(r)) => rules.push(r),
            Some(RuleOut::Else(v)) => {
                if otherwise.is_some() {
                    return Err(p.err_at(t, 0, "duplicate ELSE in RECODE"));
                }
                otherwise = Some(v);
            }
            // A rule-shaped line we can't represent (variable
            // values, `(.)`) → whole RECODE is Unsupported.
            None => return Ok(p.unsup(i, j + 1)),
        }
        j += 1;
    }
    if !ended {
        // An unterminated block reports its opening statement.
        return Err(p.err_at(toks, 0, "missing END for RECODE block"));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(
            toks,
            Statement::Manage(Command::Recode {
                from,
                to,
                rules,
                otherwise,
            }),
        )],
        next: j,
    })
}

enum RuleOut {
    Rule(RecodeRule),
    Else(Value),
}

/// One RECODE rule line → Rule/Else, or `None` when the rule uses
/// values `RecodeRule` can't represent (identifiers, `(.)` missing).
fn recode_rule(p: &Parser, toks: &[Lexed]) -> Option<RuleOut> {
    // `ELSE = <value>`
    if p.kw(toks, 0, "ELSE") && matches!(toks.get(1).map(|t| &t.tok), Some(Tok::Eq)) {
        let (v, end) = lit_value(p, toks, 2)?;
        return (end == toks.len()).then_some(RuleOut::Else(v));
    }
    // `<bound> [- <bound>] = <value>`
    let (lo, mut j) = bound(p, toks, 0)?;
    if matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Minus)) {
        let (hi, end) = bound(p, toks, j + 1)?;
        j = end;
        if matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Eq)) {
            let (v, vend) = lit_value(p, toks, j + 1)?;
            if vend == toks.len() {
                return Some(RuleOut::Rule(RecodeRule::Range { lo, hi, to: v }));
            }
        }
        return None;
    }
    if matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Eq)) {
        let (from_v, _) = match lo {
            RecodeBound::Value(v) => (v, ()),
            _ => return None, // `LOVALUE = x` without a range is not a rule.
        };
        let (v, vend) = lit_value(p, toks, j + 1)?;
        if vend == toks.len() {
            return Some(RuleOut::Rule(RecodeRule::Single {
                from: from_v,
                to: v,
            }));
        }
    }
    None
}

/// A range bound: `LOVALUE`, `HIVALUE` or a literal.
fn bound(p: &Parser, toks: &[Lexed], j: usize) -> Option<(RecodeBound, usize)> {
    match p.word(toks, j).as_deref() {
        Some("LOVALUE") => Some((RecodeBound::LoValue, j + 1)),
        Some("HIVALUE") => Some((RecodeBound::HiValue, j + 1)),
        _ => lit_value(p, toks, j).map(|(v, end)| (RecodeBound::Value(v), end)),
    }
}

/// A RECODE literal: number, string, file-quoted text, `(+)`/`(-)`,
/// date, time or date+time.
fn lit_value(_p: &Parser, toks: &[Lexed], j: usize) -> Option<(Value, usize)> {
    match toks.get(j).map(|t| &t.tok) {
        Some(Tok::Int(v)) => Some((Value::Integer(*v), j + 1)),
        Some(Tok::Real(v)) => Some((Value::Real(*v), j + 1)),
        // `-42` / `-3.5` — the printer emits signed literals.
        Some(Tok::Minus) => match toks.get(j + 1).map(|t| &t.tok) {
            Some(Tok::Int(v)) => Some((Value::Integer(-v), j + 2)),
            Some(Tok::Real(v)) => Some((Value::Real(-v), j + 2)),
            _ => None,
        },
        Some(Tok::Str(s)) | Some(Tok::File(s)) => Some((Value::Text(s.clone()), j + 1)),
        Some(Tok::Date(Some(d))) => {
            // `m/d/y h:mm:ss` — a date followed by a time literal.
            if let Some(Tok::Time(Some(t))) = toks.get(j + 1).map(|t| &t.tok) {
                Some((Value::DateTime((*d as i64) * 86400 + (*t as i64)), j + 2))
            } else {
                Some((Value::Date(*d), j + 1))
            }
        }
        Some(Tok::Time(Some(t))) => Some((Value::Time(*t), j + 1)),
        Some(Tok::LParen) => match (
            toks.get(j + 1).map(|t| &t.tok),
            toks.get(j + 2).map(|t| &t.tok),
        ) {
            (Some(Tok::Plus), Some(Tok::RParen)) => Some((Value::Boolean(true), j + 3)),
            (Some(Tok::Minus), Some(Tok::RParen)) => Some((Value::Boolean(false), j + 3)),
            _ => None, // `(.)` missing can't be a Recode value.
        },
        _ => None,
    }
}
