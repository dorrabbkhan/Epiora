//! `eval_column`: evaluates an `Expr` over every row of a `Dataset`.
//! The operator semantics port `Rule_CompareExp`, `Rule_AddExp`,
//! `Rule_MultExp`, `Rule_ConcatExp`, `Rule_AndExp`/`OrExp`/`XorExp` and
//! `Rule_NotExp` from `Epi.Core.AnalysisInterpreter/Rules/` — the same
//! type branches, the same null domain, typed `LangError`s instead of
//! EI7's thrown exceptions.

use super::ast::{BinOp, Expr, Lit, UnOp};
use super::missing::{cell_ev, ei_to_string, ev_type, is_empty, truthy, value_ev, EV};
use super::ops::{add, compare, concat, div, like_match, modulo, mul, parse_f64, pow, sub};
use crate::functions;
use crate::LangError;
use epiora_core::dataset::{ColumnData, Dataset, Value, ValueType};
use rand_chacha::ChaCha20Rng;
use rand_core::{Rng, SeedableRng};
use std::collections::BTreeMap;

/// The recipe's as-of instant — the only clock the engine reads
/// (spec §5.3). `date` is days since 1970-01-01; `seconds` is seconds
/// since midnight.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AsOf {
    /// Days since 1970-01-01.
    pub date: i32,
    /// Seconds since midnight.
    pub seconds: i32,
}

/// The evaluation environment: the recipe's as-of instant plus the
/// session globals captured at recipe start — never live reads
/// (`ENVIRON`, `EXISTS`, `FILEDATE` resolve here; see decisions.md).
#[derive(Clone, Debug, Default)]
pub struct EvalEnv {
    /// The recipe's as-of instant; `SYSTEMDATE`/`SYSTEMTIME` read it.
    pub as_of: AsOf,
    /// Session globals by name. The reserved `environ:<name>`,
    /// `exists:<path>` and `filedate:<path>` keys feed the three
    /// host-reading functions.
    pub globals: BTreeMap<String, Value>,
}

/// Evaluate `e` over `ds` into a result column: per-cell values per
/// the EI7 rules, `None` for missing (EI7's `null` *and* empty text),
/// and the column's `ValueType` inferred from the present values.
pub fn eval_column(e: &Expr, ds: &Dataset, env: &EvalEnv) -> Result<ColumnData, LangError> {
    let mut ctx = Ctx::new(ds, env);
    ctx.seed_rng(e);
    let mut out = Vec::with_capacity(ds.row_count());
    for row in 0..ds.row_count() {
        ctx.row = row;
        out.push(eval(e, &mut ctx)?);
        ctx.prev_result = out.last().cloned();
    }
    Ok(column(out))
}

/// Per-evaluation state; `functions` reads the row context, the
/// seeded stream and the GROUPROWINDEX/LAGVALUE carries.
pub(crate) struct Ctx<'a> {
    pub(crate) ds: &'a Dataset,
    pub(crate) env: &'a EvalEnv,
    /// Upper-cased column name → column index.
    columns: BTreeMap<String, usize>,
    /// The current row.
    pub(crate) row: usize,
    rng: ChaCha20Rng,
    /// The previous row's result of the whole expression — the cell
    /// `PreceedingDataRow[qualifiedId]` reads (GROUPROWINDEX).
    pub(crate) prev_result: Option<EV>,
}

impl<'a> Ctx<'a> {
    fn new(ds: &'a Dataset, env: &'a EvalEnv) -> Self {
        let mut columns = BTreeMap::new();
        for (i, c) in ds.columns().iter().enumerate() {
            columns.entry(c.name.to_ascii_uppercase()).or_insert(i);
        }
        Self {
            ds,
            env,
            columns,
            row: 0,
            rng: ChaCha20Rng::seed_from_u64(0),
            prev_result: None,
        }
    }

