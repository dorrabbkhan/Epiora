//! The canonical command spelling (`print_command`): the re-parseable
//! text recipe `canonical_text` is built from and `.pgm` round-trips
//! against (`parse_program(print_command(c))` yields `c`). `Expr`
//! prints in its fully-parenthesized form so expressions round-trip
//! exactly.

use crate::functions::calendar::civil_from_days;
use crate::manage::{AggFunc, Command, MergeMode, RecodeBound, RecodeRule, Scope, SetOption};
use crate::view::{JoinMode, Order, Process, StatisticsLevel};
use epiora_core::dataset::{Value, ValueType};

/// The canonical, re-parseable spelling of one command.
pub fn print_command(c: &Command) -> String {
    let mut out = String::new();
    write_command(&mut out, c);
    out
}

/// Append one command's canonical text (block commands span lines).
pub(crate) fn write_command(out: &mut String, c: &Command) {
    match c {
        Command::Select(e) => out.push_str(&format!("SELECT {e}")),
        Command::CancelSelect => out.push_str("CANCEL SELECT"),
        Command::Sort(keys) => {
            out.push_str("SORT");
            for (k, ord) in keys {
                out.push(' ');
                out.push_str(k);
                if *ord == Order::Desc {
                    out.push_str(" DESCENDING");
                }
            }
        }
        Command::CancelSort => out.push_str("CANCEL SORT"),
        Command::CancelRead => out.push_str("CANCEL READ"),
        Command::Define { name, ty, scope } => {
            out.push_str(&format!("DEFINE {name} {}", type_word(ty)));
            if *scope != Scope::Standard {
                out.push_str(&format!(" {}", scope_word(*scope)));
            }
        }
        Command::DefineExpr { name, expr, scope } => {
            out.push_str(&format!("DEFINE {name} = {expr}"));
            if *scope != Scope::Standard {
                out.push_str(&format!(" {}", scope_word(*scope)));
            }
        }
        Command::Undefine(n) => out.push_str(&format!("UNDEFINE {n}")),
        Command::Assign { name, expr } => out.push_str(&format!("ASSIGN {name} = {expr}")),
        Command::If {
            cond,
            then,
            otherwise,
        } => {
            out.push_str(&format!("IF {cond} THEN"));
            for c in then {
                out.push('\n');
                write_command(out, c);
            }
            if !otherwise.is_empty() {
                out.push_str("\nELSE");
                for c in otherwise {
                    out.push('\n');
                    write_command(out, c);
                }
            }
            out.push_str("\nEND");
        }
        Command::Recode {
            from,
            to,
            rules,
            otherwise,
        } => {
            out.push_str(&format!("RECODE {from} TO {to}"));
            for r in rules {
                out.push('\n');
                match r {
                    RecodeRule::Single { from, to } => {
                        out.push_str(&format!("{} = {}", value_text(from), value_text(to)))
                    }
                    RecodeRule::Range { lo, hi, to } => out.push_str(&format!(
                        "{} - {} = {}",
                        bound_text(lo),
                        bound_text(hi),
                        value_text(to)
                    )),
                }
            }
            if let Some(v) = otherwise {
                out.push_str(&format!("\nELSE = {}", value_text(v)));
            }
            out.push_str("\nEND");
        }
        Command::Merge {
            dataset,
            keys,
            mode,
        } => {
            out.push_str(&format!("MERGE {}", dataset.name));
            for (d, s) in keys {
                out.push_str(&format!(" {d}::{s}"));
            }
            match mode {
                MergeMode::MatchOrInsert => {}
                MergeMode::Update => out.push_str(" UPDATE"),
                MergeMode::Append => out.push_str(" APPEND"),
            }
        }
        Command::Relate { dataset, keys } => {
            out.push_str(&format!("RELATE {}", dataset.name));
            for (d, s) in keys {
                out.push_str(&format!(" {d}::{s}"));
            }
            if dataset.join == JoinMode::All {
                out.push_str(" ALL");
            }
        }
        Command::DeleteRecords(e) => out.push_str(&format!("DELETE {e}")),
        Command::Undelete(None) => out.push_str("UNDELETE *"),
        Command::Undelete(Some(e)) => out.push_str(&format!("UNDELETE {e}")),
        Command::DeleteVariables(ns) => {
            out.push_str("DELETE VARIABLES");
            for n in ns {
                out.push(' ');
                out.push_str(n);
            }
        }
        Command::KeyVars(ns) => {
            out.push_str("KEYVARS");
            for n in ns {
                out.push(' ');
                out.push_str(n);
            }
        }
        Command::Set(o) => out.push_str(&format!("SET {}", set_text(o))),
        Command::Summarize {
            aggregates,
            by,
            into,
        } => {
            out.push_str("SUMMARIZE");
            for a in aggregates {
                let arg = a.arg.as_deref().unwrap_or("");
                out.push_str(&format!(
                    " {}::{}{}",
                    a.name,
                    agg_word(a.func),
                    arg_arg(arg)
                ));
            }
            out.push_str(&format!(" TO {}", into.name));
            if !by.is_empty() {
                out.push_str(&format!(" STRATAVAR={}", by.join(" ")));
            }
        }
    }
}

