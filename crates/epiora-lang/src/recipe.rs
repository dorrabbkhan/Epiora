//! The recipe (v1.1 spec §5.4): the canonical command sequence of a
//! session — the as-of instant plus every cleaning command since the
//! last READ. `canonical_text` is the stable, re-parseable spelling a
//! receipt's `working_view` stamp hashes; `working_view_stamp` is the
//! stamp itself (`None` while the recipe has no steps — a run on a
//! view that never saw a cleaning command is a plain v1/v2 receipt).

use crate::expr::eval::AsOf;
use crate::functions::calendar::civil_from_days;
use crate::manage::{AggFunc, Command, MergeMode, RecodeBound, RecodeRule, Scope, SetOption};
use crate::view::{JoinMode, Order, Process, StatisticsLevel, WorkingView};
use epiora_core::dataset::{Dataset, Value, ValueType};
use epiora_core::receipt::{DatasetStamp, WorkingViewStamp};
use sha2::Digest;

/// A session's recorded commands with its as-of instant.
#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    /// The session's as-of instant (`SYSTEMDATE`/`SYSTEMTIME`).
    pub as_of: AsOf,
    /// Commands since the last READ.
    pub steps: Vec<Command>,
}

impl Recipe {
    /// The canonical, re-parseable command text: one `AS-OF` header
    /// then one block per step, `Expr` printed in its fully
    /// parenthesized form so `parse(print(e)) = e`.
    pub fn canonical_text(&self) -> String {
        let mut out = String::new();
        let (y, m, d) = civil_from_days(self.as_of.date);
        out.push_str(&format!(
            "AS-OF {y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}\n",
            self.as_of.seconds / 3600,
            self.as_of.seconds % 3600 / 60,
            self.as_of.seconds % 60
        ));
        for (i, step) in self.steps.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            write_command(&mut out, step);
        }
        out.trim_end_matches('\n').to_string()
    }

    /// SHA-256 hex of the canonical text — `recipe_sha256` in the
    /// receipt's working-view stamp.
    pub fn sha256_hex(&self) -> String {
        let digest = sha2::Sha256::digest(self.canonical_text().as_bytes());
        let mut hex = String::with_capacity(digest.len() * 2);
        for byte in digest {
            hex.push(char::from_digit((byte >> 4) as u32, 16).unwrap_or('0'));
            hex.push(char::from_digit((byte & 0xf) as u32, 16).unwrap_or('0'));
        }
        hex
    }
}

/// The `working_view` stamp a session worker adds to `RunContext`
/// once a recipe has steps (spec §6.1): the source dataset's stamp,
/// the recipe text and hash, and the materialized view's content
/// hash. `None` for a step-less recipe — receipt v4 exists only when
/// a working view exists.
pub fn working_view_stamp(
    source: &Dataset,
    view: &Dataset,
    recipe: &Recipe,
) -> Option<WorkingViewStamp> {
    if recipe.steps.is_empty() {
        return None;
    }
    Some(WorkingViewStamp {
        source: DatasetStamp {
            id: source.id().to_string(),
            version: source.version(),
            sha256: source.content_hash_hex(),
        },
        recipe_text: recipe.canonical_text(),
        recipe_sha256: recipe.sha256_hex(),
        view_sha256: view.content_hash_hex(),
    })
}

fn write_command(out: &mut String, c: &Command) {
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
            format!("{m}/{dd}/{y}")
        }
        Value::Time(t) => format!("{}:{:02}:{:02}", t / 3600, t % 3600 / 60, t % 60),
        Value::DateTime(s) => {
            let (y, m, d, h, mi, sec) = crate::functions::calendar::civil_from_secs(*s);
            format!("{m}/{d}/{y} {h}:{mi:02}:{sec:02}")
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

/// Replays the recipe's steps over `source` — deterministic: the same
/// recipe always yields the same view hash or the same typed error.
pub fn replay(
    source: &Dataset,
    linked: &dyn crate::view::LinkedDatasets,
    recipe: &Recipe,
) -> Result<WorkingView, crate::LangError> {
    let mut view = WorkingView::new(source.clone());
    let mut env = crate::expr::eval::EvalEnv {
        as_of: recipe.as_of,
        globals: std::collections::BTreeMap::new(),
    };
    for step in &recipe.steps {
        crate::manage::apply(&mut view, linked, &mut env, step)?;
    }
    Ok(view)
}