    /// RND draws come from a seeded ChaCha20 stream (hard rule 7; the
    /// seed is the recipe's as-of plus the expression's canonical
    /// text, so the same recipe replays identically).
    fn seed_rng(&mut self, e: &Expr) {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        let mut mix = |bytes: &[u8]| {
            for &b in bytes {
                h = (h ^ u64::from(b)).wrapping_mul(0x0000_0100_0000_01b3);
            }
        };
        mix(&self.env.as_of.date.to_le_bytes());
        mix(&self.env.as_of.seconds.to_le_bytes());
        mix(e.to_string().as_bytes());
        self.rng = ChaCha20Rng::seed_from_u64(h);
    }

    /// A bounded draw on `[lower, lower + span)` — Lemire's
    /// nearly-divisionless method, the identical algorithm
    /// `epiora-core`'s `random_numbers` uses on the u32 stream.
    pub(crate) fn draw_bounded(&mut self, lower: i64, span: u64) -> i64 {
        let threshold = ((1u64 << 32) - span) % span;
        loop {
            let m = u64::from(self.rng.next_u32()) * span;
            if (m & 0xffff_ffff) >= threshold {
                return lower + (m >> 32) as i64;
            }
        }
    }

    /// Identifier resolution: dataset column (case-insensitive) first,
    /// then a session global (same rule — EI7's MemoryRegion holds
    /// both). Unknown names are `unknown_variable`.
    pub(crate) fn resolve(&self, name: &str) -> Result<EV, LangError> {
        if let Some(&i) = self.columns.get(&name.to_ascii_uppercase()) {
            return Ok(cell_ev(&self.ds.columns()[i].data, self.row));
        }
        if let Some(v) = self
            .env
            .globals
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
        {
            return Ok(value_ev(v));
        }
        Err(LangError::UnknownVariable {
            name: name.to_string(),
        })
    }

    /// A named cell's value on an arbitrary row (the
    /// `ExecuteOnPreceeding` path LAGVALUE/GROUPROWINDEX use).
    pub(crate) fn resolve_at(&self, name: &str, row: usize) -> Result<EV, LangError> {
        if let Some(&i) = self.columns.get(&name.to_ascii_uppercase()) {
            return Ok(cell_ev(&self.ds.columns()[i].data, row));
        }
        if let Some(v) = self
            .env
            .globals
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
        {
            return Ok(value_ev(v));
        }
        Err(LangError::UnknownVariable {
            name: name.to_string(),
        })
    }

    /// `env.globals` lookup by a prefixed key, case-insensitive.
    pub(crate) fn global(&self, prefix: &str, key: &str) -> Option<&Value> {
        let want = format!("{prefix}:{key}");
        self.env
            .globals
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(&want))
            .map(|(_, v)| v)
    }
}

