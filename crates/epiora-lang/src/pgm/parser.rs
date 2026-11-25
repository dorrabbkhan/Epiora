//! The `.pgm` parser: a line-bounded recursive-descent parse of the
//! Analysis grammar (spec §6.1: "the full Epi Info 7 Analysis
//! grammar, so every valid Epi Info program parses, including
//! commands Epiora does not run yet"). Supported commands produce
//! `Statement` nodes; recognized-but-unrun commands become
//! `Statement::Unsupported` with their name and source text — only
//! genuinely unrecognized text is a `pgm_syntax` error.
//!
//! Expression spans delegate to [`crate::parse_expr`] on the token
//! slice (translating positions back), so the whole expression
//! grammar is available without re-implementing it.

use super::ast::{Program, Spanned, Statement};
use super::lexer::{lex_line, Lexed, Tok};
use crate::expr::ast::Expr;
use crate::LangError;

mod blocks;
mod commands;
mod helpers;
mod manage_stmt;

/// Parse program text (decoded by [`super::decode_pgm_text`]; the
/// caller splits bytes → text so Windows-1252 survives). CRLF, CR
/// and LF line endings all work.
pub fn parse_program(text: &str) -> Result<Program, LangError> {
    Parser::new(text).program()
}

/// The outcome of parsing one statement.
pub(crate) struct StmtOut {
    /// Statements produced (a `SET` line can produce several).
    pub(crate) statements: Vec<Spanned<Statement>>,
    /// Index of the next unconsumed line.
    pub(crate) next: usize,
}

pub(crate) struct Parser<'a> {
    text: &'a str,
    lines: Vec<Line<'a>>,
}

struct Line<'a> {
    /// Line text without the newline.
    raw: &'a str,
    /// Lexed tokens — a stored error surfaces only when a statement
    /// needs this line (skipped block bodies never do).
    toks: Result<Vec<Lexed>, LangError>,
}

impl<'a> Parser<'a> {
    fn new(text: &'a str) -> Parser<'a> {
        // Line splitting: LF, CRLF and bare CR all end a line; a
        // trailing newline leaves one empty line at the end (it
        // lexes to nothing and is skipped).
        let bytes = text.as_bytes();
        let mut lines = Vec::new();
        let mut start = 0usize;
        let mut i = 0usize;
        while i <= bytes.len() {
            let eol = i == bytes.len() || matches!(bytes[i], b'\n' | b'\r');
            if !eol {
                i += 1;
                continue;
            }
            let raw = &text[start..i];
            lines.push(Line {
                raw,
                toks: lex_line(raw, (lines.len() + 1) as u32, start),
            });
            if i == bytes.len() {
                break;
            }
            i += if bytes[i] == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
                2
            } else {
                1
            };
            start = i;
        }
        Parser { text, lines }
    }

    fn program(&self) -> Result<Program, LangError> {
        let mut statements = Vec::new();
        let mut i = 0usize;
        while i < self.lines.len() {
            if self.toks(i)?.is_empty() {
                i += 1;
                continue;
            }
            let out = self.statement(i)?;
            statements.extend(out.statements);
            i = out.next;
        }
        Ok(Program { statements })
    }

    /// Line `i`'s tokens, or its stored lex error.
    pub(crate) fn toks(&self, i: usize) -> Result<&[Lexed], LangError> {
        match &self.lines[i].toks {
            Ok(t) => Ok(t),
            Err(e) => Err(e.clone()),
        }
    }

