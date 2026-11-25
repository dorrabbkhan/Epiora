//! Statement parsers for the non-`Command` statements: READ, WRITE,
//! LIST, DISPLAY, the procedures (FREQ/TABLES/MEANS) and RUNPGM.

use super::{Parser, StmtOut, Tok};
use crate::pgm::ast::{DisplayWhat, ProgramRef, Statement, WriteMode, WriteTarget, WriteVars};
use crate::pgm::lexer::Lexed;
use crate::view::DatasetRef;
use crate::LangError;

/// `READ [String] [['file'|'{brace}'] ':'] <qid>` — the dataset or
/// `file:table` forms. `FileSpec`, `LINKNAME` and SQL/ODBC sources
/// are register-documented gaps → Unsupported.
pub(crate) fn read(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    let mut j = 1;
    // Optional leading format string (`"Epi7"`, `"HTML"`, `"SQL"`…).
    let had_str = matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Str(_)));
    if had_str {
        j += 1;
    }
    // Optional `file:` or `{conn}:` prefix.
    let mut file: Option<String> = None;
    match toks.get(j).map(|t| &t.tok) {
        Some(Tok::File(f)) | Some(Tok::Brace(f)) => {
            if matches!(toks.get(j + 1).map(|t| &t.tok), Some(Tok::Colon)) {
                file = Some(f.clone());
                j += 2;
            } else {
                return Ok(p.unsup(i, i + 1));
            }
        }
        _ => {}
    }
    let Some((table, end)) = p.qid(toks, j) else {
        return Ok(p.unsup(i, i + 1));
    };
    j = end;
    if j != n {
        // Trailing options (LINKNAME, FILESPEC…) are outside the
        // supported READ form.
        return Ok(p.unsup(i, i + 1));
    }
    // `READ "fmt" 'file':t` — the file forms need FileSpec, which is
    // not implemented; a bare `READ "fmt" t` is the Simple_Read form.
    if had_str && file.is_some() {
        return Ok(p.unsup(i, i + 1));
    }
    let name = match file {
        Some(f) => format!("{f}:{table}"),
        None => table,
    };
    Ok(StmtOut {
        statements: vec![p.spanned(toks, Statement::Read(DatasetRef::named(&name)))],
        next: i + 1,
    })
}

/// `WRITE <REPLACE|APPEND> "<fmt>" <target> <selection>` —
/// `target` is `qid`, `file:qid` or `file`; `selection` is `*`,
/// `* EXCEPT vars` or a variable list.
pub(crate) fn write(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    let mode = match p.word(toks, 1).as_deref() {
        Some("REPLACE") => WriteMode::Replace,
        Some("APPEND") => WriteMode::Append,
        _ => return Ok(p.unsup(i, i + 1)),
    };
    let Some(Tok::Str(format)) = toks.get(2).map(|t| &t.tok) else {
        return Ok(p.unsup(i, i + 1));
    };
    let format = format.clone();
    // The grammar's output formats.
    if !matches!(
        format.to_ascii_uppercase().as_str(),
        "EPI7" | "EPI2000" | "EPI 2000" | "EXCEL 8.0" | "TEXT"
    ) {
        return Ok(p.unsup(i, i + 1));
    }
    let mut j = 3;
    let target = match toks.get(j).map(|t| &t.tok) {
        Some(Tok::File(f)) | Some(Tok::Brace(f)) => {
            if matches!(toks.get(j + 1).map(|t| &t.tok), Some(Tok::Colon)) {
                j += 2;
                match p.qid(toks, j) {
                    Some((t, end)) => {
                        j = end;
                        format!("{f}:{t}")
                    }
                    None => return Ok(p.unsup(i, i + 1)),
                }
            } else {
                // A bare file target.
                j += 1;
                f.clone()
            }
        }
        _ => match p.qid(toks, j) {
            Some((t, end)) => {
                j = end;
                t
            }
            None => return Ok(p.unsup(i, i + 1)),
        },
    };
    // The selection: `* [EXCEPT vars]` or a variable list.
    let vars = match toks.get(j).map(|t| &t.tok) {
        Some(Tok::Star) => {
            j += 1;
            if p.kw(toks, j, "EXCEPT") {
                j += 1;
                let (v, end) = p.idlist(toks, j, &[]);
                if v.is_empty() {
                    return Ok(p.unsup(i, i + 1));
                }
                j = end;
                WriteVars::Except(v)
            } else {
                WriteVars::All
            }
        }
        _ => {
            let (v, end) = p.idlist(toks, j, &[]);
            if v.is_empty() {
                return Ok(p.unsup(i, i + 1));
            }
            j = end;
            WriteVars::Only(v)
        }
    };
    if j != n {
        return Ok(p.unsup(i, i + 1));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(
            toks,
            Statement::Write(WriteTarget {
                mode,
                format,
                target,
                vars,
            }),
        )],
        next: i + 1,
    })
}

