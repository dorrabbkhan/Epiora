//! `every_reference_program_step_matches` (v1.1 plan Task 11, spec
//! §15.2): replays every `fixtures/ei7/v11/*.json` reference program
//! step by step against the raw `.mdb` tables (loaded natively through
//! `epiora-mdb` + `epiora-legacy::decode_batch`), comparing every
//! `results` entry at the `ei7_display` tolerance (`0.5 × 10^-d`, d =
//! the fixture's displayed decimals) and every LIST row exactly.
//!
//! DISPLAY TABLES / DBVARIABLES rows are EI7-internal dumps (Access
//! formats and PK index names the EPTB batches do not carry) and are
//! not compared — their `results` are empty.

use epiora_core::dataset::{Column, ColumnData, Dataset};
use epiora_core::procedures::{freq, means, FreqOptions, MeansOptions, RunContext};
use epiora_lang::{
    parse_expr, replay, AsOf, Command, DatasetRef, Expr, JoinMode, MergeMode, Order, Process,
    Recipe, RecodeBound, RecodeRule, Scope, SetOption, StatisticsLevel, WorkingView,
};
use epiora_legacy::batch::{decode_batch, RawTable};
use epiora_mdb::Mdb;
use serde_json::{json, Value as Json};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------- fixture + raw-table loading ----------

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates dir")
        .parent()
        .expect("repo root")
        .to_path_buf()
}

#[derive(serde::Deserialize)]
struct Fixture {
    steps: Vec<Step>,
}
#[derive(serde::Deserialize)]
struct Step {
    #[serde(default)]
    line: u32,
    command: String,
    #[serde(default)]
    rows: Option<Vec<Vec<Json>>>,
    #[serde(default)]
    results: BTreeMap<String, ResultPin>,
}
#[derive(serde::Deserialize)]
struct ResultPin {
    value: f64,
    decimals: i32,
}

/// EI7's raw-read column typing: Access byte/int → Integer, float →
/// Real, text/memo/guid → Text, datetime serial → DateTime seconds
/// since 1970-01-01, YesNo → 0/1 Integer (EI7's LIST shows 0/1 for
/// them, never True/False).
fn raw_table(name: &str, raw: &RawTable) -> Dataset {
    let mut cols = Vec::new();
    for c in &raw.columns {
        let data = match &c.data {
            epiora_legacy::batch::RawData::Int(v) => ColumnData::Integer(v.clone()),
            epiora_legacy::batch::RawData::Float(v) => ColumnData::Real(v.clone()),
            epiora_legacy::batch::RawData::Text(v) => ColumnData::Text(v.clone()),
            epiora_legacy::batch::RawData::Bool(v) => {
                ColumnData::Integer(v.iter().map(|x| x.map(i64::from)).collect())
            }
            epiora_legacy::batch::RawData::DateTime(v) => ColumnData::DateTime(
                v.iter()
                    .map(|x| x.map(|s| ((s - 25569.0) * 86400.0).round() as i64))
                    .collect(),
            ),
            epiora_legacy::batch::RawData::BinarySkipped => continue,
        };
        cols.push(Column {
            name: c.name.clone(),
            data,
        });
    }
    Dataset::new(name, 1, cols).expect("raw table as dataset")
}

fn load_table(mdbs: &BTreeMap<String, Mdb>, file: &str, table: &str) -> Dataset {
    let mdb = mdbs
        .get(&file.to_ascii_lowercase())
        .unwrap_or_else(|| panic!("mdb {file}"));
    let bytes = mdb.batch(table, 0, u32::MAX).expect("batch");
    raw_table(table, &decode_batch(&bytes).expect("decode"))
}

// ---------- the session: current view + loaded tables ----------

struct MapLinks<'a>(&'a BTreeMap<String, Dataset>);

impl epiora_lang::LinkedDatasets for MapLinks<'_> {
    fn get(&self, name: &str) -> Option<&Dataset> {
        self.0
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v)
    }
    fn link(&self, _p: &str, _c: &str) -> Option<&epiora_core::dataset::DatasetLink> {
        None
    }
}

