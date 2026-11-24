//! Command translation helpers (the corpus's `.pgm` subset).

use epiora_lang::{parse_expr, Expr, Process, RecodeBound, RecodeRule, SetOption, StatisticsLevel};
use std::path::Path;

pub fn expr(t: &str) -> Expr {
    parse_expr(t).unwrap_or_else(|e| panic!("expr {t:?}: {e}"))
}

/// `'<path>':<table>` → (file stem, table).
pub fn read_target(t: &str) -> (String, String) {
    let (path, table) = t.rsplit_once(':').expect("file:table");
    let stem = Path::new(path.trim_matches('\''))
        .file_stem()
        .expect("stem")
        .to_string_lossy()
        .to_string();
    (stem, table.trim().to_string())
}

pub fn lit_value(t: &str) -> epiora_core::dataset::Value {
    use epiora_core::dataset::Value;
    let t = t.trim();
    if let Some(s) = t
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .or_else(|| t.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
    {
        return Value::Text(s.to_string());
    }
    match t {
        "(+)" => return Value::Boolean(true),
        "(-)" => return Value::Boolean(false),
        _ => {}
    }
    if let Ok(i) = t.parse::<i64>() {
        Value::Integer(i)
    } else if let Ok(x) = t.parse::<f64>() {
        Value::Real(x)
    } else {
        panic!("literal {t:?}")
    }
}

pub fn recode_bound(t: &str) -> RecodeBound {
    match t.trim().to_ascii_uppercase().as_str() {
        "LOVALUE" => RecodeBound::LoValue,
        "HIVALUE" => RecodeBound::HiValue,
        v => RecodeBound::Value(lit_value(v)),
    }
}

pub fn recode_rule(line: &str) -> RecodeRule {
    let (lhs, to) = line.rsplit_once('=').expect("rule has =");
    let to = lit_value(to);
    if let Some((lo, hi)) = lhs.split_once(" - ") {
        RecodeRule::Range {
            lo: recode_bound(lo),
            hi: recode_bound(hi),
            to,
        }
    } else {
        RecodeRule::Single {
            from: lit_value(lhs),
            to,
        }
    }
}

pub fn set_option(clause: &str) -> SetOption {
    let (k, v) = clause.split_once('=').expect("set clause");
    match (
        k.trim().to_ascii_uppercase().as_str(),
        v.trim().to_ascii_uppercase().as_str(),
    ) {
        ("PROCESS", "BOTH") => SetOption::Process(Process::Both),
        ("PROCESS", "UNDELETED") => SetOption::Process(Process::Undeleted),
        ("PROCESS", "DELETED") => SetOption::Process(Process::Deleted),
        ("MISSING", on) => SetOption::Missing(on == "ON"),
        ("DELETED", on) => SetOption::Deleted(on == "ON"),
        ("STATISTICS", s) => SetOption::Statistics(match s {
            "NONE" => StatisticsLevel::None,
            "MINIMAL" => StatisticsLevel::Minimal,
            "INTERMEDIATE" => StatisticsLevel::Intermediate,
            "COMPLETE" => StatisticsLevel::Complete,
            _ => panic!("statistics {s}"),
        }),
        ("PERCENTS", on) => SetOption::Percents(on == "ON"),
        ("IGNORE", on) => SetOption::Ignore(on == "ON"),
        ("SELECT", on) => SetOption::Select(on == "ON"),
        other => panic!("set clause {other:?}"),
    }
}
