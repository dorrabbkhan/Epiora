//! The `.pgm` lexer — one line at a time, so a statement always ends
//! at its line's end (EI7 commands are line-bounded). On top of the
//! expression terminals it adds the `File` (`'…'`), `{brace}` string,
//! `::`/`:` and `!` (DLL call) terminals; `***` and a leading `*` are
//! comments, `//` comments out the rest of its line, and
//! `*`-comments are how EI3 saved programs mark disabled lines
//! (`*assign x=1`).

use crate::expr::ast::{is_id_head, is_id_tail};
use crate::expr::lit::{lex_number, number, real};
use crate::LangError;

/// One lexed token with its position and byte span in the program.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Lexed {
    /// The token.
    pub tok: Tok,
    /// 1-based line.
    pub line: u32,
    /// 1-based column (characters).
    pub column: u32,
    /// Byte offset of the token's first byte in the program text.
    pub start: usize,
    /// Byte offset one past the token's last byte.
    pub end: usize,
}

/// Tokens, mirroring the Analysis grammar's statement terminals.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Tok {
    Ident(String),
    /// `"…"` — the String terminal.
    Str(String),
    /// `'…'` — the File terminal (any length).
    File(String),
    /// `{…}` — the BraceString terminal.
    Brace(String),
    Int(i64),
    Real(f64),
    /// `M/D/Y` — `None` when the calendar date is invalid.
    Date(Option<i32>),
    /// `H:MM[:SS]`.
    Time(Option<i32>),
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
    Colon,
    /// `::` — MERGE/RELATE/SUMMARIZE keys.
    DColon,
    /// `!` — DLL member access (`var = class!method(…)`).
    Bang,
}

