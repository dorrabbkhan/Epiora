//! Fuzz harness (v1.1 spec §6.5): arbitrary bytes decode into a small
//! dataset plus a random `Command` sequence, which [`crate::replay`]
//! runs twice — the two materialized views must hash identically, and
//! a failure must surface as a `LangError`, never a panic.

use crate::{
    replay, AsOf, BinOp, Command, DatasetRef, Expr, LinkedDatasets, Lit, MergeMode, Order, Process,
    Recipe, Scope, SetOption,
};
use arbitrary::{Arbitrary, Result, Unstructured};
use epiora_core::dataset::{Column, ColumnData, Dataset, DatasetLink, ValueType};

const MAX_COLUMNS: usize = 6;
const MAX_ROWS: usize = 32;
const MAX_STEPS: usize = 16;

fn as_of() -> AsOf {
    AsOf {
        date: 20_000,
        seconds: 0,
    }
}

struct NoLinks;

impl LinkedDatasets for NoLinks {
    fn get(&self, _name: &str) -> Option<&Dataset> {
        None
    }
    fn link(&self, _p: &str, _c: &str) -> Option<&DatasetLink> {
        None
    }
}

/// A small arbitrary column: integers, reals or short texts.
#[derive(Debug)]
enum FuzzData {
    Integer(Vec<Option<i64>>),
    Real(Vec<Option<f64>>),
    Text(Vec<Option<String>>),
}