    /// The line's raw text.
    pub(crate) fn raw(&self, i: usize) -> &'a str {
        self.lines[i].raw
    }

    /// Number of source lines.
    pub(crate) fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// The head keyword of line `i` (upper-cased), when it starts
    /// with an identifier.
    pub(crate) fn head(&self, i: usize) -> Option<String> {
        match self.toks(i).ok()?.first().map(|t| &t.tok) {
            Some(Tok::Ident(w)) => Some(w.to_ascii_uppercase()),
            _ => None,
        }
    }

    /// A syntax error at token `j` of `toks`.
    pub(crate) fn err_at(&self, toks: &[Lexed], j: usize, message: &str) -> LangError {
        let t = toks.get(j).or_else(|| toks.last());
        LangError::Syntax {
            line: t.map(|t| t.line).unwrap_or(1),
            column: t.map(|t| t.column).unwrap_or(1),
            message: message.to_string(),
        }
    }

    /// The source slice covering `toks[lo..hi]`.
    pub(crate) fn span(&self, toks: &[Lexed], lo: usize, hi: usize) -> &'a str {
        &self.text[toks[lo].start..toks[hi - 1].end]
    }

    /// `toks[j]` is the identifier `kw` (case-insensitive).
    pub(crate) fn kw(&self, toks: &[Lexed], j: usize, kw: &str) -> bool {
        matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Ident(w)) if w.eq_ignore_ascii_case(kw))
    }

    /// `toks[j]` is `Tok::Ident` — the word, upper-cased.
    pub(crate) fn word(&self, toks: &[Lexed], j: usize) -> Option<String> {
        match toks.get(j).map(|t| &t.tok) {
            Some(Tok::Ident(w)) => Some(w.to_ascii_uppercase()),
            _ => None,
        }
    }

    /// A qualified identifier `a[.b…]` at `toks[j]` → (name, next j).
    pub(crate) fn qid(&self, toks: &[Lexed], j: usize) -> Option<(String, usize)> {
        let Tok::Ident(first) = toks.get(j).map(|t| &t.tok)? else {
            return None;
        };
        let mut name = first.clone();
        let mut k = j + 1;
        while matches!(toks.get(k).map(|t| &t.tok), Some(Tok::Dot)) {
            let Some(Tok::Ident(part)) = toks.get(k + 1).map(|t| &t.tok) else {
                return None;
            };
            name.push('.');
            name.push_str(part);
            k += 2;
        }
        Some((name, k))
    }

    /// Parse `toks[lo..hi]` as a complete expression — the source
    /// slice must lex and parse exactly (positions translated).
    pub(crate) fn expr(&self, toks: &[Lexed], lo: usize, hi: usize) -> Result<Expr, LangError> {
        if lo >= hi {
            return Err(self.err_at(toks, lo.saturating_sub(1), "expected an expression"));
        }
        let slice = self.span(toks, lo, hi);
        crate::parse_expr(slice).map_err(|e| match e {
            LangError::Syntax {
                column, message, ..
            } => LangError::Syntax {
                line: toks[lo].line,
                column: toks[lo].column + column.saturating_sub(1),
                message,
            },
            other => other,
        })
    }

    /// The longest prefix of `toks[lo..hi]` that is a complete
    /// expression → (expr, exclusive end index). Lets trailing
    /// option words follow the expression (`DELETE e RUNSILENT`).
    pub(crate) fn expr_prefix(
        &self,
        toks: &[Lexed],
        lo: usize,
        hi: usize,
    ) -> Result<(Expr, usize), LangError> {
        let mut end = hi;
        let mut first_err = None;
        while end > lo {
            match self.expr(toks, lo, end) {
                Ok(e) => return Ok((e, end)),
                Err(e) => {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                    end -= 1;
                }
            }
        }
        match first_err {
            Some(e) => Err(e),
            None => Err(self.err_at(toks, lo, "expected an expression")),
        }
    }

    /// One statement starting at line `i`.
    pub(crate) fn statement(&self, i: usize) -> Result<StmtOut, LangError> {
        let toks = self.toks(i)?;
        let Some(head) = self.head(i) else {
            return Err(self.err_at(toks, 0, "expected a command"));
        };
        match head.as_str() {
            "READ" => commands::read(self, i, toks),
            "WRITE" => commands::write(self, i, toks),
            "LIST" => commands::list(self, i, toks),
            "DISPLAY" => commands::display(self, i, toks),
            "FREQ" | "TABLES" | "MEANS" => commands::procedure(self, i, toks, &head),
            "RUNPGM" => commands::runpgm(self, i, toks),
            "QUIT" | "EXIT" => {
                if toks.len() == 1 {
                    Ok(StmtOut {
                        statements: vec![self.spanned(toks, Statement::Quit)],
                        next: i + 1,
                    })
                } else {
                    Ok(self.unsup(i, i + 1))
                }
            }
            "SELECT" | "CANCEL" | "SORT" | "DEFINE" | "UNDEFINE" | "ASSIGN" | "LET" | "IF"
            | "RECODE" | "MERGE" | "RELATE" | "DELETE" | "UNDELETE" | "KEYVARS" | "SET"
            | "SUMMARIZE" => manage_stmt::manage(self, i, toks, &head),
            "PYTHON" | "R" | "CMD" | "SUB" | "VIEW" | "FORM" | "PAGE" | "FIELD" | "RECORD"
            | "DEFINEVARIABLES" | "ALWAYS" | "BEFORE" | "AFTER" | "FILESPEC" => {
                self.block_unsup(i, toks, &head)
            }
            "BEEP" | "ABOUT" | "CLOSEOUT" | "NEWPAGE" | "PRINTOUT" | "SYSINFO" | "TYPEOUT"
            | "HEADER" | "ROUTEOUT" => Ok(StmtOut {
                statements: vec![self.spanned(toks, Statement::Output(self.raw(i).trim().into()))],
                next: i + 1,
            }),
            _ => {
                if let Some(_name) = self.register_name(toks) {
                    return Ok(self.unsup(i, i + 1));
                }
                if let Some((_, eq)) = self.qid(toks, 0) {
                    if matches!(toks.get(eq).map(|t| &t.tok), Some(Tok::Eq)) {
                        return manage_stmt::simple_assign(self, i, toks, eq);
                    }
                }
                Err(self.err_at(toks, 0, &format!("unrecognized command {head}")))
            }
        }
    }

    /// A `Spanned` covering a statement that starts at `toks[0]`.
    pub(crate) fn spanned(&self, toks: &[Lexed], node: Statement) -> Spanned<Statement> {
        Spanned {
            line: toks[0].line,
            column: toks[0].column,
            node,
        }
    }

    /// An `Unsupported` statement spanning lines `i..j`.
    pub(crate) fn unsup(&self, i: usize, j: usize) -> StmtOut {
        let toks = match self.toks(i) {
            Ok(t) => t,
            Err(_) => &[][..],
        };
        let name = self
            .register_name(toks)
            .or_else(|| self.word(toks, 0))
            .unwrap_or_default();
        let text: String = (i..j)
            .map(|k| self.raw(k).trim_end())
            .collect::<Vec<_>>()
            .join("\n")
            .trim()
            .to_string();
        let first = toks.first();
        StmtOut {
            statements: vec![Spanned {
                line: first.map(|t| t.line).unwrap_or((i + 1) as u32),
                column: first.map(|t| t.column).unwrap_or(1),
                node: Statement::Unsupported { name, text },
            }],
            next: j,
        }
    }

    /// A block command (`PYTHON … END-PYTHON`, `SUB … END-SUB`,
    /// check-code `VIEW … END`, …) — an Unsupported statement from
    /// the head line through its `END` line. Body lines are never
    /// lexed, so embedded code of any shape survives.
    fn block_unsup(&self, i: usize, toks: &[Lexed], head: &str) -> Result<StmtOut, LangError> {
        let ends: &[&str] = match head {
            "PYTHON" => &["END-PYTHON", "END"],
            "R" => &["END-R", "END"],
            "CMD" => &["END-CMD", "END"],
            "SUB" => &["END-SUB", "END"],
            _ => &["END"],
        };
        let mut j = i + 1;
        while j < self.lines.len() {
            let up = self.raw(j).trim().to_ascii_uppercase();
            if ends.iter().any(|e| up == *e) {
                return Ok(self.unsup(i, j + 1));
            }
            j += 1;
        }
        Err(self.err_at(toks, 0, &format!("missing {} for {head}", ends[0])))
    }
}