fn arg_arg(a: &str) -> String {
    if a.is_empty() {
        "()".into()
    } else {
        format!("({a})")
    }
}

fn type_word(ty: &ValueType) -> &'static str {
    match ty {
        ValueType::Integer | ValueType::Real => "NUMERIC",
        ValueType::Text => "TEXTINPUT",
        ValueType::Boolean => "YN",
        ValueType::Date => "DATEFORMAT",
        ValueType::Time => "TIMEFORMAT",
        ValueType::DateTime => "DATETIMEFORMAT",
    }
}

fn scope_word(s: Scope) -> &'static str {
    match s {
        Scope::Standard => "STANDARD",
        Scope::Global => "GLOBAL",
        Scope::Permanent => "PERMANENT",
    }
}

fn bound_text(b: &RecodeBound) -> String {
    match b {
        RecodeBound::LoValue => "LOVALUE".into(),
        RecodeBound::HiValue => "HIVALUE".into(),
        RecodeBound::Value(v) => value_text(v),
    }
}

fn value_text(v: &Value) -> String {
    match v {
        Value::Text(s) => format!("\"{s}\""),
        Value::Integer(i) => i.to_string(),
        Value::Real(x) => format!("{x}"),
        Value::Boolean(b) => {
            if *b {
                "(+)".into()
            } else {
                "(-)".into()
            }
        }
        Value::Date(d) => {
            let (y, m, dd) = civil_from_days(*d);
            format!("{m}/{dd}/{y:04}")
        }
        Value::Time(t) => format!("{}:{:02}:{:02}", t / 3600, t % 3600 / 60, t % 60),
        Value::DateTime(s) => {
            let (y, m, d, h, mi, sec) = crate::functions::calendar::civil_from_secs(*s);
            format!("{m}/{d}/{y:04} {h}:{mi:02}:{sec:02}")
        }
    }
}

fn set_text(o: &SetOption) -> String {
    match o {
        SetOption::Process(p) => format!(
            "PROCESS={}",
            match p {
                Process::Undeleted => "UNDELETED",
                Process::Both => "BOTH",
                Process::Deleted => "DELETED",
            }
        ),
        SetOption::Missing(on) => format!("MISSING={}", onoff(*on)),
        SetOption::Deleted(on) => format!("DELETED={}", onoff(*on)),
        SetOption::Statistics(s) => format!(
            "STATISTICS={}",
            match s {
                StatisticsLevel::None => "NONE",
                StatisticsLevel::Minimal => "MINIMAL",
                StatisticsLevel::Intermediate => "INTERMEDIATE",
                StatisticsLevel::Complete => "COMPLETE",
            }
        ),
        SetOption::Percents(on) => format!("PERCENTS={}", onoff(*on)),
        SetOption::Ignore(on) => format!("IGNORE={}", onoff(*on)),
        SetOption::Select(on) => format!("SELECT={}", onoff(*on)),
    }
}

fn onoff(on: bool) -> &'static str {
    if on {
        "ON"
    } else {
        "OFF"
    }
}

fn agg_word(f: AggFunc) -> &'static str {
    match f {
        AggFunc::Count => "Count",
        AggFunc::Sum => "Sum",
        AggFunc::Avg => "Avg",
        AggFunc::Min => "Min",
        AggFunc::Max => "Max",
        AggFunc::First => "First",
        AggFunc::Last => "Last",
        AggFunc::Stdev => "Stdev",
        AggFunc::Variance => "Variance",
    }
}
