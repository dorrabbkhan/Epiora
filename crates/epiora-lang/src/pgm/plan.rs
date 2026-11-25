//! The planner (spec §6.1): a parsed program becomes the action
//! list a run would execute — `Manage`/`Procedure`/`Read`/`Write`
//! `List` for the supported commands, `Note` for output-only
//! commands and `QUIT`/`EXIT`, and `Unsupported{line, command,
//! target_version}` for everything the register knows but Epiora
//! does not run.
//!
//! `RUNPGM` inlines the saved program's own actions (keeping its
//! line numbers) through a [`ProgramSource`]; a name already on the
//! call stack is a `pgm_runpgm_cycle` error, and a missing program
//! is an `Unsupported` action.
//!
//! `may_differ` marks every `Procedure` action that follows an
//! `affects_data` `Unsupported` action — the unrun command may have
//! changed the data the procedure reports on. Once set it never
//! resets: the plan names no reset, so a later `READ`/`WRITE`
//! leaves it on.

use super::ast::{DisplayWhat, Program, ProgramRef, Statement, WriteTarget};
use super::register::Register;
use crate::manage::Command;
use crate::view::DatasetRef;
use crate::LangError;

/// Where a `RUNPGM` looks up program text. `name` is the spelled
/// reference: a saved program's name, a file's path, or
/// `file:name` for a program inside a project.
pub trait ProgramSource {
    /// The program text for `name`, or `None` when it is unknown.
    fn program(&self, name: &str) -> Option<String>;
}

/// One planned action — a statement's execution-time effect.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// A cleaning command.
    Manage {
        /// Source line.
        line: u32,
        /// The command.
        command: Command,
    },
    /// A procedure call. `may_differ` is set when an unrun
    /// `affects_data` command may have changed the reported data.
    Procedure {
        /// Source line.
        line: u32,
        /// `FREQ` / `TABLES` / `MEANS`.
        name: String,
        /// The parsed options JSON.
        options: serde_json::Value,
        /// `true` when an earlier unsupported data command may have
        /// changed the view this procedure reports on.
        may_differ: bool,
    },
    /// `READ` — a new working view.
    Read {
        /// Source line.
        line: u32,
        /// The dataset target.
        target: DatasetRef,
    },
    /// `WRITE` — materialize the view to a target.
    Write {
        /// Source line.
        line: u32,
        /// The write target.
        target: WriteTarget,
    },
    /// `LIST` — display variables.
    List {
        /// Source line.
        line: u32,
        /// The variables as spelled.
        vars: Vec<String>,
    },
    /// An output-only command or `QUIT`/`EXIT` — recorded text.
    Note {
        /// Source line.
        line: u32,
        /// The command's text (or the `RUNPGM` reference).
        text: String,
    },
    /// A command the register knows but Epiora does not run — or a
    /// `RUNPGM` the source cannot resolve.
    Unsupported {
        /// Source line.
        line: u32,
        /// The register name (or `RUNPGM <name>` for an
        /// unresolvable program).
        command: String,
        /// The register's target version (`None` = `not_planned`
        /// or a missing program).
        target_version: Option<String>,
    },
}

/// Plan `program` against the register with no `ProgramSource`
/// (every `RUNPGM` resolves to an `Unsupported` action).
pub fn plan(program: &Program, reg: &Register) -> Vec<Action> {
    struct None_;
    impl ProgramSource for None_ {
        fn program(&self, _name: &str) -> Option<String> {
            None
        }
    }
    // With no source no `RUNPGM` inlines, so `RunPgmCycle` cannot
    // occur — the only error path is unreachable.
    plan_with(program, reg, &None_).unwrap_or_default()
}

/// Plan `program`, resolving `RUNPGM` targets through `src`.
pub fn plan_with(
    program: &Program,
    reg: &Register,
    src: &dyn ProgramSource,
) -> Result<Vec<Action>, LangError> {
    let mut out = Vec::new();
    let mut stack: Vec<String> = Vec::new();
    let mut differ = false;
    plan_into(program, reg, src, &mut stack, &mut differ, &mut out)?;
    Ok(out)
}

/// The `ProgramSource` lookup key for a `RUNPGM` reference.
fn ref_key(r: &ProgramRef) -> String {
    match r {
        ProgramRef::Named(n) => n.clone(),
        ProgramRef::File(f) => f.clone(),
        ProgramRef::FileProgram { file, name } => format!("{file}:{name}"),
    }
}

#[allow(clippy::too_many_arguments)]
fn plan_into(
    program: &Program,
    reg: &Register,
    src: &dyn ProgramSource,
    stack: &mut Vec<String>,
    differ: &mut bool,
    out: &mut Vec<Action>,
) -> Result<(), LangError> {
    for st in &program.statements {
        let line = st.line;
        match &st.node {
            Statement::Manage(c) => out.push(Action::Manage {
                line,
                command: c.clone(),
            }),
            Statement::Procedure { name, options } => out.push(Action::Procedure {
                line,
                name: name.clone(),
                options: options.clone(),
                may_differ: *differ,
            }),
            Statement::Read(d) => out.push(Action::Read {
                line,
                target: d.clone(),
            }),
            Statement::Write(t) => out.push(Action::Write {
                line,
                target: t.clone(),
            }),
            Statement::List(vars) => out.push(Action::List {
                line,
                vars: vars.clone(),
            }),
            Statement::Display(w) => out.push(Action::Note {
                line,
                text: format!("DISPLAY {}", display_word(w)),
            }),
            Statement::Output(t) => out.push(Action::Note {
                line,
                text: t.clone(),
            }),
            Statement::Quit => {
                out.push(Action::Note {
                    line,
                    text: "QUIT".to_string(),
                });
                // QUIT/EXIT ends the plan at its line.
                return Ok(());
            }
            Statement::RunPgm(r) => {
                let key = ref_key(r);
                match src.program(&key) {
                    Some(text) => {
                        if stack.iter().any(|s| s == &key) {
                            return Err(LangError::RunPgmCycle { name: key });
                        }
                        out.push(Action::Note {
                            line,
                            text: format!("RUNPGM {key}"),
                        });
                        stack.push(key);
                        let inner = super::parse_program(&text)?;
                        let run = plan_into(&inner, reg, src, stack, differ, out);
                        stack.pop();
                        run?;
                    }
                    // An unresolvable program is an Unsupported
                    // action — no note.
                    None => out.push(Action::Unsupported {
                        line,
                        command: format!("RUNPGM {key}"),
                        target_version: None,
                    }),
                }
            }
            Statement::Unsupported { name, .. } => {
                let entry = reg.command(name);
                out.push(Action::Unsupported {
                    line,
                    command: name.clone(),
                    target_version: entry.and_then(|e| e.version.clone()),
                });
                if entry.is_some_and(|e| e.affects_data) {
                    *differ = true;
                }
            }
        }
    }
    Ok(())
}

fn display_word(w: &DisplayWhat) -> &'static str {
    match w {
        DisplayWhat::DbVariables { .. } => "DBVARIABLES",
        DisplayWhat::DbViews => "DBVIEWS",
        DisplayWhat::Tables => "TABLES",
        DisplayWhat::Recipe => "RECIPE",
    }
}