/// One expression node at the current row.
pub(crate) fn eval(e: &Expr, ctx: &mut Ctx) -> Result<EV, LangError> {
    match e {
        Expr::Literal(l) => Ok(match l {
            Lit::Missing => EV::Missing,
            Lit::Boolean(b) => EV::Bool(*b),
            Lit::Integer(i) => EV::Int(*i),
            Lit::Real(x) => EV::Real(*x),
            Lit::Text(s) => EV::Text(s.clone()),
            Lit::Date(d) => EV::Date(*d),
            Lit::Time(t) => EV::Time(*t),
            Lit::DateTime(s) => EV::DateTime(*s),
        }),
        Expr::Ident(name) => ctx.resolve(name),
        Expr::Unary { op, operand } => {
            let v = eval(operand, ctx)?;
            Ok(match op {
                UnOp::Not => EV::Bool(!truthy(&v)),
                // `Rule_NegateExp` runs `double.Parse(ToString())` — a
                // coerced double, missing when it cannot parse.
                UnOp::Neg => match parse_f64(&ei_to_string(&v)) {
                    Some(x) => EV::Real(-x),
                    None => EV::Missing,
                },
                UnOp::Pos => match parse_f64(&ei_to_string(&v)) {
                    Some(x) => EV::Real(x),
                    None => EV::Missing,
                },
            })
        }
        Expr::Binary { op, lhs, rhs } => {
            let l = eval(lhs, ctx)?;
            let r = eval(rhs, ctx)?;
            Ok(match op {
                BinOp::Or => EV::Bool(truthy(&l) || truthy(&r)),
                BinOp::Xor => EV::Bool(truthy(&l) != truthy(&r)),
                BinOp::And => EV::Bool(truthy(&l) && truthy(&r)),
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    EV::Bool(compare(op, &l, &r))
                }
                BinOp::Concat => concat(&l, &r),
                BinOp::Add => add(&l, &r),
                BinOp::Sub => sub(&l, &r),
                BinOp::Mul => mul(&l, &r),
                BinOp::Div => div(&l, &r),
                BinOp::Mod => modulo(&l, &r),
                BinOp::Pow => pow(&l, &r),
            })
        }
        Expr::Like { operand, pattern } => {
            let l = eval(operand, ctx)?;
            // The `IsEmpty(LHSO) || IsEmpty(RHSO)` branch runs before
            // LIKE's own, so an empty operand is false.
            Ok(EV::Bool(
                !is_empty(&l) && like_match(&ei_to_string(&l), pattern),
            ))
        }
        Expr::Call { name, args } => functions::call(name, args, ctx),
        Expr::List(items) => {
            let mut last = EV::Missing;
            for item in items {
                last = eval(item, ctx)?;
            }
            Ok(last)
        }
    }
}

/// The result column: type inferred from the present values (int+real
/// → real, date+datetime → datetime, other mixes → text), empty text
/// and `Missing` both stored as `None`.
fn column(values: Vec<EV>) -> ColumnData {
    let mut kinds = values
        .iter()
        .filter(|v| !is_empty(v))
        .map(ev_type)
        .collect::<Vec<_>>();
    kinds.sort_by_key(|t| *t as u8);
    kinds.dedup();
    match kinds.as_slice() {
        [] => ColumnData::Boolean(vec![None; values.len()]),
        [ValueType::Integer] => ColumnData::Integer(
            values
                .iter()
                .map(|v| match v {
                    EV::Int(i) => Some(*i),
                    _ => None,
                })
                .collect(),
        ),
        [ValueType::Integer, ValueType::Real] | [ValueType::Real] => ColumnData::Real(
            values
                .iter()
                .map(|v| match v {
                    EV::Int(i) => Some(*i as f64),
                    EV::Real(x) if x.is_finite() => Some(*x),
                    _ => None,
                })
                .collect(),
        ),
        [ValueType::Text] => ColumnData::Text(
            values
                .iter()
                .map(|v| match v {
                    EV::Text(s) if !s.is_empty() => Some(s.clone()),
                    _ => None,
                })
                .collect(),
        ),
        [ValueType::Boolean] => ColumnData::Boolean(
            values
                .iter()
                .map(|v| match v {
                    EV::Bool(b) => Some(*b),
                    _ => None,
                })
                .collect(),
        ),
        [ValueType::Date] => ColumnData::Date(
            values
                .iter()
                .map(|v| match v {
                    EV::Date(d) => Some(*d),
                    _ => None,
                })
                .collect(),
        ),
        [ValueType::Time] => ColumnData::Time(
            values
                .iter()
                .map(|v| match v {
                    EV::Time(t) => Some(*t),
                    _ => None,
                })
                .collect(),
        ),
        [ValueType::Date, ValueType::DateTime] | [ValueType::DateTime] => ColumnData::DateTime(
            values
                .iter()
                .map(|v| match v {
                    EV::Date(d) => Some(i64::from(*d) * 86_400),
                    EV::DateTime(s) => Some(*s),
                    _ => None,
                })
                .collect(),
        ),
        _ => ColumnData::Text(
            values
                .iter()
                .map(|v| {
                    if is_empty(v) {
                        None
                    } else {
                        Some(ei_to_string(v))
                    }
                })
                .collect(),
        ),
    }
}
