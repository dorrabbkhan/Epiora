//! Output comparison: `results` pins at the `ei7_display` tolerance
//! and LIST rows exactly.

use super::fixture::Step;
use super::session::Session;
use epiora_core::dataset::{ColumnData, Dataset};
use epiora_core::procedures::{freq, means, FreqOptions, MeansOptions, RunContext};
use serde_json::{json, Value as Json};
use std::collections::BTreeMap;

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

#[allow(clippy::disallowed_methods)]
pub fn check_results(produced: &Json, step: &Step, what: &str) {
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
pub fn selection_json(sess: &Session, before: usize, delete_family: bool) -> Json {
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

pub fn compare_selection(sess: &Session, step: &Step) {
    check_results(&selection_json(sess, 0, false), step, "selection");
}

pub fn compare_delete(sess: &Session, step: &Step, before: usize) {
    check_results(&selection_json(sess, before, true), step, "delete");
}

pub fn compare_undelete(sess: &Session, step: &Step, before: usize) {
    check_results(&selection_json(sess, before, true), step, "undelete");
}

/// The MERGE banner counts: NUMBERINSERTED = source rows with no
/// destination match, NUMBERUPDATED = source rows that matched at
/// least one destination row (EI7 counts source rows applied).
pub fn compare_merge(
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

pub fn compare_freq(sess: &Session, var: &str, step: &Step) {
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

pub fn compare_means(sess: &Session, var: &str, step: &Step) {
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

pub fn compare_list(sess: &Session, vars: &[String], step: &Step) {
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

/// One LIST cell as `fixtures/ei7/steps.mjs::cellValue` reads it:
/// `''`/`Missing` → null, numeric spellings → numbers in the JS
/// `Number`/`JSON.stringify` shape (integral values integer JSON,
/// otherwise float), text verbatim, EI7's date/time spellings.
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
                if x.fract() == 0.0 && x >= i64::MIN as f64 && x < i64::MAX as f64 {
                    Json::from(x as i64)
                } else {
                    Json::from(x)
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
