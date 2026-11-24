//! Recursive-descent parser for `<Expression>`, one function per
//! non-terminal of the grammar's precedence chain:
//!
//! ```text
//! Expression  → AndExp (OR|XOR) Expression        (right)
//! AndExp      → NotExp AND AndExp                 (right)
//! NotExp      → NOT CompareExp | CompareExp
//! CompareExp  → ConcatExp (cmp | LIKE String) CompareExp   (right)
//! ConcatExp   → AddExp '&' ConcatExp              (right)
//! AddExp      → AddExp (+|−) MultExp              (left)
//! MultExp     → MultExp (*|/|MOD|%) PowExp        (left)
//! PowExp      → NegateExp '^' NegateExp           (non-assoc)
//! NegateExp   → (−|+) Value | Value
//! Value       → QualifiedID | Literal | Boolean | '(' ExprList ')' | Call
//! ```

use super::ast::{BinOp, Expr, Lit, UnOp};
use super::lexer::{lex, Lexed, Tok};
use crate::LangError;

/// Parse `text` as a complete `<Expression>`; trailing tokens are a
/// `pgm_syntax` error.
pub fn parse_expr(text: &str) -> Result<Expr, LangError> {
    let toks = lex(text)?;
    let mut p = Parser { toks: &toks, i: 0 };
    let e = p.expression()?;
    if p.peek() != &Tok::Eof {
        return Err(p.error("unexpected token after expression"));
    }
    Ok(e)
}

