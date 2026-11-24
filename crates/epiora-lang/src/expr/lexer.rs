//! The expression lexer: one token per grammar terminal, with 1-based
//! line/column for `pgm_syntax` errors. Digit runs are lexed greedily
//! as the grammar's `Date`/`Time` terminals before numbers, exactly as
//! the GOLD lexer does (`4/18/1940` is a date, not `4 / 18 / 1940`).

use super::ast::{is_id_head, is_id_tail};
use super::lit::{lex_number, number, real};
use crate::LangError;

/// One lexed token with its source position.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Lexed {
    /// The token.
    pub tok: Tok,
    /// 1-based line.
    pub line: u32,
    /// 1-based column.
    pub column: u32,
}

/// Tokens, mirroring the grammar's terminals.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Tok {
    LParen,
    RParen,
    Comma,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Amp,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Caret,
    Dot,
    /// `(+)` / `(-)`.
    BoolLit(bool),
    /// `(.)`.
    MissingLit,
    IntLit(i64),
    RealLit(f64),
    /// `#`-hex literal — lexed, but `double.TryParse` fails on it in
    /// EI7 so it always evaluates to missing.
    HexLit,
    StrLit(String),
    CharLit(String),
    /// `M/D/Y` — `None` when the calendar date is invalid (EI7's
    /// `DateTime.TryParse` fails → missing).
    DateLit(Option<i32>),
    /// `H:MM[:SS]` — `None` is unreachable (the terminal rejects bad
    /// ranges, so the lexer reports them as syntax errors instead).
    TimeLit(Option<i32>),
    Ident(String),
    KwAnd,
    KwOr,
    KwXor,
    KwNot,
    KwMod,
    KwLike,
    Eof,
}

