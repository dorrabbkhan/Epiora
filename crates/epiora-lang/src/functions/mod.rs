//! The function library: every `supported@1.1` function in
//! `register.json`, dispatched by upper-case name. Each entry ports
//! the matching `Epi.Core.AnalysisInterpreter/Rules/Rule_*.cs`
//! implementation — argument coercion, null/missing behavior and the
//! arithmetic — through `libm` where floats are involved.

pub(crate) mod date;
pub(crate) mod math;
mod spec;
pub(crate) mod text;

pub(crate) use spec::{arg_kind, function_spec, ArgKind};

use crate::expr::eval::{eval, Ctx};
use crate::expr::missing::{ei_to_string, EV};
use crate::Expr;
use crate::LangError;

/// Evaluate a call at the current row.
pub(crate) fn call(name: &str, args: &[Expr], ctx: &mut Ctx) -> Result<EV, LangError> {
    let spec = function_spec(name).ok_or_else(|| LangError::UnknownFunction {
        name: name.to_string(),
    })?;
    if !spec.arity.contains(&args.len()) {
        return Err(LangError::ExprType {
            column: name.to_string(),
            expected: spec.arity_label(),
            found: args.len().to_string(),
        });
    }
    // EI7 throws "no longer supported by Epi Info" for both.
    if matches!(name, "PFROMZ" | "ZSCORE") {
        return Err(LangError::UnsupportedFunction {
            name: name.to_string(),
        });
    }
    match name {
        // The preceding-row functions need the identifier, not a value.
        "GROUPROWINDEX" | "LAGVALUE" => {
            let Expr::Ident(ident) = &args[0] else {
                return Err(LangError::ExprType {
                    column: args[0].to_string(),
                    expected: "a variable".to_string(),
                    found: "an expression".to_string(),
                });
            };
            Ok(match name {
                "LAGVALUE" => {
                    if ctx.row == 0 {
                        EV::Missing
                    } else {
                        EV::Text(ei_to_string(&ctx.resolve_at(ident, ctx.row - 1)?))
                    }
                }
                _ => {
                    // GROUPROWINDEX: equal current/preceding argument
                    // text increments the preceding row's own result.
                    let cur = ei_to_string(&eval(&args[0], ctx)?);
                    let result = if ctx.row == 0 {
                        1
                    } else {
                        let prev = ei_to_string(&ctx.resolve_at(ident, ctx.row - 1)?);
                        if cur == prev {
                            ctx.prev_result
                                .as_ref()
                                .and_then(|v| ei_to_string(v).parse::<i64>().ok())
                                .map_or(1, |p| p + 1)
                        } else {
                            1
                        }
                    };
                    EV::Int(result)
                }
            })
        }
        _ => {
            let mut v = Vec::with_capacity(args.len());
            for arg in args {
                v.push(eval(arg, ctx)?);
            }
            Ok(match name {
                "ABS" => math::scalar(&v[0], libm::fabs),
                "COS" => math::scalar(&v[0], libm::cos),
                "SIN" => math::scalar(&v[0], libm::sin),
                "TAN" => math::scalar(&v[0], libm::tan),
                "EXP" => math::scalar(&v[0], libm::exp),
                "LN" => math::scalar(&v[0], libm::log),
                "LOG" => math::scalar(&v[0], libm::log10),
                "SQRT" => math::scalar(&v[0], libm::sqrt),
                "TRUNC" => math::scalar(&v[0], libm::trunc),
                "ROUND" => math::round(&v),
                "STEP" => math::step(&v[0], &v[1]),
                "RND" => math::rnd(&v, ctx),
                "DAY" => date::day(&v[0]),
                "MONTH" => date::month(&v[0]),
                "YEAR" => date::year(&v[0]),
                "HOUR" => date::hour(&v[0]),
                "MINUTE" => date::minute(&v[0]),
                "SECOND" => date::second(&v[0]),
                "DAYS" => date::days(&v[0], &v[1]),
                "MONTHS" => date::months(&v[0], &v[1]),
                "YEARS" => date::years(&v[0], &v[1]),
                "HOURS" => date::hours(&v[0], &v[1]),
                "MINUTES" => date::minutes(&v[0], &v[1]),
                "SECONDS" => date::seconds(&v[0], &v[1]),
                "NUMTODATE" => date::num_to_date(&v[0], &v[1], &v[2]),
                "NUMTOTIME" => date::num_to_time(&v[0], &v[1], &v[2]),
                "TXTTODATE" => date::txt_to_date(&v[0], ctx),
                "TXTTONUM" => math::txt_to_num(&v[0]),
                "EPIWEEK" => date::epiweek(&v),
                "UPPERCASE" => text::uppercase(&v[0]),
                "SUBSTRING" => text::substring(&v),
                "STRLEN" => text::strlen(&v[0]),
                "FINDTEXT" => text::findtext(&v[0], &v[1]),
                "FORMAT" => text::format(&v),
                "SYSTEMDATE" => EV::Date(ctx.env.as_of.date),
                "SYSTEMTIME" => EV::Time(ctx.env.as_of.seconds),
                "RECORDCOUNT" => EV::Int(ctx.ds.row_count() as i64),
                "ENVIRON" => environ(&v[0], ctx),
                "EXISTS" => exists(&v[0], ctx),
                "FILEDATE" => filedate(&v[0], ctx),
                "POISSONLCL" => math::poisson_lcl(&v),
                "POISSONUCL" => math::poisson_ucl(&v),
                _ => EV::Missing,
            })
        }
    }
}

/// `ENVIRON(name)` — resolved from the `environ:<name>` globals the
/// recipe captured at session start; never a live read (register
/// Task 10 method choice). Missing when nothing was captured.
fn environ(v: &EV, ctx: &Ctx) -> EV {
    if let EV::Missing = v {
        return EV::Missing;
    }
    let key = ei_to_string(v);
    match ctx.global("environ", &key) {
        Some(value) => crate::expr::missing::value_ev(value),
        None => EV::Missing,
    }
}

/// `EXISTS(path)` — the captured `exists:<path>` boolean.
fn exists(v: &EV, ctx: &Ctx) -> EV {
    if let EV::Missing = v {
        return EV::Missing;
    }
    let key = ei_to_string(v);
    match ctx.global("exists", &key) {
        Some(epiora_core::dataset::Value::Boolean(b)) => EV::Bool(*b),
        Some(_) => EV::Missing,
        None => EV::Missing,
    }
}

/// `FILEDATE(path)` — the captured `filedate:<path>` DateTime.
fn filedate(v: &EV, ctx: &Ctx) -> EV {
    if let EV::Missing = v {
        return EV::Missing;
    }
    let key = ei_to_string(v);
    match ctx.global("filedate", &key) {
        Some(epiora_core::dataset::Value::DateTime(s)) => EV::DateTime(*s),
        Some(epiora_core::dataset::Value::Date(d)) => EV::DateTime(i64::from(*d) * 86_400),
        _ => EV::Missing,
    }
}