/// Lex one source line (no newline characters) into tokens; `base`
/// is the line's byte offset in the program text. A `*` or `***`
/// comment run or a `//` comment ends the line.
pub(crate) fn lex_line(text: &str, line: u32, base: usize) -> Result<Vec<Lexed>, LangError> {
    let chars: Vec<char> = text.chars().collect();
    let bytes: Vec<usize> = text.char_indices().map(|(i, _)| base + i).collect();
    let end_byte = base + text.len();
    let mut out = Vec::new();
    let mut i = 0usize;
    let push = |out: &mut Vec<Lexed>, tok: Tok, i0: usize, i1: usize| {
        out.push(Lexed {
            tok,
            line,
            column: (i0 + 1) as u32,
            start: bytes[i0],
            end: bytes.get(i1).copied().unwrap_or(end_byte),
        });
    };
    // A line whose first non-blank character is `*` is a comment
    // (`***` comments included — EI3's `*command` disabled lines too).
    let mut k = 0usize;
    while matches!(chars.get(k), Some(' ' | '\t')) {
        k += 1;
    }
    if chars.get(k) == Some(&'*') {
        return Ok(out);
    }
    let err = |i0: usize, message: &str| LangError::Syntax {
        line,
        column: (i0 + 1) as u32,
        message: message.to_string(),
    };
    while let Some(&c) = chars.get(i) {
        let i0 = i;
        let at = |n: usize| chars.get(i + n).copied();
        match c {
            ' ' | '\t' | '\r' | '\u{feff}' => i += 1,
            '*' => {
                // `***` (any run ≥ 3) anywhere ends the line.
                let mut n = 1;
                while at(n) == Some('*') {
                    n += 1;
                }
                if n >= 3 {
                    break;
                }
                push(&mut out, Tok::Star, i0, i0 + 1);
                i += n;
            }
            // `//` comments out the rest of the line — a whole-line
            // comment lexes to nothing, a trailing one ends the
            // statement's tokens. String, file, brace and `[…]`
            // arms consume their contents whole, so a `//` inside
            // them never reaches here.
            '/' if at(1) == Some('/') => break,
            '(' | ')' | ',' | '=' | '&' | '+' | '-' | '/' | '%' | '^' | '!' => {
                let tok = match c {
                    '(' => Tok::LParen,
                    ')' => Tok::RParen,
                    ',' => Tok::Comma,
                    '=' => Tok::Eq,
                    '&' => Tok::Amp,
                    '+' => Tok::Plus,
                    '-' => Tok::Minus,
                    '/' => Tok::Slash,
                    '%' => Tok::Percent,
                    '^' => Tok::Caret,
                    _ => Tok::Bang,
                };
                push(&mut out, tok, i0, i0 + 1);
                i += 1;
            }
            '<' => {
                let (tok, n) = match at(1) {
                    Some('>') => (Tok::Ne, 2),
                    Some('=') => (Tok::Le, 2),
                    _ => (Tok::Lt, 1),
                };
                push(&mut out, tok, i0, i0 + n);
                i += n;
            }
            '>' => {
                let n = if at(1) == Some('=') { 2 } else { 1 };
                push(&mut out, if n == 2 { Tok::Ge } else { Tok::Gt }, i0, i0 + n);
                i += n;
            }
            ':' => {
                let (tok, n) = if at(1) == Some(':') {
                    (Tok::DColon, 2)
                } else {
                    (Tok::Colon, 1)
                };
                push(&mut out, tok, i0, i0 + n);
                i += n;
            }
            '.' if matches!(at(1), Some(d) if d.is_ascii_digit()) => {
                i += 1;
                let end = lex_number(&chars, i, true);
                let mut s = String::from("0.");
                s.extend(&chars[i..end]);
                let t = lit(real(&s, line, (i0 + 1) as u32), line, (i0 + 1) as u32)?;
                push(&mut out, t, i0, end);
                i = end;
            }
            '.' => {
                push(&mut out, Tok::Dot, i0, i0 + 1);
                i += 1;
            }
            '"' => {
                let mut s = String::new();
                let mut n = 1;
                loop {
                    match at(n) {
                        Some('"') => break,
                        Some('\n') | None => {
                            return Err(err(i0, "unterminated string literal"));
                        }
                        Some(ch) => {
                            s.push(ch);
                            n += 1;
                        }
                    }
                }
                push(&mut out, Tok::Str(s), i0, i0 + n + 1);
                i += n + 1;
            }
            '\'' => {
                let mut s = String::new();
                let mut n = 1;
                loop {
                    match at(n) {
                        Some('\'') => break,
                        Some('\n') | None => {
                            return Err(err(i0, "unterminated file literal"));
                        }
                        Some(ch) => {
                            s.push(ch);
                            n += 1;
                        }
                    }
                }
                push(&mut out, Tok::File(s), i0, i0 + n + 1);
                i += n + 1;
            }
            '{' => {
                let mut s = String::new();
                let mut n = 1;
                loop {
                    match at(n) {
                        Some('}') => break,
                        Some('\n') | None => {
                            return Err(err(i0, "unterminated brace string"));
                        }
                        Some(ch) => {
                            s.push(ch);
                            n += 1;
                        }
                    }
                }
                push(&mut out, Tok::Brace(s), i0, i0 + n + 1);
                i += n + 1;
            }
            '[' => {
                let mut s = String::new();
                let mut n = 1;
                loop {
                    match at(n) {
                        Some(']') => break,
                        Some('\n') | None => {
                            return Err(err(i0, "unterminated bracketed name"));
                        }
                        Some(ch) => {
                            s.push(ch);
                            n += 1;
                        }
                    }
                }
                push(&mut out, Tok::Ident(s), i0, i0 + n + 1);
                i += n + 1;
            }
            _ if c.is_ascii_digit() => {
                let end = lex_number(&chars, i, false);
                let s: String = chars[i..end].iter().collect();
                let t = lit(number(&s, line, (i0 + 1) as u32), line, (i0 + 1) as u32)?;
                push(&mut out, t, i0, end);
                i = end;
            }
            _ if is_id_head(c) => {
                let mut n = 1;
                while matches!(at(n), Some(t) if is_id_tail(t)) {
                    n += 1;
                }
                let name: String = chars[i..i + n].iter().collect();
                push(&mut out, Tok::Ident(name), i0, i0 + n);
                i += n;
            }
            _ => return Err(err(i0, &format!("unexpected character {c:?}"))),
        }
    }
    Ok(out)
}

/// Map the expression lexer's literal token onto a pgm token.
fn lit(
    tok: Result<crate::expr::lexer::Tok, LangError>,
    line: u32,
    column: u32,
) -> Result<Tok, LangError> {
    use crate::expr::lexer::Tok as E;
    Ok(match tok? {
        E::IntLit(i) => Tok::Int(i),
        E::RealLit(x) => Tok::Real(x),
        E::DateLit(d) => Tok::Date(d),
        E::TimeLit(t) => Tok::Time(t),
        // `lex_number` + `number`/`real` only ever produce the four
        // literal tokens above.
        other => {
            return Err(LangError::Syntax {
                line,
                column,
                message: format!("malformed number (token {other:?})"),
            })
        }
    })
}