/// Lex `text` into tokens (the final element is `Eof`).
pub(crate) fn lex(text: &str) -> Result<Vec<Lexed>, LangError> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = Vec::new();
    let mut pos = Position::default();
    while let Some(&c) = chars.get(pos.i) {
        let (line, column) = (pos.line, pos.col);
        let at = |n: usize| chars.get(pos.i + n).copied();
        let err = |message: &str| LangError::Syntax {
            line,
            column,
            message: message.to_string(),
        };
        match c {
            ' ' | '\t' | '\r' => pos.advance(c),
            '\n' => pos.advance(c),
            '(' => match (at(1), at(2)) {
                (Some('+'), Some(')')) => {
                    pos.advance_n(3);
                    out.push(lexed(Tok::BoolLit(true), line, column));
                }
                (Some('-'), Some(')')) => {
                    pos.advance_n(3);
                    out.push(lexed(Tok::BoolLit(false), line, column));
                }
                (Some('.'), Some(')')) => {
                    pos.advance_n(3);
                    out.push(lexed(Tok::MissingLit, line, column));
                }
                _ => {
                    pos.advance(c);
                    out.push(lexed(Tok::LParen, line, column));
                }
            },
            ')' | ',' | '=' | '&' | '+' | '*' | '%' | '^' | '-' => {
                pos.advance(c);
                let tok = match c {
                    ')' => Tok::RParen,
                    ',' => Tok::Comma,
                    '=' => Tok::Eq,
                    '&' => Tok::Amp,
                    '+' => Tok::Plus,
                    '-' => Tok::Minus,
                    '*' => Tok::Star,
                    '%' => Tok::Percent,
                    _ => Tok::Caret,
                };
                out.push(lexed(tok, line, column));
            }
            '/' => {
                pos.advance(c);
                out.push(lexed(Tok::Slash, line, column));
            }
            '<' => {
                let tok = match at(1) {
                    Some('>') => Some(Tok::Ne),
                    Some('=') => Some(Tok::Le),
                    _ => None,
                };
                match tok {
                    Some(t) => {
                        pos.advance_n(2);
                        out.push(lexed(t, line, column));
                    }
                    None => {
                        pos.advance(c);
                        out.push(lexed(Tok::Lt, line, column));
                    }
                }
            }
            '>' => {
                if at(1) == Some('=') {
                    pos.advance_n(2);
                    out.push(lexed(Tok::Ge, line, column));
                } else {
                    pos.advance(c);
                    out.push(lexed(Tok::Gt, line, column));
                }
            }
            '"' => {
                let mut s = String::new();
                let mut n = 1;
                loop {
                    match at(n) {
                        Some('"') => break,
                        Some('\n') | None => {
                            return Err(err("unterminated string literal"));
                        }
                        Some(ch) => {
                            s.push(ch);
                            n += 1;
                        }
                    }
                }
                pos.advance_n(n + 1);
                out.push(lexed(Tok::StrLit(s), line, column));
            }
            '\'' => match (at(1), at(2)) {
                (Some(ch), Some('\'')) if ch != '\n' => {
                    pos.advance_n(3);
                    out.push(lexed(Tok::CharLit(ch.to_string()), line, column));
                }
                _ => return Err(err("malformed character literal")),
            },
            '[' => {
                let mut s = String::new();
                let mut n = 1;
                loop {
                    match at(n) {
                        Some(']') => break,
                        Some('\n') | None => {
                            return Err(err("unterminated bracketed name"));
                        }
                        Some(ch) => {
                            s.push(ch);
                            n += 1;
                        }
                    }
                }
                pos.advance_n(n + 1);
                out.push(lexed(Tok::Ident(s), line, column));
            }
            '#' => {
                // HexLiteral = '#'({Digit}[abcdef])+
                let mut n = 1;
                while matches!(at(n), Some(d) if d.is_ascii_digit())
                    && matches!(at(n + 1), Some(h) if matches!(h, 'a'..='f' | 'A'..='F'))
                {
                    n += 2;
                }
                if n == 1 {
                    return Err(err("malformed hex literal"));
                }
                pos.advance_n(n);
                out.push(lexed(Tok::HexLit, line, column));
            }
            '.' if matches!(at(1), Some(d) if d.is_ascii_digit()) => {
                pos.advance(c);
                let end = lex_number(&chars, pos.i, true);
                let mut text = String::from("0.");
                text.extend(&chars[pos.i..end]);
                pos.advance_n(end - pos.i);
                out.push(lexed(real(&text, line, column)?, line, column));
            }
            '.' => {
                pos.advance(c);
                out.push(lexed(Tok::Dot, line, column));
            }
            _ if c.is_ascii_digit() => {
                let end = lex_number(&chars, pos.i, false);
                let text: String = chars[pos.i..end].iter().collect();
                pos.advance_n(end - pos.i);
                out.push(lexed(number(&text, line, column)?, line, column));
            }
            _ if is_id_head(c) => {
                let mut n = 1;
                while matches!(at(n), Some(t) if is_id_tail(t)) {
                    n += 1;
                }
                let name: String = chars[pos.i..pos.i + n].iter().collect();
                pos.advance_n(n);
                let upper = name.to_ascii_uppercase();
                let tok = match upper.as_str() {
                    "AND" => Tok::KwAnd,
                    "OR" => Tok::KwOr,
                    "XOR" => Tok::KwXor,
                    "NOT" => Tok::KwNot,
                    "MOD" => Tok::KwMod,
                    "LIKE" => Tok::KwLike,
                    _ => Tok::Ident(name),
                };
                out.push(lexed(tok, line, column));
            }
            _ => return Err(err(&format!("unexpected character {c:?}"))),
        }
    }
    out.push(lexed(Tok::Eof, pos.line, pos.col));
    Ok(out)
}

fn lexed(tok: Tok, line: u32, column: u32) -> Lexed {
    Lexed { tok, line, column }
}

struct Position {
    i: usize,
    line: u32,
    col: u32,
}

/// Source positions are 1-based — the `LangError::Syntax` contract.
impl Default for Position {
    fn default() -> Self {
        Self {
            i: 0,
            line: 1,
            col: 1,
        }
    }
}

impl Position {
    fn advance(&mut self, c: char) {
        self.i += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
    }
    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            self.i += 1;
            self.col += 1;
        }
    }
}