impl<'a> Arbitrary<'a> for FuzzData {
    fn arbitrary(u: &mut Unstructured<'a>) -> Result<Self> {
        let n = u.int_in_range(0usize..=MAX_ROWS)?;
        Ok(match u.int_in_range(0u8..=2)? {
            0 => Self::Integer(
                (0..n)
                    .map(|_| {
                        Ok(if u.arbitrary::<bool>()? {
                            Some(u.arbitrary::<i64>()? % 1000)
                        } else {
                            None
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            1 => Self::Real(
                (0..n)
                    .map(|_| {
                        Ok(if u.arbitrary::<bool>()? {
                            Some(u.arbitrary::<u32>()? as f64 / 8.0)
                        } else {
                            None
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            _ => Self::Text(
                (0..n)
                    .map(|_| -> Result<Option<String>> {
                        if u.arbitrary::<bool>()? {
                            let len = u.int_in_range(0usize..=8)?;
                            let mut s = String::new();
                            for _ in 0..len {
                                s.push(char::from(b'a' + (u.arbitrary::<u8>()? % 26)));
                            }
                            Ok(Some(s))
                        } else {
                            Ok(None)
                        }
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
        })
    }
}

fn build_dataset(u: &mut Unstructured<'_>) -> Result<Dataset> {
    let ncols = u.int_in_range(1usize..=MAX_COLUMNS)?;
    let mut cols = Vec::new();
    let mut rows = 0usize;
    for i in 0..ncols {
        let data = match FuzzData::arbitrary(u)? {
            FuzzData::Integer(v) => {
                rows = rows.max(v.len());
                ColumnData::Integer(v)
            }
            FuzzData::Real(v) => {
                rows = rows.max(v.len());
                ColumnData::Real(v)
            }
            FuzzData::Text(v) => {
                rows = rows.max(v.len());
                ColumnData::Text(v)
            }
        };
        cols.push(Column {
            name: format!("C{i}"),
            data,
        });
    }
    // Equalize lengths — columns decoded independently.
    fn pad<T: Clone>(v: &mut Vec<Option<T>>, rows: usize) {
        v.resize(rows, None);
    }
    for c in &mut cols {
        match &mut c.data {
            ColumnData::Integer(v) => v.resize(rows, None),
            ColumnData::Real(v) => v.resize(rows, None),
            ColumnData::Text(v) => v.resize(rows, None),
            ColumnData::Boolean(v) => pad(v, rows),
            ColumnData::Date(v) => pad(v, rows),
            ColumnData::Time(v) => pad(v, rows),
            ColumnData::DateTime(v) => pad(v, rows),
        }
    }
    Dataset::new("fuzz", 1, cols).map_err(|_| arbitrary::Error::IncorrectFormat)
}

/// One decoded command — column references resolved against the
/// dataset's real names so some commands hit and some miss.
fn any_command(u: &mut Unstructured<'_>, names: &[String]) -> Result<Command> {
    let pick = |u: &mut Unstructured<'_>| -> Result<String> {
        if names.is_empty() {
            return Err(arbitrary::Error::IncorrectFormat);
        }
        Ok(names[u.int_in_range(0usize..=names.len() - 1)?].clone())
    };
    let atom = |u: &mut Unstructured<'_>| -> Result<Expr> {
        Ok(match u.int_in_range(0u8..=4)? {
            0 => Expr::Ident(pick(u)?),
            1 => Expr::Literal(Lit::Integer(u.arbitrary::<i64>()? % 100)),
            2 => Expr::Literal(Lit::Real(u.arbitrary::<u8>()? as f64 / 4.0)),
            3 => Expr::Literal(Lit::Text(format!("t{}", u.arbitrary::<u8>()? % 16))),
            _ => Expr::Literal(Lit::Missing),
        })
    };
    let cmp = |u: &mut Unstructured<'_>| -> Result<Expr> {
        Ok(Expr::Binary {
            op: match u.int_in_range(0u8..=5)? {
                0 => BinOp::Eq,
                1 => BinOp::Ne,
                2 => BinOp::Lt,
                3 => BinOp::Le,
                4 => BinOp::Gt,
                _ => BinOp::Ge,
            },
            lhs: Box::new(atom(u)?),
            rhs: Box::new(atom(u)?),
        })
    };
    Ok(match u.int_in_range(0u8..=11)? {
        0 => Command::Select(cmp(u)?),
        1 => Command::CancelSelect,
        2 => Command::Sort(vec![(
            pick(u)?,
            if u.arbitrary::<bool>()? {
                Order::Asc
            } else {
                Order::Desc
            },
        )]),
        3 => Command::CancelSort,
        4 => Command::Define {
            name: pick(u)?,
            ty: ValueType::Real,
            scope: Scope::Standard,
        },
        5 => Command::Assign {
            name: pick(u)?,
            expr: atom(u)?,
        },
        6 => Command::DeleteRecords(cmp(u)?),
        7 => Command::Undelete(if u.arbitrary::<bool>()? {
            None
        } else {
            Some(cmp(u)?)
        }),
        8 => Command::DeleteVariables(vec![pick(u)?]),
        9 => Command::KeyVars(vec![pick(u)?]),
        10 => Command::Set(if u.arbitrary::<bool>()? {
            SetOption::Missing(u.arbitrary::<bool>()?)
        } else {
            SetOption::Process(match u.int_in_range(0u8..=2)? {
                0 => Process::Both,
                1 => Process::Undeleted,
                _ => Process::Deleted,
            })
        }),
        _ => Command::Merge {
            dataset: DatasetRef::named("nowhere"),
            keys: vec![(pick(u)?, pick(u)?)],
            mode: MergeMode::MatchOrInsert,
        },
    })
}

/// Random dataset + random command sequence into `replay` (v1.1 plan
/// Task 11): replayed twice, the materialized view hashes must agree;
/// a failure must be a typed `LangError`, never a panic. The
/// assertions are the fuzz crash signal; the scoped allow lifts the
/// crate's `clippy::panic` deny for exactly them.
#[allow(clippy::panic)]
pub fn recipe_any(data: &[u8]) {
    let mut u = Unstructured::new(data);
    let Ok(ds) = build_dataset(&mut u) else {
        return;
    };
    let names: Vec<String> = ds.columns().iter().map(|c| c.name.clone()).collect();
    let Ok(nsteps) = u.int_in_range(0usize..=MAX_STEPS) else {
        return;
    };
    let Ok(steps) = (0..nsteps)
        .map(|_| any_command(&mut u, &names))
        .collect::<Result<Vec<Command>>>()
    else {
        return;
    };
    let recipe = Recipe {
        as_of: as_of(),
        steps,
    };
    let _ = recipe.canonical_text();
    match (
        replay(&ds, &NoLinks, &recipe),
        replay(&ds, &NoLinks, &recipe),
    ) {
        (Ok(a), Ok(b)) => {
            assert_eq!(
                a.materialize().content_hash_hex(),
                b.materialize().content_hash_hex(),
                "replay must be deterministic"
            );
        }
        (Err(ea), Err(eb)) => assert_eq!(ea.code(), eb.code()),
        _ => panic!("replay must be deterministic"),
    }
}
