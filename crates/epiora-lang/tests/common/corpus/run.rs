//! `run_line` — one translated corpus step against the session.

use super::compare::{
    compare_delete, compare_freq, compare_list, compare_means, compare_merge, compare_selection,
    compare_undelete,
};
use super::fixture::Step;
use super::parse::{expr, lit_value, recode_rule, set_option};
use super::session::Session;
use epiora_lang::{Command, DatasetRef, JoinMode, MergeMode, Order, Scope};
use std::path::Path;

/// Runs one translated line; returns the number of steps consumed
/// (block commands consume their body lines too).
pub fn run_line(sess: &mut Session, steps: &[Step], i: usize) -> usize {
    let step = &steps[i];
    let text = step.command.trim();
    let up = text.to_ascii_uppercase();

    if up.starts_with("ROUTEOUT")
        || up.starts_with("CLOSEOUT")
        || up.starts_with("QUIT")
        || up.starts_with("DISPLAY")
    {
        return 1;
    }
    if let Some(var) = text.strip_prefix("FREQ ") {
        compare_freq(sess, var.trim(), step);
        return 1;
    }
    if let Some(var) = text.strip_prefix("MEANS ") {
        compare_means(sess, var.trim(), step);
        return 1;
    }
    if let Some(vars) = text.strip_prefix("LIST ") {
        let vars: Vec<String> = vars.split_whitespace().map(|s| s.to_string()).collect();
        compare_list(sess, &vars, step);
        return 1;
    }
    if let Some(e) = text.strip_prefix("SELECT ") {
        sess.command(Command::Select(expr(e)));
        compare_selection(sess, step);
        return 1;
    }
    if up == "CANCEL SELECT" {
        sess.command(Command::CancelSelect);
        compare_selection(sess, step);
        return 1;
    }
    if up.starts_with("SORT") {
        let toks: Vec<_> = text[4..].split_whitespace().collect();
        let mut keys = vec![];
        let mut t = 0;
        while t < toks.len() {
            let name = toks[t].to_string();
            t += 1;
            let dir = match toks.get(t).map(|d| d.to_ascii_uppercase()) {
                Some(d) if d == "DESC" || d == "DESCENDING" => {
                    t += 1;
                    Order::Desc
                }
                Some(d) if d == "ASC" || d == "ASCENDING" => {
                    t += 1;
                    Order::Asc
                }
                _ => Order::Asc,
            };
            keys.push((name, dir));
        }
        sess.command(Command::Sort(keys));
        compare_selection(sess, step);
        return 1;
    }
    if up == "CANCEL SORT" {
        sess.command(Command::CancelSort);
        compare_selection(sess, step);
        return 1;
    }
    if let Some(rest) = text.strip_prefix("DEFINE ") {
        if let Some((name, e)) = rest.split_once('=') {
            sess.command(Command::DefineExpr {
                name: name.trim().to_string(),
                expr: expr(e.trim()),
                scope: Scope::Standard,
            });
        } else {
            let mut it = rest.split_whitespace();
            let name = it.next().expect("define name").to_string();
            let ty = match it
                .next()
                .expect("define type")
                .to_ascii_uppercase()
                .as_str()
            {
                "NUMERIC" => epiora_core::dataset::ValueType::Real,
                "TEXTINPUT" => epiora_core::dataset::ValueType::Text,
                "YN" | "YESNO" => epiora_core::dataset::ValueType::Boolean,
                "DATEFORMAT" => epiora_core::dataset::ValueType::Date,
                "TIMEFORMAT" => epiora_core::dataset::ValueType::Time,
                "DATETIMEFORMAT" => epiora_core::dataset::ValueType::DateTime,
                other => panic!("define type {other}"),
            };
            sess.command(Command::Define {
                name,
                ty,
                scope: Scope::Standard,
            });
        }
        return 1;
    }
    if let Some(rest) = text.strip_prefix("ASSIGN ") {
        let (name, e) = rest.split_once('=').expect("assign has =");
        sess.command(Command::Assign {
            name: name.trim().to_string(),
            expr: expr(e),
        });
        return 1;
    }
    if let Some(rest) = text.strip_prefix("UNDEFINE ") {
        sess.command(Command::Undefine(rest.trim().to_string()));
        return 1;
    }
    if up.starts_with("IF ") {
        let cond = text[3..]
            .trim_end()
            .strip_suffix("THEN")
            .expect("IF THEN")
            .trim();
        let mut then = vec![];
        let mut otherwise = vec![];
        let mut j = i + 1;
        let mut into_else = false;
        while j < steps.len() {
            let l = steps[j].command.trim();
            if l.eq_ignore_ascii_case("END") {
                break;
            }
            if l.eq_ignore_ascii_case("ELSE") {
                into_else = true;
            } else if let Some(a) = l.strip_prefix("ASSIGN ") {
                let (n, e) = a.split_once('=').expect("assign");
                let c = Command::Assign {
                    name: n.trim().to_string(),
                    expr: expr(e),
                };
                if into_else {
                    otherwise.push(c);
                } else {
                    then.push(c);
                }
            } else {
                panic!("IF body line {l:?} at {}", steps[j].line);
            }
            j += 1;
        }
        sess.command(Command::If {
            cond: expr(cond),
            then,
            otherwise,
        });
        return j - i + 1; // consume END too
    }
    if up.starts_with("RECODE ") {
        let mut parts = text[7..].split_whitespace();
        let from = parts.next().expect("from").to_string();
        assert_eq!(
            parts.next().map(|s| s.to_ascii_uppercase()),
            Some("TO".into())
        );
        let to = parts.next().expect("to").to_string();
        let mut rules = vec![];
        let mut otherwise = None;
        let mut j = i + 1;
        while j < steps.len() && !steps[j].command.trim().eq_ignore_ascii_case("END") {
            let l = steps[j].command.trim();
            match l.split_once('=') {
                Some((k, v)) if k.trim().eq_ignore_ascii_case("ELSE") => {
                    otherwise = Some(lit_value(v))
                }
                _ => rules.push(recode_rule(l)),
            }
            j += 1;
        }
        sess.command(Command::Recode {
            from,
            to,
            rules,
            otherwise,
        });
        return j - i + 1;
    }
    if up.starts_with("DELETE ") {
        let e = text[7..]
            .trim()
            .strip_suffix("RUNSILENT")
            .map(|s| s.trim())
            .unwrap_or(text[7..].trim());
        let before = sess.count();
        sess.command(Command::DeleteRecords(expr(e)));
        compare_delete(sess, step, before);
        return 1;
    }
    if up.starts_with("UNDELETE") {
        let inner = text[8..]
            .trim()
            .strip_suffix("RUNSILENT")
            .map(|s| s.trim())
            .unwrap_or(text[8..].trim());
        let before = sess.count();
        let arg = if inner == "*" || inner.is_empty() {
            None
        } else {
            Some(expr(inner))
        };
        sess.command(Command::Undelete(arg));
        compare_undelete(sess, step, before);
        return 1;
    }
    if up.starts_with("SET ") {
        let inner = text[4..]
            .trim()
            .strip_suffix("END-SET")
            .map(|s| s.trim())
            .unwrap_or(text[4..].trim());
        sess.command(Command::Set(set_option(inner)));
        return 1;
    }
    if up.starts_with("MERGE ") {
        let rest = text[6..].trim();
        let (pathq, after) = rest.split_once("':").expect("'file':table");
        let file = Path::new(pathq.trim_matches('\''))
            .file_stem()
            .expect("stem")
            .to_string_lossy()
            .to_string();
        let mut toks = after.split_whitespace();
        let table = toks.next().expect("table").to_string();
        let keys: Vec<(String, String)> = toks
            .filter(|t| t.contains("::"))
            .map(|t| {
                let (a, b) = t.split_once("::").expect("::");
                (a.to_string(), b.to_string())
            })
            .collect();
        sess.table(&file, &table);
        let before_src = sess.source.clone();
        sess.command(Command::Merge {
            dataset: DatasetRef::named(&table),
            keys: keys.clone(),
            mode: MergeMode::MatchOrInsert,
        });
        // The merge wrote the destination table (EI7 writes the file).
        let dest = sess.source.id().to_ascii_lowercase();
        sess.tables.insert(dest, sess.view.dataset.clone());
        compare_merge(sess, &before_src, &table, &keys, step);
        return 1;
    }
    if up.starts_with("RELATE ") {
        let toks: Vec<_> = text[7..].split_whitespace().collect();
        let name = toks[0].to_string();
        let mut keys = vec![];
        let mut join = JoinMode::Matching;
        for t in &toks[1..] {
            if let Some((a, b)) = t.split_once("::") {
                keys.push((a.to_string(), b.to_string()));
            } else if t.eq_ignore_ascii_case("ALL") {
                join = JoinMode::All;
            }
        }
        // The child table comes from the same file as the source.
        sess.table(&sess_file(sess), &name);
        sess.command(Command::Relate {
            dataset: DatasetRef { name, join },
            keys,
        });
        compare_selection(sess, step);
        return 1;
    }
    panic!("unhandled step {}: {text}", step.line);
}

/// The mdb file stem the current source came from.
fn sess_file(sess: &Session) -> String {
    for (stem, mdb) in sess.mdbs {
        if mdb
            .tables()
            .map(|ts| ts.iter().any(|t| t.eq_ignore_ascii_case(sess.source.id())))
            .unwrap_or(false)
        {
            return stem.clone();
        }
    }
    panic!("source file for {}", sess.source.id())
}