/// `LIST [* [EXCEPT vars]|vars] [GRIDTABLE]` — the interactive
/// forms. `UPDATE` and the HTML layout options are register gaps →
/// Unsupported.
pub(crate) fn list(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    let mut j = 1;
    let mut vars: Vec<String> = Vec::new();
    let mut ok = true;
    if matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Star)) {
        vars.push("*".to_string());
        j += 1;
        if p.kw(toks, j, "EXCEPT") {
            vars.push("EXCEPT".to_string());
            j += 1;
            let (v, end) = p.idlist(toks, j, &["GRIDTABLE", "UPDATE"]);
            if v.is_empty() {
                ok = false;
            }
            vars.extend(v);
            j = end;
        }
    } else {
        let (v, end) = p.idlist(toks, j, &["GRIDTABLE", "UPDATE", "EXCEPT", "OUTTABLE"]);
        vars = v;
        j = end;
    }
    if !ok {
        return Ok(p.unsup(i, i + 1));
    }
    // Optional GRIDTABLE; anything else (UPDATE, HTML options) is
    // not the supported form.
    if p.kw(toks, j, "GRIDTABLE") {
        j += 1;
    }
    if j != n {
        return Ok(p.unsup(i, i + 1));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(toks, Statement::List(vars))],
        next: i + 1,
    })
}

/// `DISPLAY DBVARIABLES|DBVIEWS|TABLES|RECIPE […]` — other subjects
/// are register gaps → Unsupported.
pub(crate) fn display(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    let what = match p.word(toks, 1).as_deref() {
        Some("DBVIEWS") => Some(DisplayWhat::DbViews),
        Some("TABLES") => Some(DisplayWhat::Tables),
        Some("RECIPE") => Some(DisplayWhat::Recipe),
        Some("DBVARIABLES") => {
            // `DISPLAY DBVARIABLES [FIELDVAR|DEFINE|LIST] [vars]
            // [OUTTABLE=t]` — FIELDVAR/DEFINE/LIST are spelling
            // variants; the variable list narrows the display.
            let mut j = 2;
            while matches!(
                p.word(toks, j).as_deref(),
                Some("FIELDVAR" | "DEFINE" | "LIST")
            ) {
                j += 1;
            }
            let mut fields = Vec::new();
            let mut ok = true;
            while j < n {
                if matches!(toks.get(j + 1).map(|t| &t.tok), Some(Tok::Eq)) {
                    // `NAME = v` option (OUTTABLE=…) — skip its value.
                    let (v, end) = p.opt_value(toks, j + 2);
                    if v.is_none() {
                        ok = false;
                        break;
                    }
                    j = end;
                } else if let Some((q, end)) = p.qid(toks, j) {
                    fields.push(q);
                    j = end;
                } else {
                    ok = false;
                    break;
                }
            }
            if !ok {
                None
            } else {
                Some(DisplayWhat::DbVariables { fields })
            }
        }
        _ => None,
    };
    match what {
        Some(w) => Ok(StmtOut {
            statements: vec![p.spanned(toks, Statement::Display(w))],
            next: i + 1,
        }),
        None => Ok(p.unsup(i, i + 1)),
    }
}