fn as_of() -> AsOf {
    AsOf {
        date: 20_000,
        seconds: 0,
    }
}

fn do_replay(
    source: &Dataset,
    tables: &BTreeMap<String, Dataset>,
    steps: &[Command],
) -> WorkingView {
    replay(
        source,
        &MapLinks(tables),
        &Recipe {
            as_of: as_of(),
            steps: steps.to_vec(),
        },
    )
    .expect("replay")
}

struct Session<'m> {
    mdbs: &'m BTreeMap<String, Mdb>,
    /// lower-case name → current table contents (MERGE writes its
    /// destination back, as EI7's merge writes the file).
    tables: BTreeMap<String, Dataset>,
    /// The READ table backing the view.
    source: Dataset,
    /// Cleaning commands since the last READ.
    steps: Vec<Command>,
    view: WorkingView,
}

impl<'m> Session<'m> {
    fn open(mdbs: &'m BTreeMap<String, Mdb>, file: &str, table: &str) -> Session<'m> {
        let mut tables = BTreeMap::new();
        let source = load_table(mdbs, file, table);
        tables.insert(table.to_ascii_lowercase(), source.clone());
        let view = do_replay(&source, &tables, &[]);
        Session {
            mdbs,
            tables,
            source,
            steps: vec![],
            view,
        }
    }

    /// Ensures `table` (from `file`) is loaded; MERGE destinations are
    /// already there.
    fn table(&mut self, file: &str, name: &str) {
        let key = name.to_ascii_lowercase();
        if !self.tables.contains_key(&key) {
            let ds = load_table(self.mdbs, file, name);
            self.tables.insert(key, ds);
        }
    }

    fn read(&mut self, file: &str, table: &str) {
        self.table(file, table);
        self.source = self.tables[&table.to_ascii_lowercase()].clone();
        self.steps.clear();
        self.view = do_replay(&self.source, &self.tables, &[]);
    }

    fn command(&mut self, cmd: Command) {
        self.steps.push(cmd);
        self.view = do_replay(&self.source, &self.tables, &self.steps);
    }

    fn count(&self) -> usize {
        self.view.materialize().row_count()
    }
}

// ---------- command translation (the corpus's .pgm subset) ----------

fn expr(t: &str) -> Expr {
    parse_expr(t).unwrap_or_else(|e| panic!("expr {t:?}: {e}"))
}

/// `'<path>':<table>` → (file stem, table).
fn read_target(t: &str) -> (String, String) {
    let (path, table) = t.rsplit_once(':').expect("file:table");
    let stem = Path::new(path.trim_matches('\''))
        .file_stem()
        .expect("stem")
        .to_string_lossy()
        .to_string();
    (stem, table.trim().to_string())
}

fn lit_value(t: &str) -> epiora_core::dataset::Value {
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

fn recode_bound(t: &str) -> RecodeBound {
    match t.trim().to_ascii_uppercase().as_str() {
        "LOVALUE" => RecodeBound::LoValue,
        "HIVALUE" => RecodeBound::HiValue,
        v => RecodeBound::Value(lit_value(v)),
    }
}

fn recode_rule(line: &str) -> RecodeRule {
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

fn set_option(clause: &str) -> SetOption {
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

/// Runs one translated line; returns the number of steps consumed
/// (block commands consume their body lines too).
fn run_line(sess: &mut Session, steps: &[Step], i: usize) -> usize {
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

// ---------- output comparison ----------

fn ctx() -> RunContext {
    RunContext {
        timestamp: "2026-11-23T00:00:00Z".into(),
        project: None,
        working_view: None,
    }
}

/// Resolves `freq.categories[0].ci_95_clopper_pearson.lower`-style
/// paths against the produced JSON.
fn resolve<'a>(v: &'a Json, path: &str) -> Option<&'a Json> {
    let mut cur = v;
    for seg in path.split('.') {
        if let Some((name, idx)) = seg.split_once('[') {
            cur = cur.get(name)?;
            let i: usize = idx.trim_end_matches(']').parse().ok()?;
            cur = cur.get(i)?;
        } else {
            cur = cur.get(seg)?;
        }
    }
    Some(cur)
}

fn check_results(produced: &Json, step: &Step, what: &str) {
    for (path, pin) in &step.results {
        let actual = resolve(produced, path)
            .and_then(|v| v.as_f64())
            .unwrap_or_else(|| panic!("{what} L{}: no numeric at {path}", step.line));
        let tol = 0.5 * 10f64.powi(-pin.decimals);
        assert!(
            (actual - pin.value).abs() <= tol,
            "{what} L{} {path}: expected {} got {actual} (tol {tol})",
            step.line,
            pin.value
        );
    }
}

/// The `selection.*` (and `delete.*`/`undelete.*`) JSON a step's
/// results pin.
fn selection_json(sess: &Session, before: usize, delete_family: bool) -> Json {
    let mut root = serde_json::Map::new();
    root.insert(
        "selection".into(),
        json!({
            "record_count": sess.count() as u64,
            "deleted_records_included": i64::from(sess.view.deleted_records_included()),
        }),
    );
    if delete_family {
        let after = sess.count();
        root.insert(
            "delete".into(),
            json!({"records_deleted": before.saturating_sub(after)}),
        );
        root.insert(
            "undelete".into(),
            json!({"records_restored": after.saturating_sub(before)}),
        );
    }
    Json::Object(root)
}

fn compare_selection(sess: &Session, step: &Step) {
    check_results(&selection_json(sess, 0, false), step, "selection");
}

fn compare_delete(sess: &Session, step: &Step, before: usize) {
    check_results(&selection_json(sess, before, true), step, "delete");
}

fn compare_undelete(sess: &Session, step: &Step, before: usize) {
    check_results(&selection_json(sess, before, true), step, "undelete");
}

/// The MERGE banner counts: NUMBERINSERTED = source rows with no
/// destination match, NUMBERUPDATED = source rows that matched at
/// least one destination row (EI7 counts source rows applied).
fn compare_merge(
    sess: &Session,
    before_src: &Dataset,
    table: &str,
    keys: &[(String, String)],
    step: &Step,
) {
    let src = sess
        .tables
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(table))
        .map(|(_, v)| v)
        .expect("merge source");
    let mut inserted = 0u64;
    let mut updated = 0u64;
    for r in 0..src.row_count() {
        let hit = (0..before_src.row_count()).any(|d| {
            keys.iter()
                .all(|(dk, sk)| cell_text(before_src, dk, d) == cell_text(src, sk, r))
        });
        if hit {
            updated += 1;
        } else {
            inserted += 1;
        }
    }
    if let Some(rows) = &step.rows {
        let numbers: BTreeMap<String, Json> = rows
            .iter()
            .skip(1)
            .filter_map(|row| match (row.first(), row.get(1)) {
                (Some(Json::String(k)), Some(v)) => Some((k.clone(), v.clone())),
                _ => None,
            })
            .collect();
        if let Some(v) = numbers.get("NUMBERINSERTED") {
            assert_eq!(v.as_u64(), Some(inserted), "L{} inserted", step.line);
        }
        if let Some(v) = numbers.get("NUMBERUPDATED") {
            assert_eq!(v.as_u64(), Some(updated), "L{} updated", step.line);
        }
    }
    check_results(&selection_json(sess, 0, false), step, "merge");
}

/// A cell as the text EI7's join compares.
fn cell_text(ds: &Dataset, col: &str, row: usize) -> String {
    let c = ds.column(col).expect("key column");
    match &c.data {
        ColumnData::Integer(v) => v[row].map(|x| x.to_string()).unwrap_or_default(),
        ColumnData::Real(v) => v[row]
            .map(|x| {
                if x.fract() == 0.0 && x.abs() < 1e15 {
                    format!("{}", x as i64)
                } else {
                    format!("{x}")
                }
            })
            .unwrap_or_default(),
        ColumnData::Text(v) => v[row].clone().unwrap_or_default(),
        ColumnData::Boolean(v) => v[row].map(|b| b.to_string()).unwrap_or_default(),
        ColumnData::Date(v) => v[row].map(|x| x.to_string()).unwrap_or_default(),
        ColumnData::Time(v) => v[row].map(|x| x.to_string()).unwrap_or_default(),
        ColumnData::DateTime(v) => v[row].map(|x| x.to_string()).unwrap_or_default(),
    }
}

fn compare_freq(sess: &Session, var: &str, step: &Step) {
    let m = sess.view.materialize();
    let out = freq(
        &m,
        &FreqOptions {
            variables: vec![var.to_string()],
            stratify_by: None,
            include_missing: sess.view.settings.include_missing,
            ci_method: Default::default(),
        },
        &ctx(),
    )
    .expect("freq");
    let mut table = serde_json::to_value(&out.result.tables[0]).expect("freq json");
    // EI7 orders the missing category first; the engine appends
    // `"(missing)"` last (spec §4.1). Align for the positional
    // `categories[i]` keys.
    if let (Some(labels), Some(cats)) = (
        table["labels"].as_array().cloned(),
        table["categories"].as_array().cloned(),
    ) {
        if let Some(mi) = labels.iter().position(|l| l == "(missing)") {
            let mut l2 = labels;
            let mut c2 = cats;
            let lm = l2.remove(mi);
            let cm = c2.remove(mi);
            l2.insert(0, lm);
            c2.insert(0, cm);
            // Recompute cumulative percents in EI7's missing-first
            // order (the engine's stored values follow its own order).
            let mut acc = 0.0f64;
            for c in &mut c2 {
                acc += c["percent"].as_f64().unwrap_or(0.0);
                c["cumulative_percent"] = Json::from(acc);
            }
            table["labels"] = Json::Array(l2);
            table["categories"] = Json::Array(c2);
        }
    }
    // The fixture's `freq.total` is the displayed Total row — the
    // category sum; the engine's `total` counts all rows in scope.
    let sum: u64 = table["categories"]
        .as_array()
        .map(|cs| cs.iter().filter_map(|c| c["count"].as_u64()).sum())
        .unwrap_or(0);
    table["total"] = Json::from(sum);
    check_results(&json!({"freq": table}), step, "freq");
}

fn compare_means(sess: &Session, var: &str, step: &Step) {
    let m = sess.view.materialize();
    let out = means(
        &m,
        &MeansOptions {
            variable: var.to_string(),
            group_by: None,
            include_missing: false,
        },
        &ctx(),
    )
    .expect("means");
    let j = serde_json::to_value(&out.result).expect("means json");
    check_results(&json!({"means": j}), step, "means");
}

fn compare_list(sess: &Session, vars: &[String], step: &Step) {
    let m = sess.view.materialize();
    let mut rows: Vec<Vec<Json>> = vec![vars
        .iter()
        .map(|v| {
            Json::String(
                m.column(v)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|_| v.clone()),
            )
        })
        .collect()];
    for r in 0..m.row_count() {
        rows.push(vars.iter().map(|v| list_cell(&m, v, r)).collect());
    }
    let expected = step.rows.clone().expect("list step has rows");
    assert_eq!(rows, expected, "LIST L{} rows", step.line);
    check_results(&selection_json(sess, 0, false), step, "list");
}

/// One LIST cell as the fixture's `cellValue` reads it: `''`/`Missing`
/// → null, numeric spellings → numbers, text verbatim, EI7's date/
/// time spellings.
fn list_cell(ds: &Dataset, var: &str, row: usize) -> Json {
    let c = ds.column(var).unwrap_or_else(|_| panic!("list var {var}"));
    fn real(x: f64) -> Json {
        let s = format!("{x:.10}");
        let s = s.trim_end_matches('0').trim_end_matches('.');
        if s.contains('.') {
            Json::from(s.parse::<f64>().expect("real"))
        } else {
            Json::from(s.parse::<i64>().expect("int"))
        }
    }
    fn civil(z: i64) -> (i64, u32, u32) {
        let z = z + 719_468;
        let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let y = yoe + era * 400;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = doy - (153 * mp + 2) / 5 + 1;
        let m = if mp < 10 { mp + 3 } else { mp - 9 };
        (y + i64::from(m <= 2), m as u32, d as u32)
    }
    fn dt(secs: i64) -> String {
        let (y, mo, d) = civil(secs.div_euclid(86400));
        let t = secs.rem_euclid(86400);
        let (h, mi, s) = (t / 3600, t % 3600 / 60, t % 60);
        let (h12, ap) = match h {
            0 => (12, "AM"),
            12 => (12, "PM"),
            h if h < 12 => (h, "AM"),
            h => (h - 12, "PM"),
        };
        format!("{mo}/{d}/{y} {h12}:{mi:02}:{s:02} {ap}")
    }
    // `cellValue` on a text cell.
    fn text(s: &str) -> Json {
        if s.is_empty() || s == "Missing" {
            return Json::Null;
        }
        let stripped: String = s.chars().filter(|c| *c != ',').collect();
        if !s.contains('%')
            && !s.contains('(')
            && stripped
                .parse::<f64>()
                .ok()
                .filter(|x| x.is_finite())
                .is_some()
        {
            return stripped.parse::<f64>().map_or(Json::Null, |x| {
                if s.contains('.') {
                    Json::from(x)
                } else {
                    Json::from(x as i64)
                }
            });
        }
        Json::String(s.to_string())
    }
    match &c.data {
        ColumnData::Integer(v) => v[row].map_or(Json::Null, Json::from),
        ColumnData::Real(v) => v[row].map_or(Json::Null, real),
        ColumnData::Text(v) => v[row].as_ref().map_or(Json::Null, |s| text(s)),
        ColumnData::Boolean(v) => v[row].map_or(Json::Null, |b| {
            Json::String(if b { "True".into() } else { "False".into() })
        }),
        ColumnData::Date(v) => v[row].map_or(Json::Null, |d| {
            let (y, mo, dd) = civil(i64::from(d));
            Json::String(format!("{mo}/{dd}/{y} 12:00:00 AM"))
        }),
        ColumnData::Time(v) => v[row].map_or(Json::Null, |t| {
            Json::String(format!(
                "{:02}:{:02}:{:02}",
                t / 3600,
                t % 3600 / 60,
                t % 60
            ))
        }),
        ColumnData::DateTime(v) => v[row].map_or(Json::Null, |s| Json::String(dt(s))),
    }
}

// ---------- the test ----------

#[test]
fn every_reference_program_step_matches() {
    let dir = root().join("fixtures/ei7/v11");
    let mut mdbs = BTreeMap::new();
    for name in ["Sample", "Nutrition"] {
        let p = root().join(format!("fixtures/legacy/samples/ei7/{name}/{name}.mdb"));
        mdbs.insert(
            name.to_ascii_lowercase(),
            Mdb::open(&p).unwrap_or_else(|e| panic!("open {p:?}: {e}")),
        );
    }
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("v11 fixture dir")
        .filter_map(|e| {
            let p = e.expect("entry").path();
            (p.extension().is_some_and(|x| x == "json")).then_some(p)
        })
        .collect();
    files.sort();
    assert!(files.len() >= 7, "expected the Task 3 programs in {dir:?}");
    for path in files {
        let fixture: Fixture = serde_json::from_str(
            &std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {path:?}")),
        )
        .unwrap_or_else(|_| panic!("parse {path:?}"));
        let mut sess: Option<Session> = None;
        let mut i = 0;
        while i < fixture.steps.len() {
            let step = &fixture.steps[i];
            if step
                .command
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("READ ")
            {
                let (file, table) = read_target(step.command[4..].trim());
                match &mut sess {
                    Some(s) => s.read(&file, &table),
                    None => sess = Some(Session::open(&mdbs, &file, &table)),
                }
                check_results(
                    &selection_json(sess.as_ref().expect("session"), 0, false),
                    step,
                    "read",
                );
                i += 1;
                continue;
            }
            let s = sess
                .as_mut()
                .unwrap_or_else(|| panic!("no READ before L{}", step.line));
            i += run_line(s, &fixture.steps, i);
        }
    }
}
