//! The `Expr` AST — one node per production of the grammar's
//! `Expression → Value` chain — plus its canonical printing (the form
//! `parse_print_roundtrip` exercises).

use serde::{Deserialize, Serialize};
use std::fmt;

/// A parsed expression (grammar `<Expression>`).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Expr {
    /// A literal.
    Literal(Lit),
    /// A variable reference: a column, a `table.column` qualified
    /// name, a `[bracketed]` name or a `grid[row, col]` field (names
    /// keep their source case; resolution is case-insensitive).
    Ident(String),
    /// `NOT e`, `-e`, `+e`.
    Unary {
        /// The operator.
        op: UnOp,
        /// The operand.
        operand: Box<Expr>,
    },
    /// `a + b`, `a = b`, `a AND b`, `a MOD b`, `a & b`, …
    Binary {
        /// The operator.
        op: BinOp,
        /// Left operand.
        lhs: Box<Expr>,
        /// Right operand.
        rhs: Box<Expr>,
    },
    /// `a LIKE "pat*rn"` — the grammar pins the pattern to a `String`
    /// literal, so the AST keeps the raw pattern text.
    Like {
        /// The value matched against the pattern.
        operand: Box<Expr>,
        /// The `*`-wildcard pattern text.
        pattern: String,
    },
    /// `NAME(args)` or a bare `SYSTEMDATE`/`SYSTEMTIME`/`RECORDCOUNT`.
    Call {
        /// The callee, upper-cased by the parser.
        name: String,
        /// Arguments in order.
        args: Vec<Expr>,
    },
    /// `(a, b, …)` — the grammar's `Expr List`; evaluates to the last
    /// element's value.
    List(Vec<Expr>),
}

/// A literal value (grammar `<Literal>` plus the `Boolean` terminal).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Lit {
    /// `(.)` — missing.
    Missing,
    /// `(+)` / `(-)`.
    Boolean(bool),
    /// Whole number.
    Integer(i64),
    /// Floating point.
    Real(f64),
    /// `"text"` or `'c'`.
    Text(String),
    /// `M/D/YYYY` — days since 1970-01-01.
    Date(i32),
    /// `HH:MM[:SS]` — seconds since midnight.
    Time(i32),
    /// `M/D/YYYY HH:MM[:SS]` — seconds since 1970-01-01.
    DateTime(i64),
}

/// Unary operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnOp {
    /// `-x`.
    Neg,
    /// `+x`.
    Pos,
    /// `NOT x`.
    Not,
}

/// Binary operators, in grammar precedence order (loosest first).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BinOp {
    /// `OR`.
    Or,
    /// `XOR`.
    Xor,
    /// `AND`.
    And,
    /// `=`.
    Eq,
    /// `<>`.
    Ne,
    /// `<`.
    Lt,
    /// `<=`.
    Le,
    /// `>`.
    Gt,
    /// `>=`.
    Ge,
    /// `&` — string concatenation.
    Concat,
    /// `+`.
    Add,
    /// `-`.
    Sub,
    /// `*`.
    Mul,
    /// `/`.
    Div,
    /// `MOD` or `%`.
    Mod,
    /// `^`.
    Pow,
}

impl fmt::Display for Expr {
    /// Canonical spelling: every binary and unary operand is fully
    /// parenthesized, so `parse(print(e))` yields `e` again.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(l) => l.fmt(f),
            Self::Ident(name) => {
                if bare_name(name) {
                    f.write_str(name)
                } else {
                    write!(f, "[{name}]")
                }
            }
            Self::Unary { op, operand } => {
                let kw = match op {
                    UnOp::Neg => "-",
                    UnOp::Pos => "+",
                    UnOp::Not => "NOT ",
                };
                write!(f, "({kw}{operand})")
            }
            Self::Binary { op, lhs, rhs } => {
                let kw = match op {
                    BinOp::Or => "OR",
                    BinOp::Xor => "XOR",
                    BinOp::And => "AND",
                    BinOp::Eq => "=",
                    BinOp::Ne => "<>",
                    BinOp::Lt => "<",
                    BinOp::Le => "<=",
                    BinOp::Gt => ">",
                    BinOp::Ge => ">=",
                    BinOp::Concat => "&",
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Mod => "MOD",
                    BinOp::Pow => "^",
                };
                write!(f, "({lhs} {kw} {rhs})")
            }
            Self::Like { operand, pattern } => write!(f, "({operand} LIKE \"{pattern}\")"),
            Self::Call { name, args } => {
                write!(f, "{name}(")?;
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    arg.fmt(f)?;
                }
                f.write_str(")")
            }
            Self::List(items) => {
                f.write_str("(")?;
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    item.fmt(f)?;
                }
                f.write_str(")")
            }
        }
    }
}

impl fmt::Display for Lit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str("(.)"),
            Self::Boolean(b) => f.write_str(if *b { "(+)" } else { "(-)" }),
            Self::Integer(i) => write!(f, "{i}"),
            Self::Real(x) => {
                // The grammar's `RealLiteral` requires a decimal
                // point; `Display` can print "1e300" or "2" — make
                // them re-lexable.
                let s = format!("{x}");
                match s.find(['e', 'E']) {
                    Some(i) if !s[..i].contains('.') => {
                        write!(f, "{}.0{}", &s[..i], &s[i..])
                    }
                    _ if s.contains(['e', 'E']) || s.contains('.') => f.write_str(&s),
                    _ => write!(f, "{s}.0"),
                }
            }
            Self::Text(s) => write!(f, "\"{s}\""),
            Self::Date(days) => {
                let (y, m, d) = crate::functions::calendar::civil_from_days(*days);
                write!(f, "{m}/{d}/{y:04}")
            }
            Self::Time(secs) => {
                write!(f, "{}:{:02}:{:02}", secs / 3600, secs / 60 % 60, secs % 60)
            }
            Self::DateTime(secs) => {
                let (y, m, d, h, mi, s) = crate::functions::calendar::civil_from_secs(*secs);
                write!(f, "{m}/{d}/{y:04} {h}:{mi:02}:{s:02}")
            }
        }
    }
}

/// Whether `name` survives re-lexing as a bare identifier (the
/// grammar's `{ID Head}{ID Tail}*`); anything else prints bracketed.
fn bare_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if is_id_head(c) => {}
        _ => return false,
    }
    chars.all(is_id_tail)
}

pub(crate) fn is_id_head(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '@' || !c.is_ascii()
}

pub(crate) fn is_id_tail(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '@' | '#' | '$') || !c.is_ascii()
}
