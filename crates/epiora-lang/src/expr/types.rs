//! `Schema` (the names a `type_check` can see) and the static type
//! pass. The checker is strict only where Epi Info's own typing makes
//! a value impossible: operands typed for CLR branches (the `is
//! DateTime`/`is Double` tests), arities and identifier-only arguments
//! are `expr_type` errors naming the column; everything EI7 coerces
//! through `ToString()`/`TryParse` stays accepted.

use super::ast::{BinOp, Expr, Lit, UnOp};
use crate::functions::{arg_kind, function_spec, ArgKind};
use crate::LangError;
use epiora_core::dataset::{Dataset, ValueType};
use std::collections::BTreeMap;

/// A name → [`ValueType`] map for `type_check`: dataset columns plus
/// any session globals the caller adds.
#[derive(Clone, Debug, Default)]
pub struct Schema {
    /// Keyed by upper-case name; values keep the source spelling.
    fields: BTreeMap<String, (String, ValueType)>,
}

impl Schema {
    /// Every column of `ds`, case-insensitively.
    pub fn from_dataset(ds: &Dataset) -> Self {
        let mut schema = Self::default();
        for column in ds.columns() {
            schema.insert(column.name.clone(), column.data.value_type());
        }
        schema
    }

    /// Adds or replaces a name (a defined or global variable).
    pub fn insert(&mut self, name: impl Into<String>, ty: ValueType) {
        let name = name.into();
        self.fields.insert(name.to_ascii_uppercase(), (name, ty));
    }

    /// The stored type and source spelling of `name`, or `None`.
    pub fn lookup(&self, name: &str) -> Option<(&str, ValueType)> {
        self.fields
            .get(&name.to_ascii_uppercase())
            .map(|(n, t)| (n.as_str(), *t))
    }
}

/// The expression's [`ValueType`], or a typed `LangError`. `Lit::Missing`
/// types as `Boolean` — `(.)` is the grammar's `Boolean` terminal —
/// and is a wildcard operand (it evals to missing everywhere).
pub fn type_check(e: &Expr, schema: &Schema) -> Result<ValueType, LangError> {
    match e {
        Expr::Literal(l) => Ok(match l {
            Lit::Missing | Lit::Boolean(_) => ValueType::Boolean,
            Lit::Integer(_) => ValueType::Integer,
            Lit::Real(_) => ValueType::Real,
            Lit::Text(_) => ValueType::Text,
            Lit::Date(_) => ValueType::Date,
            Lit::Time(_) => ValueType::Time,
            Lit::DateTime(_) => ValueType::DateTime,
        }),
        Expr::Ident(name) => schema
            .lookup(name)
            .map(|(_, t)| t)
            .ok_or_else(|| LangError::UnknownVariable { name: name.clone() }),
        Expr::Unary { op, operand } => {
            let t = type_check(operand, schema)?;
            match op {
                UnOp::Not => Ok(ValueType::Boolean),
                // `NegateExp`/`PosExp` run `double.Parse(ToString())`:
                // text parses, the result is always a `double`.
                UnOp::Neg | UnOp::Pos => match t {
                    ValueType::Integer | ValueType::Real | ValueType::Text => Ok(ValueType::Real),
                    ValueType::Boolean if is_missing(operand) => Ok(ValueType::Real),
                    _ => Err(type_error(operand, "a number", t)),
                },
            }
        }
        Expr::Binary { op, lhs, rhs } => {
            let lt = type_check(lhs, schema)?;
            let rt = type_check(rhs, schema)?;
            let lmiss = is_missing(lhs);
            let rmiss = is_missing(rhs);
            match op {
                BinOp::Or
                | BinOp::Xor
                | BinOp::And
                | BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge => Ok(ValueType::Boolean),
                BinOp::Concat => Ok(ValueType::Text),
                BinOp::Add | BinOp::Sub => {
                    if lmiss {
                        return Ok(rt);
                    }
                    if rmiss {
                        return Ok(lt);
                    }
                    additive(op, lhs, lt, rhs, rt)
                }
                // `MultExp` accepts `Double` and `TimeSpan` (a `Time`
                // contributes `TotalDays`); `PowExp` parses either
                // operand's `ToString()`, so text is parseable.
                BinOp::Mul | BinOp::Div | BinOp::Mod => {
                    let ok =
                        |t| matches!(t, ValueType::Integer | ValueType::Real | ValueType::Time);
                    if !lmiss && !ok(lt) {
                        return Err(type_error(lhs, "a number or time", lt));
                    }
                    if !rmiss && !ok(rt) {
                        return Err(type_error(rhs, "a number or time", rt));
                    }
                    Ok(
                        if matches!(op, BinOp::Mul) && lt == rt && lt == ValueType::Integer {
                            ValueType::Integer
                        } else {
                            ValueType::Real
                        },
                    )
                }
                BinOp::Pow => {
                    let ok =
                        |t| matches!(t, ValueType::Integer | ValueType::Real | ValueType::Text);
                    if !lmiss && !ok(lt) {
                        return Err(type_error(lhs, "a parseable number", lt));
                    }
                    if !rmiss && !ok(rt) {
                        return Err(type_error(rhs, "a parseable number", rt));
                    }
                    Ok(ValueType::Real)
                }
            }
        }
        Expr::Like { .. } => Ok(ValueType::Boolean),
        Expr::Call { name, args } => {
            let spec = function_spec(name)
                .ok_or_else(|| LangError::UnknownFunction { name: name.clone() })?;
            if !spec.arity.contains(&args.len()) {
                return Err(LangError::ExprType {
                    column: name.clone(),
                    expected: format!("{} argument(s)", spec.arity_label()),
                    found: args.len().to_string(),
                });
            }
            for (i, (arg, kind)) in args.iter().zip(arg_kind(spec, args.len())).enumerate() {
                let t = type_check(arg, schema)?;
                if is_missing(arg) {
                    continue;
                }
                let ok = match kind {
                    ArgKind::Num => {
                        matches!(t, ValueType::Integer | ValueType::Real | ValueType::Text)
                    }
                    ArgKind::DateT => matches!(t, ValueType::Date | ValueType::DateTime),
                    ArgKind::DateTimeT => {
                        matches!(t, ValueType::Date | ValueType::DateTime | ValueType::Time)
                    }
                    ArgKind::Ident => matches!(arg, Expr::Ident(_)),
                    ArgKind::Any => true,
                };
                if !ok {
                    let found = if matches!(kind, ArgKind::Ident) {
                        "an expression".to_string()
                    } else {
                        type_label(t).to_string()
                    };
                    return Err(LangError::ExprType {
                        column: describe(arg),
                        expected: format!("{} (argument {})", kind.label(), i + 1),
                        found,
                    });
                }
            }
            Ok(spec.result)
        }
        Expr::List(items) => items
            .last()
            .map_or(Ok(ValueType::Boolean), |e| type_check(e, schema)),
    }
}