struct Parser<'a> {
    toks: &'a [Lexed],
    i: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> &Tok {
        &self.toks[self.i].tok
    }
    fn bump(&mut self) -> &Tok {
        let t = &self.toks[self.i].tok;
        if self.i + 1 < self.toks.len() {
            self.i += 1;
        }
        t
    }
    fn error(&self, message: &str) -> LangError {
        let t = &self.toks[self.i];
        LangError::Syntax {
            line: t.line,
            column: t.column,
            message: message.to_string(),
        }
    }

    /// `<Expression>` — `OR`/`XOR` are right-associative.
    fn expression(&mut self) -> Result<Expr, LangError> {
        let lhs = self.and_exp()?;
        match self.peek() {
            Tok::KwOr => {
                self.bump();
                Ok(binary(BinOp::Or, lhs, self.expression()?))
            }
            Tok::KwXor => {
                self.bump();
                Ok(binary(BinOp::Xor, lhs, self.expression()?))
            }
            _ => Ok(lhs),
        }
    }

    /// `<And Exp>` — `AND` is right-associative.
    fn and_exp(&mut self) -> Result<Expr, LangError> {
        let lhs = self.not_exp()?;
        if self.peek() == &Tok::KwAnd {
            self.bump();
            return Ok(binary(BinOp::And, lhs, self.and_exp()?));
        }
        Ok(lhs)
    }

    /// `<Not Exp>` — `NOT` applies to a `<Compare Exp>` operand.
    fn not_exp(&mut self) -> Result<Expr, LangError> {
        if self.peek() == &Tok::KwNot {
            self.bump();
            return Ok(unary(UnOp::Not, self.compare_exp()?));
        }
        self.compare_exp()
    }

    /// `<Compare Exp>` — comparisons are right-associative; `LIKE`
    /// takes a `String` literal as its pattern.
    fn compare_exp(&mut self) -> Result<Expr, LangError> {
        let lhs = self.concat_exp()?;
        match self.peek() {
            Tok::KwLike => {
                self.bump();
                match self.bump() {
                    Tok::StrLit(pattern) => Ok(Expr::Like {
                        operand: Box::new(lhs),
                        pattern: pattern.clone(),
                    }),
                    _ => Err(self.error("expected a \"pattern\" string after LIKE")),
                }
            }
            Tok::Eq | Tok::Ne | Tok::Lt | Tok::Le | Tok::Gt | Tok::Ge => {
                let op = match self.bump() {
                    Tok::Eq => BinOp::Eq,
                    Tok::Ne => BinOp::Ne,
                    Tok::Lt => BinOp::Lt,
                    Tok::Le => BinOp::Le,
                    Tok::Gt => BinOp::Gt,
                    _ => BinOp::Ge,
                };
                Ok(binary(op, lhs, self.compare_exp()?))
            }
            _ => Ok(lhs),
        }
    }

    /// `<Concat Exp>` — `&` is right-associative.
    fn concat_exp(&mut self) -> Result<Expr, LangError> {
        let lhs = self.add_exp()?;
        if self.peek() == &Tok::Amp {
            self.bump();
            return Ok(binary(BinOp::Concat, lhs, self.concat_exp()?));
        }
        Ok(lhs)
    }

    /// `<Add Exp>` — `+`/`−` are left-associative.
    fn add_exp(&mut self) -> Result<Expr, LangError> {
        let mut lhs = self.mult_exp()?;
        loop {
            let op = match self.peek() {
                Tok::Plus => BinOp::Add,
                Tok::Minus => BinOp::Sub,
                _ => return Ok(lhs),
            };
            self.bump();
            lhs = binary(op, lhs, self.mult_exp()?);
        }
    }

    /// `<Mult Exp>` — `*` `/` `MOD` `%` are left-associative.
    fn mult_exp(&mut self) -> Result<Expr, LangError> {
        let mut lhs = self.pow_exp()?;
        loop {
            let op = match self.peek() {
                Tok::Star => BinOp::Mul,
                Tok::Slash => BinOp::Div,
                Tok::KwMod | Tok::Percent => BinOp::Mod,
                _ => return Ok(lhs),
            };
            self.bump();
            lhs = binary(op, lhs, self.pow_exp()?);
        }
    }

    /// `<Pow Exp>` — `^` takes two `<Negate Exp>` operands and is not
    /// associative (`a^b^c` is a syntax error).
    fn pow_exp(&mut self) -> Result<Expr, LangError> {
        let lhs = self.negate_exp()?;
        if self.peek() == &Tok::Caret {
            self.bump();
            return Ok(binary(BinOp::Pow, lhs, self.negate_exp()?));
        }
        Ok(lhs)
    }

    /// `<Negate Exp>` — unary `+`/`−` binds to a single `<Value>`.
    fn negate_exp(&mut self) -> Result<Expr, LangError> {
        match self.peek() {
            Tok::Minus => {
                self.bump();
                Ok(unary(UnOp::Neg, self.value()?))
            }
            Tok::Plus => {
                self.bump();
                Ok(unary(UnOp::Pos, self.value()?))
            }
            _ => self.value(),
        }
    }

    /// `<Value>` — identifiers, calls, literals, `(expr, …)` lists.
    fn value(&mut self) -> Result<Expr, LangError> {
        match self.peek().clone() {
            Tok::BoolLit(b) => {
                self.bump();
                Ok(Expr::Literal(Lit::Boolean(b)))
            }
            Tok::MissingLit | Tok::HexLit => {
                self.bump();
                Ok(Expr::Literal(Lit::Missing))
            }
            Tok::IntLit(i) => {
                self.bump();
                Ok(Expr::Literal(Lit::Integer(i)))
            }
            Tok::RealLit(x) => {
                self.bump();
                Ok(Expr::Literal(Lit::Real(x)))
            }
            Tok::StrLit(s) | Tok::CharLit(s) => {
                self.bump();
                Ok(Expr::Literal(Lit::Text(s)))
            }
            Tok::DateLit(days) => {
                self.bump();
                // `<Literal_Date_Time> ::= Date Time`
                if let Tok::TimeLit(secs) = self.peek().clone() {
                    self.bump();
                    let dt = match (days, secs) {
                        (Some(d), Some(s)) => Some(i64::from(d) * 86_400 + i64::from(s)),
                        _ => None,
                    };
                    return Ok(Expr::Literal(match dt {
                        Some(secs) => Lit::DateTime(secs),
                        None => Lit::Missing,
                    }));
                }
                Ok(Expr::Literal(match days {
                    Some(d) => Lit::Date(d),
                    None => Lit::Missing,
                }))
            }
            Tok::TimeLit(secs) => {
                self.bump();
                Ok(Expr::Literal(match secs {
                    Some(s) => Lit::Time(s),
                    None => Lit::Missing,
                }))
            }
            Tok::Ident(name) => {
                self.bump();
                // `Identifier '.' <Qualified ID>` chains.
                let mut name = name;
                while self.peek() == &Tok::Dot {
                    self.bump();
                    match self.peek().clone() {
                        Tok::Ident(part) => {
                            self.bump();
                            name.push('.');
                            name.push_str(&part);
                        }
                        _ => return Err(self.error("expected a name after '.'")),
                    }
                }
                if self.peek() == &Tok::LParen {
                    self.bump();
                    let args = self.parameter_list()?;
                    return Ok(Expr::Call {
                        name: name.to_ascii_uppercase(),
                        args,
                    });
                }
                // `<FuncName2>`: bare nullary functions.
                let upper = name.to_ascii_uppercase();
                if matches!(upper.as_str(), "SYSTEMDATE" | "SYSTEMTIME" | "RECORDCOUNT") {
                    return Ok(Expr::Call {
                        name: upper,
                        args: Vec::new(),
                    });
                }
                Ok(Expr::Ident(name))
            }
            Tok::LParen => {
                self.bump();
                let mut items = vec![self.expression()?];
                while self.peek() == &Tok::Comma {
                    self.bump();
                    items.push(self.expression()?);
                }
                match self.bump() {
                    Tok::RParen => {}
                    _ => return Err(self.error("expected ')'")),
                }
                match items.len() {
                    1 => Ok(items
                        .into_iter()
                        .next()
                        .unwrap_or(Expr::Literal(Lit::Missing))),
                    _ => Ok(Expr::List(items)),
                }
            }
            _ => Err(self.error("expected a value")),
        }
    }

    /// `<FunctionParameterList>` — comma-separated `<Expression>`s.
    fn parameter_list(&mut self) -> Result<Vec<Expr>, LangError> {
        if self.peek() == &Tok::RParen {
            self.bump();
            return Ok(Vec::new());
        }
        let mut args = vec![self.expression()?];
        while self.peek() == &Tok::Comma {
            self.bump();
            args.push(self.expression()?);
        }
        match self.bump() {
            Tok::RParen => Ok(args),
            _ => Err(self.error("expected ',' or ')' in the argument list")),
        }
    }
}

fn unary(op: UnOp, operand: Expr) -> Expr {
    Expr::Unary {
        op,
        operand: Box::new(operand),
    }
}

fn binary(op: BinOp, lhs: Expr, rhs: Expr) -> Expr {
    Expr::Binary {
        op,
        lhs: Box::new(lhs),
        rhs: Box::new(rhs),
    }
}