/// `FREQ|TABLES|MEANS <vars> [options]` — variables positionally,
/// then `NAME = value` options and bare flags, collected into a JSON
/// options object. Option-level support gating is the runner's job
/// (Task 16+), not the parser's.
pub(crate) fn procedure(
    p: &Parser,
    i: usize,
    toks: &[Lexed],
    head: &str,
) -> Result<StmtOut, LangError> {
    let n = toks.len();
    let mut j = 1;
    let mut vars: Vec<String> = Vec::new();
    let mut all = false;
    let mut except: Vec<String> = Vec::new();
    // `FREQ *` is "every variable"; `*` alone takes `EXCEPT`.
    if head == "FREQ" && matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Star)) {
        all = true;
        j += 1;
        if p.kw(toks, j, "EXCEPT") {
            j += 1;
            let (v, end) = p.idlist(toks, j, &[]);
            except = v;
            j = end;
        }
    } else {
        // Positional variables (TABLES takes exposure + outcome,
        // MEANS var + crosstab, FREQ a list) — stopping before a
        // `NAME =` option.
        while let Some((q, end)) = p.qid(toks, j) {
            if matches!(toks.get(end).map(|t| &t.tok), Some(Tok::Eq)) {
                break;
            }
            vars.push(q);
            j = end;
        }
    }
    // Positional arity: TABLES needs two, MEANS one or two.
    let max_pos = match head {
        "TABLES" => 2,
        "MEANS" => 2,
        _ => usize::MAX,
    };
    let min_pos = match head {
        "TABLES" => 2,
        "MEANS" => 1,
        _ => 1,
    };
    if !all && (vars.len() < min_pos || vars.len() > max_pos) {
        return Ok(p.unsup(i, i + 1));
    }
    // Named options: `NAME = value(s)` or a bare flag word.
    let mut opts = serde_json::Map::new();
    while j < n {
        let Some(name) = p.word(toks, j) else {
            return Ok(p.unsup(i, i + 1));
        };
        if matches!(toks.get(j + 1).map(|t| &t.tok), Some(Tok::Eq)) {
            j += 2;
            // Value: string / number / identifier list (STRATAVAR).
            let (v, end) = p.opt_value(toks, j);
            match v {
                Some(v) => {
                    opts.insert(name, v);
                    j = end;
                }
                None => return Ok(p.unsup(i, i + 1)),
            }
        } else {
            opts.insert(name, serde_json::Value::Bool(true));
            j += 1;
        }
    }
    let mut obj = serde_json::Map::new();
    obj.insert("vars".into(), serde_json::json!(vars));
    obj.insert("opts".into(), serde_json::Value::Object(opts));
    if all {
        obj.insert("all".into(), serde_json::Value::Bool(true));
    }
    if !except.is_empty() {
        obj.insert("except".into(), serde_json::json!(except));
    }
    Ok(StmtOut {
        statements: vec![p.spanned(
            toks,
            Statement::Procedure {
                name: head.to_string(),
                options: serde_json::Value::Object(obj),
            },
        )],
        next: i + 1,
    })
}

/// `RUNPGM <name|"name">`, `RUNPGM <file|bare path>` or
/// `RUNPGM <file|string> ':' <name>`.
pub(crate) fn runpgm(p: &Parser, i: usize, toks: &[Lexed]) -> Result<StmtOut, LangError> {
    let n = toks.len();
    let stmt = |node| StmtOut {
        statements: vec![p.spanned(toks, node)],
        next: i + 1,
    };
    match toks.get(1).map(|t| &t.tok) {
        Some(Tok::File(f)) | Some(Tok::Brace(f)) => {
            if matches!(toks.get(2).map(|t| &t.tok), Some(Tok::Colon)) {
                if let Some((name, end)) = p.qid(toks, 3) {
                    if end == n {
                        return Ok(stmt(Statement::RunPgm(ProgramRef::FileProgram {
                            file: f.clone(),
                            name,
                        })));
                    }
                }
            } else if n == 2 {
                return Ok(stmt(Statement::RunPgm(ProgramRef::File(f.clone()))));
            }
            Ok(p.unsup(i, i + 1))
        }
        Some(Tok::Str(s)) => {
            if matches!(toks.get(2).map(|t| &t.tok), Some(Tok::Colon)) {
                if let Some((name, end)) = p.qid(toks, 3) {
                    if end == n {
                        return Ok(stmt(Statement::RunPgm(ProgramRef::FileProgram {
                            file: s.clone(),
                            name,
                        })));
                    }
                }
                return Ok(p.unsup(i, i + 1));
            }
            if n == 2 {
                return Ok(stmt(Statement::RunPgm(ProgramRef::Named(s.clone()))));
            }
            Ok(p.unsup(i, i + 1))
        }
        _ => {
            // Bare `RUNPGM name` or an unquoted path `a.b`.
            if let Some((name, end)) = p.qid(toks, 1) {
                if end == n {
                    return Ok(stmt(Statement::RunPgm(ProgramRef::Named(name))));
                }
            }
            Ok(p.unsup(i, i + 1))
        }
    }
}