/// `<Add Exp>`/`−` typing — EI7's `is Double`/`is DateTime` branches:
/// numbers add to numbers, a number of days adds to a date/time, two
/// date-likes subtract to a `double` day count. Everything else is a
/// branch EI7 leaves `null` — a type error here, honest and early.
fn additive(
    op: &BinOp,
    lhs: &Expr,
    lt: ValueType,
    rhs: &Expr,
    rt: ValueType,
) -> Result<ValueType, LangError> {
    let num = |t| matches!(t, ValueType::Integer | ValueType::Real);
    let datetime = |t| matches!(t, ValueType::Date | ValueType::DateTime);
    match (lt, rt) {
        (ValueType::Integer, ValueType::Integer) => Ok(ValueType::Integer),
        (a, b) if num(a) && num(b) => Ok(ValueType::Real),
        (ValueType::Date, ValueType::Integer) => Ok(ValueType::Date),
        (ValueType::Date, b) | (ValueType::DateTime, b) if num(b) => Ok(ValueType::DateTime),
        (a, b) if datetime(a) && datetime(b) && matches!(op, BinOp::Sub) => Ok(ValueType::Real),
        (a, b) if num(a) && datetime(b) => Err(type_error(lhs, "not a number beside a date", a)),
        _ => {
            let (e, t) = if num(lt) || datetime(lt) {
                (rhs, rt)
            } else {
                (lhs, lt)
            };
            Err(type_error(e, "a number or a date", t))
        }
    }
}

/// `(.)` is the wildcard operand.
fn is_missing(e: &Expr) -> bool {
    matches!(e, Expr::Literal(Lit::Missing))
}

fn type_error(e: &Expr, expected: &str, found: ValueType) -> LangError {
    LangError::ExprType {
        column: describe(e),
        expected: expected.to_string(),
        found: type_label(found).to_string(),
    }
}

/// The name a `type_check` error carries — the column when the faulty
/// operand is a variable, else the canonical sub-expression.
fn describe(e: &Expr) -> String {
    match e {
        Expr::Ident(name) => name.clone(),
        _ => e.to_string(),
    }
}

fn type_label(t: ValueType) -> &'static str {
    match t {
        ValueType::Integer => "integer",
        ValueType::Real => "real",
        ValueType::Text => "text",
        ValueType::Boolean => "boolean",
        ValueType::Date => "date",
        ValueType::Time => "time",
        ValueType::DateTime => "date_time",
    }
}
