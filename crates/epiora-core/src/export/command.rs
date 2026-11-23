//! `epi_info_command` — the Epi Info Classic Analysis command a
//! result card shows (v1.0 Task 11, spec §7.3/G4). The command is
//! display-only in v1.0: `FREQ A B [STRATAVAR=S]`,
//! `TABLES E O [STRATAVAR=S1 S2]`, `MEANS V [G]`, in Epi Info's
//! spelling and upper case. A variable name that is not a plain
//! identifier (spaces or other characters) is wrapped in `[ ]` as
//! Epi Info does. Options with no Epi Info equivalent
//! (`include_missing`, a non-default `ci_method`, explicit `levels`)
//! are appended as `* Epiora:` comment lines so the line the user
//! sees never hides what the engine actually did.

use crate::dataset::Value as CellValue;
use crate::procedures::{FreqOptions, MeansOptions, ProportionCi, TablesOptions, VarSpec};
use crate::EngineError;
use serde::de::DeserializeOwned;
use serde_json::Value;

fn invalid(reason: impl Into<String>) -> EngineError {
    EngineError::InvalidOptions {
        reason: reason.into(),
    }
}

fn parse<T: DeserializeOwned>(procedure: &str, options: &Value) -> Result<T, EngineError> {
    serde_json::from_value(options.clone()).map_err(|e| {
        invalid(format!(
            "{procedure} options do not parse as its options: {e}"
        ))
    })
}

/// Epi Info's identifier rule: a letter or underscore followed by
/// letters, digits or underscores.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// A variable name as Epi Info prints it: upper case when it is a
/// plain identifier, `[name]` otherwise.
fn cmd_name(name: &str) -> String {
    if is_identifier(name) {
        name.to_uppercase()
    } else {
        format!("[{name}]")
    }
}

/// One cell value in a `levels` comment (`e`, `1.5`, `true`).
fn cell_text(v: &CellValue) -> String {
    match v {
        CellValue::Integer(n) => n.to_string(),
        CellValue::Real(x) => x.to_string(),
        CellValue::Text(s) => s.clone(),
        CellValue::Boolean(b) => b.to_string(),
        CellValue::Date(d) => d.to_string(),
        // Seconds in their unit — TABLES never takes Time/DateTime
        // levels, so these arms are unreachable in practice.
        CellValue::Time(t) => t.to_string(),
        CellValue::DateTime(t) => t.to_string(),
    }
}

fn levels_comment(field: &str, spec: &VarSpec) -> Option<String> {
    spec.levels.as_ref().map(|levels| {
        format!(
            "* Epiora: {field}.levels={},{}",
            cell_text(&levels[0]),
            cell_text(&levels[1])
        )
    })
}

fn stratified(line: &mut Vec<String>, strata: &[String]) {
    if !strata.is_empty() {
        let names: Vec<String> = strata.iter().map(|s| cmd_name(s)).collect();
        line.push(format!("STRATAVAR={}", names.join(" ")));
    }
}

/// The Epi Info command for `procedure` (`"freq"`, `"tables"` or
/// `"means"`) under its canonical `options` — a single line plus any
/// `* Epiora:` comment lines, joined by `\n`. Anything else is
/// `InvalidOptions`.
pub fn epi_info_command(procedure: &str, options: &Value) -> Result<String, EngineError> {
    match procedure {
        "freq" => {
            let o: FreqOptions = parse(procedure, options)?;
            let mut words = vec!["FREQ".to_string()];
            words.extend(o.variables.iter().map(|v| cmd_name(v)));
            if let Some(s) = &o.stratify_by {
                words.push(format!("STRATAVAR={}", cmd_name(s)));
            }
            let mut lines = vec![words.join(" ")];
            if o.include_missing {
                lines.push("* Epiora: include_missing".to_string());
            }
            if o.ci_method != ProportionCi::ClopperPearson {
                let name = match o.ci_method {
                    ProportionCi::ClopperPearson => "clopper_pearson",
                    ProportionCi::Wilson => "wilson",
                };
                lines.push(format!("* Epiora: ci_method={name}"));
            }
            Ok(lines.join("\n"))
        }
        "tables" => {
            let o: TablesOptions = parse(procedure, options)?;
            let mut words = vec![
                "TABLES".to_string(),
                cmd_name(&o.exposure.name),
                cmd_name(&o.outcome.name),
            ];
            stratified(&mut words, &o.stratify_by);
            let mut lines = vec![words.join(" ")];
            for comment in [
                levels_comment("exposure", &o.exposure),
                levels_comment("outcome", &o.outcome),
            ]
            .into_iter()
            .flatten()
            {
                lines.push(comment);
            }
            if o.include_missing {
                lines.push("* Epiora: include_missing".to_string());
            }
            Ok(lines.join("\n"))
        }
        "means" => {
            let o: MeansOptions = parse(procedure, options)?;
            let mut words = vec!["MEANS".to_string(), cmd_name(&o.variable)];
            if let Some(g) = &o.group_by {
                words.push(cmd_name(g));
            }
            let mut lines = vec![words.join(" ")];
            if o.include_missing {
                lines.push("* Epiora: include_missing".to_string());
            }
            Ok(lines.join("\n"))
        }
        other => Err(invalid(format!(
            "no Epi Info command for procedure {other:?}"
        ))),
    }
}
